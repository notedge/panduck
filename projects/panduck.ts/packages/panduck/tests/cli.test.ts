import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { runPanduck } from "./helpers/cli.js";

const fixtures = join(dirname(fileURLToPath(import.meta.url)), "../../panduck-fixtures/fixtures/formats");

test("formats emits markdown capability json", () => {
    const formats = runPanduck(["formats", "markdown", "--json"]);
    assert.equal(formats.code, 0, formats.stderr);
    assert.match(formats.stdout, /"format": "markdown"/);
    assert.match(formats.stdout, /"conversions":/);
});

test("formats lists only conversions whose source and target can do their direction", () => {
    const formats = runPanduck(["formats", "--json"]);
    assert.equal(formats.code, 0, formats.stderr);
    const payload = JSON.parse(formats.stdout) as {
        formats: Array<{ format: string; writer: { state: string } }>;
        conversions: string[];
    };

    assert.equal(payload.conversions.includes("xlsx:html"), false, "a render-only workbook must not advertise export");
    assert.equal(payload.conversions.includes("pptx:markdown"), false, "a render-only deck must not advertise export");
    assert.ok(payload.conversions.includes("doc:html"), "the legacy doc reader is a listed source");
    assert.ok(payload.conversions.includes("typst:html"), "typst keeps its registered HTML target");
    assert.ok(payload.conversions.includes("markdown:pandoc-json"), "pandoc-json is a native writer");
    assert.ok(payload.conversions.includes("pandoc-json:markdown"), "pandoc-json is a native reader");
    assert.equal(
        payload.formats.some((record) => record.format === "pandoc-json"),
        true,
        "a natively routed format joins the registry without a hand-written record",
    );

    assert.equal(
        payload.formats.some((record) => record.format === "xlsx"),
        false,
        "render-only office formats stay outside the conversion registry",
    );
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

test("plan reports partial coverage for a listed partial route", () => {
    const plan = runPanduck(["plan", "package.json", "--from", "markdown", "--to", "html", "--json"]);
    assert.equal(plan.code, 0, plan.stderr);
    const report = JSON.parse(plan.stdout) as {
        status: string;
        coverage: { state: string; unknown: string[] };
    };
    assert.equal(report.status, "partial", "a partial reader must not be reported as success");
    assert.equal(report.coverage.state, "partial");
    assert.deepEqual(report.coverage.unknown, ["script", "style"]);
});

test("plan refuses a route the capability registry does not list", () => {
    const plan = runPanduck(["plan", "package.json", "--from", "xlsx", "--to", "markdown", "--json"]);
    assert.equal(plan.code, 3, plan.stderr);
    const report = JSON.parse(plan.stdout) as {
        status: string;
        diagnostics: Array<{ code: string }>;
    };
    assert.equal(report.status, "blocked");
    assert.equal(report.diagnostics[0].code, "panduck.route.unlisted");
});

test("convert refuses a render-only workbook before the native lane", () => {
    const workbook = join(fixtures, "xlsx", "multi-sheet.xlsx");
    const convert = runPanduck(["convert", workbook, "--from", "xlsx", "--to", "markdown", "--report", "-", "--diagnostics", "silent"]);
    assert.equal(convert.code, 3, convert.stderr);
    const report = JSON.parse(convert.stdout) as {
        status: string;
        diagnostics: Array<{ code: string }>;
    };
    assert.equal(report.status, "blocked");
    assert.equal(report.diagnostics[0].code, "panduck.route.unlisted");
});

test("org stays unavailable instead of advertising a reader route", () => {
    const formats = runPanduck(["formats", "--json"]);
    assert.equal(formats.code, 0, formats.stderr);
    const payload = JSON.parse(formats.stdout) as {
        formats: Array<{ format: string; reader: { state: string }; writer: { state: string } }>;
        conversions: string[];
    };
    const org = payload.formats.find((record) => record.format === "org");
    assert.ok(org, "org stays a known format id");
    assert.equal(org.reader.state, "unavailable");
    assert.equal(org.writer.state, "unavailable");
    assert.equal(
        payload.conversions.some((route) => route.startsWith("org:") || route.endsWith(":org")),
        false,
        "an unavailable org reader must not advertise any conversion",
    );

    const workdir = mkdtempSync(join(tmpdir(), "panduck-org-"));
    try {
        const source = join(workdir, "basic.org");
        writeFileSync(source, "* Heading\nBody\n");
        const convert = runPanduck([
            "convert",
            source,
            "--from",
            "org",
            "--to",
            "markdown",
            "--report",
            "-",
            "--diagnostics",
            "silent",
        ]);
        assert.equal(convert.code, 3, convert.stderr);
        const report = JSON.parse(convert.stdout) as {
            status: string;
            diagnostics: Array<{ code: string }>;
        };
        assert.equal(report.status, "blocked");
        assert.equal(report.diagnostics[0].code, "panduck.route.unlisted");
    } finally {
        rmSync(workdir, { recursive: true, force: true });
    }
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

test("convert serves pandoc-json as a writer and as a reader", () => {
    const workdir = mkdtempSync(join(tmpdir(), "panduck-pandoc-route-"));
    try {
        const source = join(workdir, "route.md");
        const artifact = join(workdir, "route.json");
        const back = join(workdir, "route-back.md");
        writeFileSync(source, "# Route\n\nBody with **bold**.\n", "utf8");

        const forward = runPanduck([
            "convert",
            source,
            "--from",
            "markdown",
            "--to",
            "pandoc-json",
            "-o",
            artifact,
            "--report",
            "-",
            "--diagnostics",
            "silent",
        ]);
        assert.equal(forward.code, 0, forward.stderr);
        const forwardReport = JSON.parse(forward.stdout) as {
            status: string;
            pipeline: { reader: string; writer: string };
            outputs: Array<{ published: boolean }>;
        };
        assert.equal(forwardReport.status, "success");
        assert.equal(forwardReport.pipeline.reader, "markdown");
        assert.equal(forwardReport.pipeline.writer, "pandoc-json");
        assert.equal(forwardReport.outputs[0]?.published, true);

        const ast = JSON.parse(readFileSync(artifact, "utf8")) as {
            "pandoc-api-version": unknown;
            blocks: unknown[];
        };
        assert.deepEqual(ast["pandoc-api-version"], [1, 23, 1]);
        assert.ok(ast.blocks.length > 0, "the pandoc writer emits blocks");

        const reverse = runPanduck([
            "convert",
            artifact,
            "--from",
            "pandoc-json",
            "--to",
            "markdown",
            "-o",
            back,
            "--report",
            "-",
            "--diagnostics",
            "silent",
        ]);
        assert.equal(reverse.code, 0, reverse.stderr);
        const reverseReport = JSON.parse(reverse.stdout) as {
            status: string;
            pipeline: { reader: string; writer: string };
        };
        assert.equal(reverseReport.status, "success");
        assert.equal(reverseReport.pipeline.reader, "pandoc-json");
        assert.equal(reverseReport.pipeline.writer, "markdown");
        assert.match(readFileSync(back, "utf8"), /Body with \*\*bold\*\*/);
    } finally {
        rmSync(workdir, { recursive: true, force: true });
    }
});

test("convert keeps blocking a pandoc-json route the native table rejects", () => {
    const workdir = mkdtempSync(join(tmpdir(), "panduck-pandoc-blocked-"));
    try {
        const source = join(workdir, "route.json");
        writeFileSync(
            source,
            JSON.stringify({
                "pandoc-api-version": [1, 23, 1],
                meta: {},
                blocks: [{ t: "Para", c: [{ t: "Str", c: "Route" }] }],
            }),
            "utf8",
        );
        const convert = runPanduck([
            "convert",
            source,
            "--from",
            "pandoc-json",
            "--to",
            "typst",
            "--report",
            "-",
            "--diagnostics",
            "silent",
        ]);
        assert.equal(convert.code, 3, convert.stderr);
        const report = JSON.parse(convert.stdout) as {
            status: string;
            diagnostics: Array<{ code: string }>;
        };
        assert.equal(report.status, "blocked");
        assert.equal(report.diagnostics[0].code, "panduck.route.unlisted");
    } finally {
        rmSync(workdir, { recursive: true, force: true });
    }
});

test("convert names an unknown target format instead of guessing a route", () => {
    const workdir = mkdtempSync(join(tmpdir(), "panduck-unknown-target-"));
    try {
        const source = join(workdir, "route.md");
        writeFileSync(source, "# Route\n", "utf8");
        const convert = runPanduck([
            "convert",
            source,
            "--from",
            "markdown",
            "--to",
            "nonsense",
            "--report",
            "-",
            "--diagnostics",
            "silent",
        ]);
        assert.equal(convert.code, 3, convert.stderr);
        assert.match(convert.stdout, /unknown target format nonsense/);
    } finally {
        rmSync(workdir, { recursive: true, force: true });
    }
});
