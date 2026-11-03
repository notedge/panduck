import assert from "node:assert/strict";
import { access, mkdtemp, readdir, rm, writeFile } from "node:fs/promises";
import { constants } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
    runConversionPipeline,
    runMarkdownProjectPipeline,
} from "../src/cli/pipeline.js";
import { ExitCode } from "../src/cli/exit-codes.js";
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

const failedReportJson = JSON.stringify({
    schema_version: "panduck.report/v0",
    operation: "convert",
    status: "failed",
    pipeline: { reader: "docx", writer: "markdown", stages: ["read", "ir", "write"] },
    diagnostics: {
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
    },
    losses: [],
    outputs: [],
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

test("native convert failure blocks the output file instead of writing empty text", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-pipeline-fail-"));
    const outputPath = join(workdir, "out.md");
    const { ctx, bindings } = stubBindings({
        convertDocument: () => ({
            exitCode: 1,
            text: "",
            reportJson: failedReportJson,
        }),
    });

    try {
        const result = await runConversionPipeline({
            ctx,
            bindings,
            inputPath: "broken.docx",
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
    const { ctx, bindings } = stubBindings({
        supportsMarkdownProject: () => true,
        convertMarkdownProject: () => ({
            exitCode: 1,
            indexMarkdown: "",
            publishedAssets: [],
            publishedChapters: [],
            unresolvedAssets: [],
            outputDir: "",
            reportJson: failedReportJson,
        }),
    });

    try {
        const result = await runMarkdownProjectPipeline({
            ctx,
            bindings,
            inputPath: "broken.docx",
            outputDir,
            resolved: { from: "docx", to: "markdown-project", ambiguous: false, hints: [".docx"] },
            shared,
            toolVersion: "0.0.0-test",
        });

        assert.equal(result.exitCode, ExitCode.IrOrWriterFailure);
        assert.equal(result.report.status, "blocked");
        assert.equal(result.report.outputs?.length, 0, "failed publish must not report outputs");
        assert.equal(await exists(outputDir), false);
        if (await exists(workdir)) {
            assert.deepEqual(await readdir(workdir), []);
        }
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});
