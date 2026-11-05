import assert from "node:assert/strict";
import { access, mkdtemp, readdir, rm, writeFile } from "node:fs/promises";
import { constants } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
    CONVERSION_DEADLINE_MS,
    EXPORT_BYTES_CEILING,
    deadlineStop,
    exitCodeForDiagnostics,
    exportCeilingStop,
    runConversionPipeline,
    runMarkdownProjectPipeline,
} from "../src/cli/pipeline.js";
import { ExitCode } from "../src/cli/exit-codes.js";
import { createDiagnostic, textArg } from "../src/diagnostic.js";
import type { SharedCliOptions } from "../src/cli/options.js";
import type { CliContext } from "../src/cli/context.js";
import type { PanduckBindings } from "../src/types.js";
import { runPanduck } from "./helpers/cli.js";

const shared: SharedCliOptions = {
    json: false,
    budgets: {},
    loss: "warn",
    strict: false,
    allowPartial: false,
    diagnostics: "silent",
    overwrite: true,
};

const failedDiagnosticsJson = JSON.stringify({
    schemaVersion: 1,
    diagnostics: [
        {
            code: "panduck.adapter.not-found",
            severity: "error",
            origin: { namespace: "panduck", component: "adapter", stage: "read" },
            message: {
                key: "panduck.adapter.not-found",
                args: [["format", { text: "docx" }]],
                fallback: "input document is not a readable docx package",
            },
            primary: null,
            secondary: [],
            notes: [],
            helps: [],
            cause: null,
            actions: [],
        },
    ],
    truncated: false,
    dropped: 0,
});

const failedImport = () => ({
    success: false,
    graphJson: null,
    coverageJson: "null",
    lossesJson: "[]",
    diagnosticsJson: failedDiagnosticsJson,
});

const noopTransform = (graphJson: string) => ({
    success: true,
    graphJson,
    passesJson: "[]",
    diagnosticsJson: JSON.stringify({ schemaVersion: 1, diagnostics: [], truncated: false, dropped: 0 }),
});

const noopExport = () => ({
    success: true,
    indexMarkdown: "",
    chapters: [],
    assets: [],
    unresolvedAssets: [],
    coverageJson: "null",
    lossesJson: "[]",
    diagnosticsJson: JSON.stringify({ schemaVersion: 1, diagnostics: [], truncated: false, dropped: 0 }),
});

function stubBindings(overrides: Partial<PanduckBindings>): { ctx: CliContext; bindings: PanduckBindings } {
    const bindings: PanduckBindings = {
        panduckVersion: () => "0.0.0-test",
        supportedFormats: () => ["docx", "markdown"],
        isSupportedFormat: () => true,
        supportsConversion: () => true,
        ...overrides,
    };
    return {
        bindings,
        ctx: { bindings, toolVersion: "0.0.0-test", registry: [] },
    };
}

async function exists(path: string): Promise<boolean> {
    try {
        await access(path, constants.F_OK);
        return true;
    } catch {
        return false;
    }
}

test("hard export ceiling and conversion deadline stop at their exact limits", () => {
    assert.equal(EXPORT_BYTES_CEILING, 134_217_728);
    assert.equal(CONVERSION_DEADLINE_MS, 300_000);
    assert.equal(exportCeilingStop(EXPORT_BYTES_CEILING), null);
    const ceiling = exportCeilingStop(EXPORT_BYTES_CEILING + 1);
    assert.equal(ceiling?.code, "panduck.export.size-exceeded");
    assert.equal(deadlineStop(CONVERSION_DEADLINE_MS), null);
    const deadline = deadlineStop(CONVERSION_DEADLINE_MS + 1);
    assert.equal(deadline?.code, "panduck.convert.deadline-exceeded");
});

test("writer unsupported diagnostics map to the target failure exit code", () => {
    const unsupported = createDiagnostic({
        code: "notedown.format.unsupported",
        severity: "error",
        component: "formats",
        args: [["format", textArg("docx")]],
    });
    assert.equal(exitCodeForDiagnostics([unsupported], "docx"), ExitCode.IrOrWriterFailure);
    assert.equal(exitCodeForDiagnostics([unsupported], "markdown"), ExitCode.InputUnsupported);
    assert.equal(exitCodeForDiagnostics([unsupported]), ExitCode.InputUnsupported);
});

test("native convert failure blocks the output file instead of writing empty text", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-pipeline-fail-"));
    const outputPath = join(workdir, "out.md");
    const inputPath = join(workdir, "broken.docx");
    await writeFile(inputPath, "not a zip container", "utf8");
    const { ctx, bindings } = stubBindings({
        importDocument: failedImport,
        transformDocument: noopTransform,
        exportDocument: noopExport,
    });

    try {
        const result = await runConversionPipeline({
            ctx,
            bindings,
            inputPath,
            outputPath,
            resolved: { from: "docx", to: "markdown", ambiguous: false, hints: [".docx"] },
            shared,
            toolVersion: "0.0.0-test",
        });

        assert.equal(result.exitCode, ExitCode.IrOrWriterFailure);
        assert.equal(result.report.status, "blocked");
        assert.equal(
            result.report.pipeline?.blocked_reason,
            "native conversion failed with exit code 1",
        );
        const diagnostic = result.report.diagnostics?.find(
            (entry) => entry.code === "panduck.adapter.not-found",
        );
        assert.ok(diagnostic, "native failure diagnostics must reach the CLI report");
        assert.deepEqual(diagnostic.origin, {
            namespace: "panduck",
            component: "adapter",
            stage: "read",
        });
        assert.equal(diagnostic.message.fallback, "input document is not a readable docx package");
        assert.deepEqual(diagnostic.message.args, [["format", { text: "docx" }]]);
        assert.equal(await exists(outputPath), false, "failed conversion must not publish an empty file");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("native convert failure writes nothing to stdout", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-native-failure-stdout-"));
    const input = join(workdir, "broken.docx");
    await writeFile(input, "this is not a zip container", "utf8");

    try {
        const run = runPanduck([
            "convert",
            input,
            "--from",
            "docx",
            "--to",
            "markdown",
            "-o",
            "-",
            "--diagnostics",
            "silent",
        ]);
        assert.equal(run.code, ExitCode.ParseFailure, `the corrupt container must fail: ${run.stderr}`);
        assert.equal(run.stdout, "", "failed conversion must not emit empty stdout");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("native markdown project failure blocks without claiming published outputs", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-project-fail-"));
    const outputDir = join(workdir, "site");
    const inputPath = join(workdir, "broken.docx");
    await writeFile(inputPath, "not a zip container", "utf8");
    const { ctx, bindings } = stubBindings({
        supportsMarkdownProject: () => true,
        importDocument: failedImport,
        transformDocument: noopTransform,
        exportMarkdownProject: noopExport,
    });

    try {
        const result = await runMarkdownProjectPipeline({
            ctx,
            bindings,
            inputPath,
            outputDir,
            resolved: { from: "docx", to: "markdown-project", ambiguous: false, hints: [".docx"] },
            shared,
            toolVersion: "0.0.0-test",
        });

        assert.equal(result.exitCode, ExitCode.IrOrWriterFailure);
        assert.equal(result.report.status, "blocked");
        assert.equal(result.report.outputs?.length, 0, "failed publish must not report outputs");
        assert.equal(await exists(outputDir), false);
        assert.deepEqual(await readdir(workdir), ["broken.docx"], "failed publish must leave only the input file");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});
