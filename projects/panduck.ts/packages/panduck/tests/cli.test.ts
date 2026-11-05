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

test("plan accepts the markdown to epub pipeline", () => {
    const plan = runPanduck(["plan", "package.json", "--from", "markdown", "--to", "epub", "--json"]);
    assert.equal(plan.code, 0, plan.stderr);
    assert.match(plan.stdout, /panduck\.report\/v0/);
    assert.match(plan.stdout, /"writer": "epub"/);
});

test("plan blocks unsupported ready pipeline with report schema", () => {
    const plan = runPanduck(["plan", "package.json", "--from", "markdown", "--to", "typst", "--json"]);
    assert.equal(plan.code, 3);
    assert.match(plan.stdout, /panduck\.report\/v0/);
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

test("cli reports its installed version without loading the native lane", () => {
    const run = runPanduck(["--version"]);
    assert.equal(run.code, 0, run.stderr);
    assert.match(run.stdout.trim(), /^\d+\.\d+\.\d+$/, "the version flag must print a semver");
});

test("convert detection does not claim a zip container for markdown", () => {
    const workdir = mkdtempSync(join(tmpdir(), "panduck-detection-"));
    const input = join(workdir, "source.md");
    writeFileSync(input, "# Title\n\nBody.\n", "utf8");
    const run = runPanduck([
        "convert",
        input,
        "--from",
        "markdown",
        "--to",
        "html",
        "-o",
        join(workdir, "source.html"),
        "--diagnostics",
        "json",
    ]);
    assert.equal(run.code, 0, run.stderr);
    const report = JSON.parse(run.stdout) as { detection: { outer?: string; inner?: string; format?: string } };
    assert.equal(report.detection.format, "markdown");
    assert.equal(report.detection.outer, undefined, "markdown is not a zip container");
    assert.equal(report.detection.inner, undefined);
    rmSync(workdir, { recursive: true, force: true });
});

test("convert infers typst from the .typ extension without --from", () => {
    const workdir = mkdtempSync(join(tmpdir(), "panduck-typst-detection-"));
    const input = join(workdir, "source.typ");
    writeFileSync(input, "#let x = 1\nHello\n", "utf8");
    const run = runPanduck([
        "convert",
        input,
        "--to",
        "html",
        "-o",
        join(workdir, "source.html"),
        "--diagnostics",
        "json",
    ]);
    assert.equal(run.code, 0, run.stderr);
    const report = JSON.parse(run.stdout) as { detection: { format?: string } };
    assert.equal(report.detection.format, "typst");
    rmSync(workdir, { recursive: true, force: true });
});
