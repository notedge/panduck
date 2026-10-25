import assert from "node:assert/strict";
import { access, mkdtemp, readdir, rm } from "node:fs/promises";
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
        schema_version: "panduck.diagnostic/v0",
        diagnostics: [
            {
                code: "panduck.adapter.not-found",
                fallback: "input document is not a readable docx package",
                severity: "error",
            },
        ],
        truncated: 0,
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
        assert.ok(
            result.report.diagnostics?.some(
                (diagnostic) => diagnostic.code === "panduck.adapter.not-found",
            ),
            "native failure diagnostics must reach the CLI report",
        );
        assert.equal(await exists(outputPath), false, "failed conversion must not publish an empty file");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("native convert failure writes nothing to stdout", async () => {
    const { ctx, bindings } = stubBindings({
        convertDocument: () => ({
            exitCode: 4,
            text: "",
            reportJson: failedReportJson,
        }),
    });

    const stdout = process.stdout.write.bind(process.stdout);
    const chunks: string[] = [];
    process.stdout.write = ((chunk: string | Uint8Array) => {
        chunks.push(typeof chunk === "string" ? chunk : Buffer.from(chunk).toString("utf8"));
        return true;
    }) as typeof process.stdout.write;

    try {
        const result = await runConversionPipeline({
            ctx,
            bindings,
            inputPath: "broken.docx",
            outputPath: "-",
            resolved: { from: "docx", to: "markdown", ambiguous: false, hints: [".docx"] },
            shared,
            toolVersion: "0.0.0-test",
        });

        assert.equal(result.exitCode, ExitCode.IrOrWriterFailure);
        assert.equal(result.report.status, "blocked");
        assert.equal(chunks.join(""), "", "failed conversion must not emit empty stdout");
    } finally {
        process.stdout.write = stdout;
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
