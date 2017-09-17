import { mkdir, mkdtemp, rename, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";
import { randomBytes } from "node:crypto";

import type { PanduckBindings } from "../types.js";
import { ExitCode } from "./exit-codes.js";
import type { SharedCliOptions } from "./options.js";
import { createReport, type PanduckReport } from "./report.js";
import { emitReport } from "./diagnostics.js";
import type { CliContext } from "./context.js";
import type { ResolvedFormats } from "./context.js";
import { isStdoutPath } from "./paths.js";

export type PipelineResult = {
    exitCode: number;
    report: PanduckReport;
};

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

    if (!bindings.convertDocument) {
        return blocked(ctx, operation, inputPath, resolved, shared, "native convertDocument binding is missing");
    }

    const response = bindings.convertDocument(from, to, inputPath);
    const markdown = response.markdown ?? "";
    const binary = response.binary;
    const parsedReport = safeParseReport(response.reportJson);

    if (publishOutput && outputPath && !isStdoutPath(outputPath)) {
        if (binary && binary.length > 0) {
            await publishBinary(outputPath, binary);
        } else {
            await publishText(outputPath, markdown);
        }
    } else if (!suppressStdout && (isStdoutPath(outputPath) || !outputPath)) {
        if (binary && binary.length > 0) {
            process.stdout.write(binary);
        } else {
            process.stdout.write(markdown);
        }
    }

    const lossCount = Array.isArray(parsedReport?.losses) ? parsedReport.losses.length : 0;
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
        detection: { format: from, outer: "zip", inner: "opc", confidence: "verified", hints: resolved.hints },
        pipeline: {
            reader: from,
            writer: to,
            stages: operation === "check" ? ["read", "ir", "write", "validate"] : ["read", "ir", "write", "publish"],
        },
        losses: parsedReport?.losses as PanduckReport["losses"],
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

function safeParseReport(raw: string): Record<string, unknown> | null {
    try {
        return JSON.parse(raw) as Record<string, unknown>;
    } catch {
        return null;
    }
}

function blocked(
    ctx: CliContext,
    operation: "convert" | "check",
    inputPath: string,
    resolved: ResolvedFormats,
    shared: SharedCliOptions,
    reason: string,
): PipelineResult {
    const report = createReport({
        operation,
        toolVersion: ctx.toolVersion,
        status: "blocked",
        inputs: [{ path: inputPath, format: resolved.from }],
        pipeline: { blocked_reason: reason },
        diagnostics: [{ owner: "panduck", severity: "error", message: reason }],
        determinism: { profile: shared.profile ?? "default", config: shared.config ?? null },
        statusPolicy: { loss: shared.loss, strict: shared.strict, exit_code: ExitCode.IrOrWriterFailure },
    });
    return { exitCode: ExitCode.IrOrWriterFailure, report };
}

export async function emitPipelineResult(
    result: PipelineResult,
    shared: SharedCliOptions,
): Promise<number> {
    emitReport(result.report, shared.diagnostics, shared.json);
    return result.exitCode;
}
