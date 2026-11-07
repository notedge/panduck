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

test("markdown footnotes keep their precise pandoc-json reasons and survive a real roundtrip", () => {
    const workdir = mkdtempSync(join(tmpdir(), "panduck-pandoc-note-"));
    try {
        type PandocBlock = { t: string; c?: unknown };
        type Report = { status: string; losses: Array<{ code: string }> };

        const cases: Array<{ name: string; markdown: string; status: string; codes: string[] }> = [
            {
                name: "safe",
                markdown: "Text with a note.[^a]\n\n[^a]: body with *emphasis* and `code`\n",
                status: "success",
                codes: [],
            },
            {
                name: "missing",
                markdown: "Text with a note.[^a]\n",
                status: "success_with_loss",
                codes: ["export.pandoc-json.note_definition_missing"],
            },
            {
                name: "empty",
                markdown: "Text with a note.[^a]\n\n[^a]:\n",
                status: "success_with_loss",
                codes: ["export.pandoc-json.note_definition_empty", "export.pandoc-json.opaque_raw"],
            },
            {
                name: "multiblock",
                markdown: "Text with a note.[^a]\n\n[^a]: first line\n    second line\n",
                status: "success_with_loss",
                codes: ["export.pandoc-json.note_definition_multiblock", "export.pandoc-json.opaque_raw"],
            },
            {
                name: "nested",
                markdown: "Text with a note.[^a]\n\n[^a]: body with [^b]\n",
                status: "success_with_loss",
                codes: ["export.pandoc-json.note_definition_nested", "export.pandoc-json.opaque_raw"],
            },
            {
                name: "block-level",
                markdown: "Text with a note.[^a]\n\n[^a]: - list item\n",
                status: "success_with_loss",
                codes: ["export.pandoc-json.note_definition_block_level", "export.pandoc-json.opaque_raw"],
            },
            {
                // The body image keeps its source URL, so the definition is
                // rebuilt as a native Note instead of staying opaque.
                name: "image",
                markdown: "Text with a note.[^a]\n\n[^a]: see ![alt](pic.png)\n",
                status: "success",
                codes: [],
            },
            {
                // A body whose reference cannot be resolved has no inline
                // shape, so it still stays opaque with a precise reason.
                name: "lossy",
                markdown: "Text with a note.[^a]\n\n[^a]: [text][missing]\n",
                status: "success_with_loss",
                codes: ["export.pandoc-json.note_definition_unrepresentable", "export.pandoc-json.opaque_raw"],
            },
        ];

        for (const entry of cases) {
            const source = join(workdir, `${entry.name}.md`);
            const artifact = join(workdir, `${entry.name}.json`);
            writeFileSync(source, entry.markdown, "utf8");
            const run = runPanduck([
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
            assert.equal(run.code, 0, `${entry.name}: ${run.stderr}`);
            const report = JSON.parse(run.stdout) as Report;
            assert.equal(report.status, entry.status, entry.name);
            const codes = [...report.losses.map((loss) => loss.code)].sort();
            assert.deepEqual(codes, [...entry.codes].sort(), `${entry.name} loss codes`);

            const ast = JSON.parse(readFileSync(artifact, "utf8")) as { blocks: PandocBlock[] };
            const notes = ast.blocks.flatMap((block) =>
                block.t === "Para" ? (block.c as PandocBlock[]).filter((inline) => inline.t === "Note") : [],
            );
            assert.equal(notes.length, 1, `${entry.name} must keep one Note reference`);
            const raw = ast.blocks.filter(
                (block) =>
                    block.t === "RawBlock" &&
                    Array.isArray(block.c) &&
                    (block.c as string[])[0] === "notedown.footnote_definition",
            );
            const rebuilt = entry.name === "safe" || entry.name === "image";
            if (rebuilt) {
                assert.deepEqual(raw, [], "a rebuilt definition must not linger as a raw block");
            } else {
                assert.deepEqual(notes[0].c, [], "an opaque definition leaves an empty Note");
            }
            if (entry.name === "missing" || rebuilt) {
                assert.deepEqual(raw, [], "a rebuilt or missing definition keeps no raw block");
            } else {
                assert.equal(raw.length, 1, `${entry.name} keeps exactly one raw definition`);
            }
            if (entry.name === "safe") {
                assert.match(JSON.stringify(notes[0]), /body with /, "the rebuilt Note carries the body");
            }
            if (entry.name === "image") {
                assert.match(JSON.stringify(notes[0]), /pic\.png/, "the rebuilt Note keeps the body image");
            }
        }

        const artifact = join(workdir, "safe.json");
        const back = join(workdir, "safe-back.md");
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
        const reverseReport = JSON.parse(reverse.stdout) as Report;
        assert.equal(reverseReport.status, "success");
        assert.deepEqual(reverseReport.losses, []);
        const roundtripped = readFileSync(back, "utf8");
        assert.match(roundtripped, /\[\^pandoc-note-1\]/);
        assert.match(roundtripped, /\[\^pandoc-note-1\]: body with \*emphasis\* and `code`/);

        // The image body is a native Note in both directions, so a second
        // forward pass must rebuild it again instead of falling back to raw.
        const imageBack = join(workdir, "image-back.md");
        const imageForward = runPanduck([
            "convert",
            join(workdir, "image.json"),
            "--from",
            "pandoc-json",
            "--to",
            "markdown",
            "-o",
            imageBack,
            "--report",
            "-",
            "--diagnostics",
            "silent",
        ]);
        assert.equal(imageForward.code, 0, imageForward.stderr);
        const imageForwardReport = JSON.parse(imageForward.stdout) as Report;
        assert.deepEqual(
            imageForwardReport.losses.map((loss) => loss.code),
            ["import.pandoc-json.asset_unresolved"],
            "the body image keeps its URL and only the missing media bytes are unresolved",
        );
        assert.match(readFileSync(imageBack, "utf8"), /\[\^pandoc-note-1\]: see \!\[alt\]\(pic\.png\)/);
        const imageReopened = join(workdir, "image-reopened.json");
        const imageReturn = runPanduck([
            "convert",
            imageBack,
            "--from",
            "markdown",
            "--to",
            "pandoc-json",
            "-o",
            imageReopened,
            "--report",
            "-",
            "--diagnostics",
            "silent",
        ]);
        assert.equal(imageReturn.code, 0, imageReturn.stderr);
        const imageReturnReport = JSON.parse(imageReturn.stdout) as Report;
        assert.deepEqual(imageReturnReport.losses, [], "a rebuilt image note round-trips without a note loss");
        assert.match(readFileSync(imageReopened, "utf8"), /"t":"Note"/);
        assert.match(readFileSync(imageReopened, "utf8"), /pic\.png/);
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
