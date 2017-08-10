import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const panduckBin = join(dirname(fileURLToPath(import.meta.url)), "../../panduck/bin/panduck.mjs");

function run(args) {
    const result = spawnSync(process.execPath, [panduckBin, ...args], { encoding: "utf8" });
    return { code: result.status ?? 1, stdout: result.stdout, stderr: result.stderr };
}

const formats = run(["formats", "markdown", "--json"]);
assert.equal(formats.code, 0, `formats exit code: ${formats.stderr}`);
assert.match(formats.stdout, /"format": "markdown"/, "formats json payload");

const plan = run(["plan", "package.json", "--from", "markdown", "--to", "docx", "--json"]);
assert.equal(plan.code, 3, "plan should block unsupported ready pipeline");
assert.match(plan.stdout, /panduck\.report\/v1/, "plan report schema");

console.log("panduck-cli smoke ok");
