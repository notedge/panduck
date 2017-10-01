import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
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

test("global --log-file installs native console sink without breaking commands", () => {
    const workdir = mkdtempSync(join(tmpdir(), "panduck-log-"));
    const logPath = join(workdir, "events.jsonl");
    const formats = runPanduck(["--log-file", logPath, "formats", "markdown", "--json"]);
    assert.equal(formats.code, 0, formats.stderr);
    assert.match(formats.stdout, /"format": "markdown"/);
    rmSync(workdir, { recursive: true, force: true });
});

test("convert --log-file records semantic loss diagnostics on successful routes", () => {
    const workdir = mkdtempSync(join(tmpdir(), "panduck-log-"));
    const logPath = join(workdir, "loss.jsonl");
    const inputPath = join(workdir, "sample.nd");
    const outputPath = join(workdir, "out.md");
    writeFileSync(inputPath, "# Title\n\nBody.\n", "utf8");
    const convert = runPanduck([
        "convert",
        inputPath,
        "--from",
        "notedown",
        "--to",
        "markdown",
        "-o",
        outputPath,
        "--log-file",
        logPath,
    ]);
    assert.equal(convert.code, 0, convert.stderr);
    const text = readFileSync(logPath, "utf8");
    assert.match(text, /"kind":"diagnostic"/);
    assert.match(text, /partial_coverage/);
    rmSync(workdir, { recursive: true, force: true });
});
