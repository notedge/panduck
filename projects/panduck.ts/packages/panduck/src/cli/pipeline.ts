import { mkdir, mkdtemp, rename, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";
import { randomBytes } from "node:crypto";

import type { ConvertResponse, PanduckBindings } from "../types.js";
import { createDiagnostic, parseReportEnvelope, textArg, u64Arg } from "../diagnostic.js";
import type { DiagnosticRecord } from "../diagnostic.js";
import { ExitCode } from "./exit-codes.js";
import { containerBudgetsJson } from "./options.js";
import type { SharedCliOptions } from "./options.js";
import { createReport, type PanduckReport } from "./report.js";
import { emitReport } from "./diagnostics.js";
import type { CliContext } from "./context.js";
import type { ResolvedFormats } from "./context.js";
import { isStdinPath, isStdoutPath } from "./paths.js";

export type PipelineResult = {
    exitCode: number;
    report: PanduckReport;
};

/**
 * Maps native diagnostics to the stable CLI exit-code matrix.
 *
 * The native layer reports every failure as `exit_code: 1` and carries the
 * structured identity in diagnostics, so the CLI must classify by code rather
 * than propagate the transport value.
 */
export function exitCodeForDiagnostics(diagnostics: readonly DiagnosticRecord[]): number {
    const codes = diagnostics.map((item) => item.code);
    if (codes.some((code) => code.endsWith("budget-exhausted"))) {
        return ExitCode.BudgetExceeded;
    }
    if (codes.some((code) => code.endsWith("parse-failed") || code.endsWith("invalid-input"))) {
        return ExitCode.ParseFailure;
    }
    if (
        codes.some(
            (code) =>
                code.endsWith("unsupported")
                || code.endsWith("not-implemented")
                || code.endsWith("not-ready")
                || code.endsWith("ambiguous")
                || code.endsWith("not-readable"),
        )
    ) {
        return ExitCode.InputUnsupported;
    }
    return ExitCode.IrOrWriterFailure;
}

function budgetDiagnostic(key: string, limit: number, observed: number): DiagnosticRecord {
    return createDiagnostic({
        code: "panduck.budget.exhausted",
        component: "budgets",
        severity: "error",
        fallback: `${key} budget exceeded: observed ${observed} bytes, limit ${limit}`,
        args: [
            ["budget", textArg(key)],
            ["limit", u64Arg(limit)],
            ["observed", u64Arg(observed)],
        ],
    });
}

function exceededBudget(
    shared: SharedCliOptions,
    key: string,
    observed: number,
): { limit: number; diagnostic: DiagnosticRecord } | null {
    const limit = shared.budgets[key];
    if (limit === undefined || observed <= limit) {
        return null;
    }
    return { limit, diagnostic: budgetDiagnostic(key, limit, observed) };
}

async function inputByteLength(inputPath: string, bytes?: Uint8Array): Promise<number> {
    if (bytes) {
        return bytes.byteLength;
    }
    if (isStdinPath(inputPath)) {
        return 0;
    }
    try {
        const info = await stat(inputPath);
        return info.size;
    } catch {
        // Missing inputs must reach the native layer so it can emit the
        // structured diagnostic; a budget check cannot substitute for it.
        return 0;
    }
}

function outputByteLength(response: ConvertResponse): number {
    if (response.binary && response.binary.length > 0) {
        return response.binary.byteLength;
    }
    const text = response.text ?? response.html ?? "";
    return Buffer.byteLength(text, "utf8");
}

/** Reads all of standard input; `-` input paths route through the bytes binding. */
async function readStdinBytes(): Promise<Uint8Array> {
    const chunks: Buffer[] = [];
    for await (const chunk of process.stdin) {
        chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk as string));
    }
    return new Uint8Array(Buffer.concat(chunks));
}

export function hasPipeline(
    bindings: PanduckBindings | null,
    from: string | undefined,
    to: string,
): boolean {
    if (!bindings?.supportsConversion || !from) {
        return false;
    }
    return bindings.supportsConversion(from, to);
}

export function inferCheckTarget(bindings: PanduckBindings, from: string): string | undefined {
    const routes = bindings.supportedConversions?.() ?? [];
    const targets = routes
        .filter((route) => route.startsWith(`${from}:`))
        .map((route) => route.slice(from.length + 1));
    if (targets.length === 1) {
        return targets[0];
    }
    if (targets.includes("markdown")) {
        return "markdown";
    }
    return undefined;
}

export async function runCheckPipeline(input: {
    ctx: CliContext;
    bindings: PanduckBindings;
    inputPath: string;
    resolved: ResolvedFormats;
    shared: SharedCliOptions;
}): Promise<PipelineResult> {
    const { bindings, inputPath, resolved, shared, ctx } = input;
    const from = resolved.from;
    const to = resolved.to;
    if (!from || !to) {
        throw new Error("check requires resolved from/to formats");
    }

    const workdir = await mkdtemp(join(tmpdir(), "panduck-check-"));
    const extension = to === "markdown" ? ".md" : `.${to}`;
    const outputPath = join(workdir, `check${extension}`);

    const result = await runConversionPipeline({
        ctx,
        bindings,
        inputPath,
        outputPath,
        resolved: { ...resolved, to },
        shared,
        toolVersion: ctx.toolVersion,
        operation: "check",
        suppressStdout: true,
        publishOutput: false,
    });

    return {
        exitCode: result.exitCode,
        report: {
            ...result.report,
            operation: "check",
            outputs: [{ path: outputPath, published: false }],
            pipeline: {
                ...result.report.pipeline,
                stages: ["read", "ir", "write", "validate"],
            },
        },
    };
}

export async function runConversionPipeline(input: {
    ctx: CliContext;
    bindings: PanduckBindings;
    inputPath: string;
    outputPath: string | undefined;
    resolved: ResolvedFormats;
    shared: SharedCliOptions;
    toolVersion: string;
    operation?: "convert" | "check";
    suppressStdout?: boolean;
    publishOutput?: boolean;
}): Promise<PipelineResult> {
    const {
        bindings,
        inputPath,
        outputPath,
        resolved,
        shared,
        ctx,
        operation = "convert",
        suppressStdout = false,
        publishOutput = true,
    } = input;
    const from = resolved.from;
    const to = resolved.to;
    if (!from || !to) {
        throw new Error("conversion requires resolved from/to formats");
    }

    let inputBytes: Uint8Array | undefined;
    let response: ConvertResponse;
    if (isStdinPath(inputPath)) {
        if (!bindings.convertBytesMemory) {
            return blocked(ctx, operation, inputPath, resolved, shared, "native convertBytesMemory binding is missing");
        }
        inputBytes = await readStdinBytes();
    } else if (!bindings.convertDocument) {
        return blocked(ctx, operation, inputPath, resolved, shared, "native convertDocument binding is missing");
    }

    const inputObserved = await inputByteLength(inputPath, inputBytes);
    const overInput = exceededBudget(shared, "input_bytes", inputObserved);
    if (overInput) {
        return blocked(
            ctx,
            operation,
            inputPath,
            resolved,
            shared,
            `input_bytes budget exceeded: observed ${inputObserved}, limit ${overInput.limit}`,
            [overInput.diagnostic],
            ExitCode.BudgetExceeded,
        );
    }

    const budgetsJson = containerBudgetsJson(shared.budgets);
    response = inputBytes
        ? bindings.convertBytesMemory!(from, to, "stdin", inputBytes, undefined, budgetsJson)
        : bindings.convertDocument!(from, to, inputPath, undefined, budgetsJson);
    const parsedReport = parseReportEnvelope(response.reportJson, { component: "convert" });

    if (response.exitCode !== 0) {
        return blocked(
            ctx,
            operation,
            inputPath,
            resolved,
            shared,
            `native conversion failed with exit code ${response.exitCode}`,
            parsedReport.diagnostics,
            exitCodeForDiagnostics(parsedReport.diagnostics),
        );
    }

    const outputObserved = outputByteLength(response);
    const overOutput = exceededBudget(shared, "output_bytes", outputObserved);
    if (overOutput) {
        return blocked(
            ctx,
            operation,
            inputPath,
            resolved,
            shared,
            `output_bytes budget exceeded: observed ${outputObserved}, limit ${overOutput.limit}`,
            [overOutput.diagnostic],
            ExitCode.BudgetExceeded,
        );
    }

    const text = response.text ?? response.html ?? "";
    const binary = response.binary;

    if (publishOutput && outputPath && !isStdoutPath(outputPath)) {
        if (binary && binary.length > 0) {
            await publishBinary(outputPath, binary);
        } else {
            await publishText(outputPath, text);
        }
    } else if (!suppressStdout && (isStdoutPath(outputPath) || !outputPath)) {
        if (binary && binary.length > 0) {
            process.stdout.write(binary);
        } else {
            process.stdout.write(text);
        }
    }

    const lossCount = parsedReport.losses.length;
    const status = lossCount > 0 ? "success_with_loss" : "success";
    let exitCode: number = ExitCode.Success;
    if (shared.strict || shared.loss === "deny") {
        if (lossCount > 0 || status === "success_with_loss") {
            exitCode = ExitCode.LossPolicyViolation;
        }
    }

    const report = createReport({
        operation,
        toolVersion: ctx.toolVersion,
        status,
        inputs: [{ path: inputPath, format: from }],
        detection: {
            format: from,
            ...(from === "docx" ? { outer: "zip", inner: "opc" } : {}),
            confidence: "verified",
            hints: resolved.hints,
        },
        pipeline: {
            reader: from,
            writer: to,
            stages: operation === "check" ? ["read", "ir", "write", "validate"] : ["read", "ir", "write", "publish"],
        },
        losses: parsedReport.losses,
        diagnostics: parsedReport.diagnostics,
        outputs: outputPath
            ? [{ path: outputPath, published: publishOutput && !isStdoutPath(outputPath) }]
            : [{ path: "-", published: !suppressStdout }],
        budgets: shared.budgets,
        determinism: { profile: shared.profile ?? "default", config: shared.config ?? null },
        statusPolicy: { loss: shared.loss, strict: shared.strict, exit_code: exitCode },
    });

    return { exitCode, report };
}

async function publishBinary(path: string, content: Uint8Array): Promise<void> {
    const dir = dirname(path);
    await mkdir(dir, { recursive: true });
    const temp = join(dir, `.${basename(path)}.${randomBytes(4).toString("hex")}.tmp`);
    await writeFile(temp, content);
    await rename(temp, path);
}

async function publishText(path: string, content: string): Promise<void> {
    const dir = dirname(path);
    await mkdir(dir, { recursive: true });
    const temp = join(dir, `.${basename(path)}.${randomBytes(4).toString("hex")}.tmp`);
    await writeFile(temp, content, "utf8");
    await rename(temp, path);
}

function blocked(
    ctx: CliContext,
    operation: "convert" | "check",
    inputPath: string,
    resolved: ResolvedFormats,
    shared: SharedCliOptions,
    reason: string,
    diagnostics?: PanduckReport["diagnostics"],
    exitCode: number = ExitCode.IrOrWriterFailure,
): PipelineResult {
    const report = createReport({
        operation,
        toolVersion: ctx.toolVersion,
        status: "blocked",
        inputs: [{ path: inputPath, format: resolved.from }],
        pipeline: { blocked_reason: reason },
        diagnostics:
            diagnostics && diagnostics.length > 0
                ? diagnostics
                : [createDiagnostic({
                    code: "panduck.cli.blocked",
                    component: "cli",
                    severity: "error",
                    fallback: reason,
                })],
        budgets: shared.budgets,
        determinism: { profile: shared.profile ?? "default", config: shared.config ?? null },
        statusPolicy: { loss: shared.loss, strict: shared.strict, exit_code: exitCode },
    });
    return { exitCode, report };
}

export async function runMarkdownProjectPipeline(input: {
    ctx: CliContext;
    bindings: PanduckBindings;
    inputPath: string;
    outputDir: string;
    resolved: ResolvedFormats;
    shared: SharedCliOptions;
    toolVersion: string;
}): Promise<PipelineResult> {
    const { bindings, inputPath, outputDir, resolved, shared, ctx, toolVersion } = input;
    const from = resolved.from;
    if (!from) {
        throw new Error("markdown project conversion requires resolved source format");
    }

    if (!bindings.convertMarkdownProject || !bindings.supportsMarkdownProject) {
        return blocked(
            ctx,
            "convert",
            inputPath,
            resolved,
            shared,
            "native convertMarkdownProject binding is missing",
        );
    }

    if (!bindings.supportsMarkdownProject(from)) {
        return blocked(
            ctx,
            "convert",
            inputPath,
            resolved,
            shared,
            `source format ${from} does not support markdown project export`,
        );
    }

    let inputBytes: Uint8Array | undefined;
    if (isStdinPath(inputPath)) {
        if (!bindings.convertMarkdownProjectBytes) {
            return blocked(
                ctx,
                "convert",
                inputPath,
                resolved,
                shared,
                "native convertMarkdownProjectBytes binding is missing",
            );
        }
        inputBytes = await readStdinBytes();
    }

    const inputObserved = await inputByteLength(inputPath, inputBytes);
    const overInput = exceededBudget(shared, "input_bytes", inputObserved);
    if (overInput) {
        return blocked(
            ctx,
            "convert",
            inputPath,
            resolved,
            shared,
            `input_bytes budget exceeded: observed ${inputObserved}, limit ${overInput.limit}`,
            [overInput.diagnostic],
            ExitCode.BudgetExceeded,
        );
    }

    const budgetsJson = containerBudgetsJson(shared.budgets);
    const response = inputBytes
        ? bindings.convertMarkdownProjectBytes!(from, "stdin", inputBytes, outputDir, undefined, budgetsJson)
        : bindings.convertMarkdownProject!(from, inputPath, outputDir, undefined, budgetsJson);
    const parsedReport = parseReportEnvelope(response.reportJson, { component: "convert" });

    if (response.exitCode !== 0) {
        return blocked(
            ctx,
            "convert",
            inputPath,
            resolved,
            shared,
            `native markdown project conversion failed with exit code ${response.exitCode}`,
            parsedReport.diagnostics,
            exitCodeForDiagnostics(parsedReport.diagnostics),
        );
    }

    const lossCount = parsedReport.losses.length;
    const unresolvedCount = response.unresolvedAssets.length;
    const status =
        lossCount > 0 || unresolvedCount > 0 ? "success_with_loss" : "success";
    let exitCode: number = ExitCode.Success;
    if (shared.strict || shared.loss === "deny") {
        if (lossCount > 0 || unresolvedCount > 0 || status === "success_with_loss") {
            exitCode = ExitCode.LossPolicyViolation;
        }
    }

    const report = createReport({
        operation: "convert",
        toolVersion,
        status,
        inputs: [{ path: inputPath, format: from }],
        detection: { format: from, confidence: "verified", hints: resolved.hints },
        pipeline: {
            reader: from,
            writer: "markdown-project",
            stages: ["read", "ir", "write", "materialize", "publish"],
        },
        losses: parsedReport.losses,
        diagnostics: parsedReport.diagnostics,
        outputs: [
            { path: join(outputDir, "index.md"), published: true },
            ...response.publishedChapters.map((chapter) => ({
                path: join(outputDir, chapter),
                published: true,
            })),
            ...response.publishedAssets.map((asset) => ({
                path: join(outputDir, asset),
                published: true,
            })),
            { path: join(outputDir, "panduck.report.json"), published: true },
        ],
        budgets: shared.budgets,
        determinism: { profile: shared.profile ?? "default", config: shared.config ?? null },
        statusPolicy: { loss: shared.loss, strict: shared.strict, exit_code: exitCode },
    });

    return { exitCode, report };
}

export async function emitPipelineResult(
    result: PipelineResult,
    shared: SharedCliOptions,
): Promise<number> {
    emitReport(result.report, shared.diagnostics, shared.json);
    return result.exitCode;
}
