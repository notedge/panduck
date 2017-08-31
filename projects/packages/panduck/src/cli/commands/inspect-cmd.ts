import { stat } from "node:fs/promises";

import type { Cli, ParsedOptions } from "@vmz/commander";

import {
    assertReadableInput,
    buildBlockedReport,
    createContext,
    resolveFormats,
} from "../context.js";
import { emitReport } from "../diagnostics.js";
import { ExitCode } from "../exit-codes.js";
import { CliOptionError, parseInspectStage, readSharedOptions, readString } from "../options.js";
import { createReport } from "../report.js";
import { isStdinPath } from "../paths.js";

export function registerInspectCommand(cli: Cli): void {
    cli.command("inspect", "cli.cmd.inspect")
        .option("--from <format>", "cli.opt.from")
        .option("--stage <stage>", "cli.opt.stage")
        .option("--path <layout-path>", "cli.opt.path")
        .option("--budget <limit>", "cli.opt.budget")
        .option("--json", "cli.opt.json")
        .action((options: ParsedOptions) => runInspect(options));
}

async function runInspect(options: ParsedOptions): Promise<number> {
    try {
        const shared = readSharedOptions(options);
        const inputPath = options._[0] ?? readString(options, "input");
        if (!inputPath) {
            return ExitCode.InvalidArgs;
        }
        const stage = parseInspectStage(readString(options, "stage"));
        const layoutPath = readString(options, "path");
        const ctx = createContext();
        const resolved = resolveFormats(inputPath, undefined, shared);

        if (resolved.ambiguous) {
            const report = buildBlockedReport(
                ctx,
                "inspect",
                inputPath,
                resolved,
                "ambiguous input format",
                ExitCode.InputUnsupported,
                shared,
            );
            emitReport(report, shared.diagnostics, shared.json);
            return ExitCode.InputUnsupported;
        }

        try {
            await assertReadableInput(inputPath);
        } catch {
            const report = buildBlockedReport(
                ctx,
                "inspect",
                inputPath,
                resolved,
                "input is not readable",
                ExitCode.InputUnsupported,
                shared,
            );
            emitReport(report, shared.diagnostics, shared.json);
            return ExitCode.InputUnsupported;
        }

        const size = isStdinPath(inputPath) ? null : (await stat(inputPath)).size;

        if (stage === "decode" && resolved.from === "docx" && ctx.bindings?.inspectDecode) {
            try {
                const decode = ctx.bindings.inspectDecode(inputPath, layoutPath);
                const report = createReport({
                    operation: "inspect",
                    toolVersion: ctx.toolVersion,
                    status: "success",
                    inputs: [{ path: inputPath, format: decode.format }],
                    detection: {
                        format: decode.format,
                        outer: decode.outer,
                        inner: decode.inner,
                        confidence: "verified",
                        hints: resolved.hints,
                    },
                    pipeline: { stages: ["decode"] },
                    decodedParts: decode.parts.map((part) => ({
                        path: part.path,
                        compression_method: part.compressionMethod,
                        compressed_size: part.compressedSize,
                        uncompressed_size: part.uncompressedSize,
                        decoded_size: part.decodedSize,
                    })),
                    budgets: shared.budgets,
                    determinism: { profile: shared.profile ?? "default", config: shared.config ?? null },
                    statusPolicy: {
                        loss: shared.loss,
                        strict: shared.strict,
                        exit_code: ExitCode.Success,
                    },
                });
                if (size !== null) {
                    report.budgets = { ...report.budgets, input_bytes: size };
                }
                emitReport(report, shared.diagnostics, shared.json);
                return ExitCode.Success;
            } catch (error) {
                const message = error instanceof Error ? error.message : String(error);
                const report = buildBlockedReport(
                    ctx,
                    "inspect",
                    inputPath,
                    resolved,
                    message,
                    ExitCode.IrOrWriterFailure,
                    shared,
                );
                emitReport(report, shared.diagnostics, shared.json);
                return ExitCode.IrOrWriterFailure;
            }
        }

        if (stage === "index" && resolved.from === "docx" && ctx.bindings?.inspectIndex) {
            try {
                const index = ctx.bindings.inspectIndex(inputPath);
                const report = createReport({
                    operation: "inspect",
                    toolVersion: ctx.toolVersion,
                    status: "success",
                    inputs: [{ path: inputPath, format: index.format }],
                    detection: {
                        format: index.format,
                        outer: index.outer,
                        inner: index.inner,
                        confidence: "verified",
                        hints: resolved.hints,
                    },
                    pipeline: { stages: ["index"] },
                    parts: index.parts,
                    budgets: shared.budgets,
                    determinism: { profile: shared.profile ?? "default", config: shared.config ?? null },
                    statusPolicy: {
                        loss: shared.loss,
                        strict: shared.strict,
                        exit_code: ExitCode.Success,
                    },
                });
                if (size !== null) {
                    report.budgets = { ...report.budgets, input_bytes: size };
                }
                emitReport(report, shared.diagnostics, shared.json);
                return ExitCode.Success;
            } catch (error) {
                const message = error instanceof Error ? error.message : String(error);
                const report = buildBlockedReport(
                    ctx,
                    "inspect",
                    inputPath,
                    resolved,
                    message,
                    ExitCode.IrOrWriterFailure,
                    shared,
                );
                emitReport(report, shared.diagnostics, shared.json);
                return ExitCode.IrOrWriterFailure;
            }
        }

        const report = createReport({
            operation: "inspect",
            toolVersion: ctx.toolVersion,
            status: stage === "probe" ? "success" : "partial",
            inputs: [{ path: inputPath, format: resolved.from }],
            detection: {
                format: resolved.from,
                confidence: resolved.from ? "hint" : undefined,
                hints: resolved.hints,
            },
            pipeline: {
                stages: [stage],
                blocked_reason:
                    stage === "probe"
                        ? undefined
                        : "deeper inspect stages require Acorn/Oak integration",
            },
            diagnostics:
                stage === "probe"
                    ? []
                    : [
                          {
                              owner: "panduck",
                              severity: "warning",
                              message: `inspect stage ${stage} is not implemented yet`,
                              code: "panduck.inspect.not_implemented",
                          },
                      ],
            budgets: shared.budgets,
            determinism: { profile: shared.profile ?? "default", config: shared.config ?? null },
            statusPolicy: {
                loss: shared.loss,
                strict: shared.strict,
                exit_code: stage === "probe" ? ExitCode.Success : ExitCode.IrOrWriterFailure,
            },
        });

        if (layoutPath) {
            report.diagnostics?.push({
                owner: "acorn",
                severity: "warning",
                message: `layout path ${layoutPath} indexing is not implemented yet`,
                code: "acorn.layout.not_implemented",
            });
        }

        if (size !== null) {
            report.budgets = { ...report.budgets, input_bytes: size };
        }

        emitReport(report, shared.diagnostics, shared.json);
        return stage === "probe" ? ExitCode.Success : ExitCode.IrOrWriterFailure;
    } catch (error) {
        if (error instanceof CliOptionError) {
            console.error(error.message);
            return error.exitCode;
        }
        console.error(error instanceof Error ? error.message : String(error));
        return ExitCode.InvalidArgs;
    }
}
