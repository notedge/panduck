import { mkdir, mkdtemp, readFile, rename, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";
import { randomBytes } from "node:crypto";

import type { PanduckBindings } from "../types.js";
import { PANDUCK_DIAGNOSTIC_CODES } from "../diagnostic-codes.js";
import { createDiagnostic, textArg, u64Arg } from "../diagnostic.js";
import type { DiagnosticRecord } from "../diagnostic.js";
import { convertInMemory, exportProjectInMemory, type StageOutcome, type ProjectOutcome } from "../primitives.js";
import { publishMarkdownProject as materializeMarkdownProject } from "../publish-project.js";
import { buildProjectReportJson } from "../native-report.js";
import { ExitCode } from "./exit-codes.js";
import { containerBudgetsJson } from "./options.js";
import type { SharedCliOptions } from "./options.js";
import { createReport, type PanduckReport } from "./report.js";
import { emitReport } from "./diagnostics.js";
import type { CliContext } from "./context.js";
import type { ResolvedFormats } from "./context.js";
import { routeCoverage } from "./format-registry.js";
import { isStdinPath, isStdoutPath } from "./paths.js";

export type PipelineResult = {
    exitCode: number;
    report: PanduckReport;
};

/** Hard ceiling for one exported payload (128 MiB), independent of `--budget`. */
export const EXPORT_BYTES_CEILING = 128 * 1024 * 1024;

/** Wall-clock deadline for one native conversion (300 s), checked after return. */
export const CONVERSION_DEADLINE_MS = 300_000;

/**
 * Maps native diagnostics to the stable CLI exit-code matrix.
 *
 * The native layer reports structured diagnostics for every failure, so the CLI
 * classifies by diagnostic code rather than by a transport value. A writer-side
 * `unsupported` diagnostic names the target format and maps to
 * `IrOrWriterFailure`, while reader-side and route-level unsupported cases keep
 * `InputUnsupported`.
 */
export function exitCodeForDiagnostics(
    diagnostics: readonly DiagnosticRecord[],
    targetFormat?: string,
): number {
    const codes = diagnostics.map((item) => item.code);
    if (codes.some((code) => code.endsWith("budget-exhausted"))) {
        return ExitCode.BudgetExceeded;
    }
    if (codes.some((code) => code.endsWith("parse-failed") || code.endsWith("invalid-input"))) {
        return ExitCode.ParseFailure;
    }
    if (codes.some((code) => code.endsWith("unsupported")) && isWriterUnsupported(diagnostics, targetFormat)) {
        return ExitCode.IrOrWriterFailure;
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

/** True when an `unsupported` diagnostic names the requested target format. */
function isWriterUnsupported(diagnostics: readonly DiagnosticRecord[], targetFormat?: string): boolean {
    if (!targetFormat) {
        return false;
    }
    return diagnostics.some(
        (diagnostic) =>
            diagnostic.code.endsWith("unsupported")
            && diagnostic.message.args.some(
                ([name, value]) =>
                    name === "format"
                    && typeof value === "object"
                    && value !== null
                    && typeof (value as { text?: unknown }).text === "string"
                    && (value as { text: string }).text === targetFormat,
            ),
    );
}

function exportCeilingDiagnostic(observed: number): DiagnosticRecord {
    return createDiagnostic({
        code: PANDUCK_DIAGNOSTIC_CODES.exportSizeExceeded,
        component: "export",
        severity: "error",
        fallback: `exported payload exceeds the hard ceiling: observed ${observed} bytes, ceiling ${EXPORT_BYTES_CEILING}`,
        args: [
            ["limit", u64Arg(EXPORT_BYTES_CEILING)],
            ["observed", u64Arg(observed)],
        ],
    });
}

/** Returns the hard-ceiling diagnostic when the payload is too large. */
export function exportCeilingStop(observed: number): DiagnosticRecord | null {
    return observed > EXPORT_BYTES_CEILING ? exportCeilingDiagnostic(observed) : null;
}

function deadlineDiagnostic(observedMs: number): DiagnosticRecord {
    return createDiagnostic({
        code: PANDUCK_DIAGNOSTIC_CODES.deadlineExceeded,
        component: "convert",
        severity: "error",
        fallback: `conversion exceeded the hard deadline: observed ${observedMs} ms, limit ${CONVERSION_DEADLINE_MS} ms`,
        args: [
            ["limit", u64Arg(CONVERSION_DEADLINE_MS)],
            ["observed", u64Arg(observedMs)],
        ],
    });
}

/** Returns the deadline diagnostic when one native call ran too long. */
export function deadlineStop(observedMs: number): DiagnosticRecord | null {
    return observedMs > CONVERSION_DEADLINE_MS ? deadlineDiagnostic(observedMs) : null;
}

function budgetDiagnostic(key: string, limit: number, observed: number): DiagnosticRecord {
    return createDiagnostic({
        code: PANDUCK_DIAGNOSTIC_CODES.budgetExhausted,
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

function outputByteLength(outcome: StageOutcome): number {
    if (outcome.binary && outcome.binary.length > 0) {
        return outcome.binary.byteLength;
    }
    const text = outcome.text ?? outcome.html ?? "";
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
    if (
        !bindings?.supportsConversion
        || !bindings.importDocument
        || !bindings.transformDocument
        || !bindings.exportDocument
        || !from
    ) {
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

    let inputBytes: Uint8Array;
    if (!bindings.importDocument || !bindings.transformDocument || !bindings.exportDocument) {
        return blocked(ctx, operation, inputPath, resolved, shared, "native conversion primitives are missing");
    }
    if (isStdinPath(inputPath)) {
        inputBytes = await readStdinBytes();
    } else {
        try {
            inputBytes = await readFile(inputPath);
        } catch (error) {
            const notReadable = inputReadError(inputPath, error);
            return blocked(
                ctx,
                operation,
                inputPath,
                resolved,
                shared,
                "input is not readable",
                [notReadable.diagnostic],
                notReadable.exitCode,
            );
        }
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

    const label = isStdinPath(inputPath) ? "stdin" : inputPath;
    const budgetsJson = containerBudgetsJson(shared.budgets);
    const startedAt = Date.now();
    let outcome: StageOutcome | undefined;
    let nativePanic: unknown;
    try {
        outcome = convertInMemory(bindings, { from, to, label, bytes: inputBytes, budgetsJson });
    } catch (error) {
        nativePanic = error;
    }
    const elapsedMs = Date.now() - startedAt;

    if (nativePanic !== undefined || !outcome) {
        const message = nativePanic instanceof Error ? nativePanic.message : String(nativePanic);
        return blocked(ctx, operation, inputPath, resolved, shared, "native conversion failed with exit code 1", [
            createDiagnostic({
                code: PANDUCK_DIAGNOSTIC_CODES.nativePanic,
                component: "convert",
                severity: "error",
                fallback: `Panduck native binary panicked while converting ${label}: ${message}`,
                args: [["message", textArg(message)]],
            }),
        ]);
    }

    if (!outcome.ok) {
        return blocked(
            ctx,
            operation,
            inputPath,
            resolved,
            shared,
            "native conversion failed with exit code 1",
            outcome.diagnostics,
            exitCodeForDiagnostics(outcome.diagnostics, to),
        );
    }

    const overdue = deadlineStop(elapsedMs);
    if (overdue) {
        return blocked(
            ctx,
            operation,
            inputPath,
            resolved,
            shared,
            `conversion deadline exceeded: observed ${elapsedMs} ms, limit ${CONVERSION_DEADLINE_MS} ms`,
            [overdue],
            ExitCode.BudgetExceeded,
        );
    }

    const outputObserved = outputByteLength(outcome);
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

    const overCeiling = exportCeilingStop(outputObserved);
    if (overCeiling) {
        return blocked(
            ctx,
            operation,
            inputPath,
            resolved,
            shared,
            `exported payload exceeds the hard ceiling: observed ${outputObserved} bytes, ceiling ${EXPORT_BYTES_CEILING}`,
            [overCeiling],
            ExitCode.BudgetExceeded,
        );
    }

    const text = outcome.text ?? outcome.html ?? "";
    const binary = outcome.binary;

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

    const lossCount = outcome.losses.length;
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
        coverage: routeCoverage(ctx.registry, from, to),
        losses: outcome.losses,
        diagnostics: outcome.diagnostics,
        outputs: outputPath
            ? [{ path: outputPath, published: publishOutput && !isStdoutPath(outputPath) }]
            : [{ path: "-", published: !suppressStdout }],
        budgets: shared.budgets,
        determinism: { profile: shared.profile ?? "default", config: shared.config ?? null },
        statusPolicy: { loss: shared.loss, strict: shared.strict, exit_code: exitCode },
    });

    return { exitCode, report };
}

function inputReadError(
    inputPath: string,
    error: unknown,
): { diagnostic: DiagnosticRecord; exitCode: number } {
    const errno = (error as NodeJS.ErrnoException).code;
    const notReadable = errno === "ENOENT" || errno === "EACCES" || errno === "EPERM" || errno === "EISDIR";
    return {
        diagnostic: createDiagnostic({
            code: notReadable ? PANDUCK_DIAGNOSTIC_CODES.readNotReadable : PANDUCK_DIAGNOSTIC_CODES.readIoFailed,
            component: "read",
            severity: "error",
            fallback: notReadable ? "input is not readable" : "input could not be read",
            args: [
                ["path", textArg(inputPath)],
                ["message", textArg(error instanceof Error ? error.message : String(error))],
            ],
        }),
        exitCode: notReadable ? ExitCode.InputUnsupported : ExitCode.IrOrWriterFailure,
    };
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
                    code: PANDUCK_DIAGNOSTIC_CODES.cliBlocked,
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

    if (
        !bindings.importDocument
        || !bindings.transformDocument
        || !bindings.exportMarkdownProject
        || !bindings.supportsMarkdownProject
    ) {
        return blocked(
            ctx,
            "convert",
            inputPath,
            resolved,
            shared,
            "native markdown project primitives are missing",
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

    let inputBytes: Uint8Array;
    if (isStdinPath(inputPath)) {
        inputBytes = await readStdinBytes();
    } else {
        try {
            inputBytes = await readFile(inputPath);
        } catch (error) {
            const notReadable = inputReadError(inputPath, error);
            return blocked(
                ctx,
                "convert",
                inputPath,
                resolved,
                shared,
                "input is not readable",
                [notReadable.diagnostic],
                notReadable.exitCode,
            );
        }
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

    const label = isStdinPath(inputPath) ? "stdin" : inputPath;
    const budgetsJson = containerBudgetsJson(shared.budgets);
    let outcome: ProjectOutcome | undefined;
    let nativePanic: unknown;
    try {
        outcome = exportProjectInMemory(bindings, { from, label, bytes: inputBytes, budgetsJson });
    } catch (error) {
        nativePanic = error;
    }

    if (nativePanic !== undefined || !outcome) {
        const message = nativePanic instanceof Error ? nativePanic.message : String(nativePanic);
        return blocked(ctx, "convert", inputPath, resolved, shared, "native markdown project conversion failed with exit code 1", [
            createDiagnostic({
                code: PANDUCK_DIAGNOSTIC_CODES.nativePanic,
                component: "convert",
                severity: "error",
                fallback: `Panduck native binary panicked while publishing ${label}: ${message}`,
                args: [["message", textArg(message)]],
            }),
        ]);
    }

    const lossCount = outcome.losses.length;
    const unresolvedCount = outcome.unresolvedAssets.length;
    const status =
        lossCount > 0 || unresolvedCount > 0 ? "success_with_loss" : "success";

    if (!outcome.ok) {
        return blocked(
            ctx,
            "convert",
            inputPath,
            resolved,
            shared,
            "native markdown project conversion failed with exit code 1",
            outcome.diagnostics,
            exitCodeForDiagnostics(outcome.diagnostics),
        );
    }
    let exitCode: number = ExitCode.Success;
    if (shared.strict || shared.loss === "deny") {
        if (lossCount > 0 || unresolvedCount > 0 || status === "success_with_loss") {
            exitCode = ExitCode.LossPolicyViolation;
        }
    }

    const nativeReportJson = buildProjectReportJson({
        from,
        label,
        status,
        passes: outcome.passes,
        coverage: outcome.coverage,
        losses: outcome.losses,
        diagnostics: outcome.diagnostics,
        chapters: outcome.chapters.map((chapter) => chapter.path),
        assetCount: outcome.assets.length,
        unresolvedAssets: outcome.unresolvedAssets,
    });

    try {
        await materializeMarkdownProject(outputDir, outcome, nativeReportJson);
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        return blocked(
            ctx,
            "convert",
            inputPath,
            resolved,
            shared,
            `failed to publish markdown project to ${outputDir}: ${message}`,
            [
                createDiagnostic({
                    code: PANDUCK_DIAGNOSTIC_CODES.publishFailed,
                    component: "convert",
                    severity: "error",
                    fallback: `failed to publish markdown project to ${outputDir}: ${message}`,
                    args: [["message", textArg(message)]],
                }),
            ],
            ExitCode.IrOrWriterFailure,
        );
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
        coverage: routeCoverage(ctx.registry, from, "markdown-project"),
        losses: outcome.losses,
        diagnostics: outcome.diagnostics,
        outputs: [
            { path: join(outputDir, "index.md"), published: true },
            ...outcome.chapters.map((chapter) => ({
                path: join(outputDir, chapter.path),
                published: true,
            })),
            ...outcome.assets.map((asset) => ({
                path: join(outputDir, asset.path),
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
