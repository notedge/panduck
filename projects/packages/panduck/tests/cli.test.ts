import assert from "node:assert/strict";
import test from "node:test";

import { runPanduck } from "./helpers/cli.js";

test("formats emits markdown capability json", () => {
    const formats = runPanduck(["formats", "markdown", "--json"]);
    assert.equal(formats.code, 0, formats.stderr);
    assert.match(formats.stdout, /"format": "markdown"/);
    assert.match(formats.stdout, /"conversions":/);
});

test("plan blocks unsupported ready pipeline with report schema", () => {
    const plan = runPanduck(["plan", "package.json", "--from", "markdown", "--to", "docx", "--json"]);
    assert.equal(plan.code, 3);
    assert.match(plan.stdout, /panduck\.report\/v1/);
});
