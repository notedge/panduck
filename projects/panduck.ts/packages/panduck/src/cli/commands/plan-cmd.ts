import type { Cli, ParsedOptions } from "@vmz/commander";

import {
    assertReadableInput,
    buildBlockedReport,
    buildUnlistedRouteReport,
    createContext,
    resolveFormats,
    validateIoPaths,
} from "../context.js";
import { createDiagnostic, textArg } from "../../diagnostic.js";
import { PANDUCK_DIAGNOSTIC_CODES } from "../../diagnostic-codes.js";
import {
    inputReadFailureReason,
    isRemoteUri,
    REMOTE_INPUT_DIAGNOSTIC,
} from "../paths.js";
import { emitReport } from "../diagnostics.js";
import { ExitCode } from "../exit-codes.js";
import { findFormat, isOperationReady, routeCoverage, routeListed } from "../format-registry.js";
import { CliOptionError, describeUnsupportedFlags, readSharedOptions, readString } from "../options.js";
import { hasPipeline } from "../pipeline.js";
import { configureConsoleBackend } from "../console-backend.js";
import { createReport } from "../report.js";

export function registerPlanCommand(cli: Cli): void {
    cli.command("plan", "cli.cmd.plan")
        .option("--to <format>", "cli.opt.to")
        .option("--from <format>", "cli.opt.from")
        .option("--profile <name>", "cli.opt.profile")
        .option("--config <file>", "cli.opt.config")
        .option("--budget <limit>", "cli.opt.budget")
        .option("--json", "cli.opt.json")
        .action((options: ParsedOptions) => runPlan(options));
}

async function runPlan(options: ParsedOptions): Promise<number> {
    try {
        const shared = readSharedOptions(options);
        const unsupported = describeUnsupportedFlags(options);
        if (unsupported.length > 0) {
            for (const line of unsupported) {
                console.error(line);
            }
            return ExitCode.InvalidArgs;
        }
        const inputPath = options._[0] ?? readString(options, "input");
        if (!inputPath) {
            return ExitCode.InvalidArgs;
        }
        const ctx = createContext();
        configureConsoleBackend(ctx.bindings, shared.logFile);
        const resolved = resolveFormats(inputPath, undefined, shared);

        if (resolved.ambiguous) {
            const report = buildBlockedReport(
                ctx,
                "plan",
                inputPath,
                resolved,
                "ambiguous input format",
                ExitCode.InputUnsupported,
                shared,
            );
            emitReport(report, shared.diagnostics, shared.json);
            return ExitCode.InputUnsupported;
        }

        if (!resolved.to) {
            const report = buildBlockedReport(
                ctx,
                "plan",
                inputPath,
                resolved,
                "target format is required for plan",
                ExitCode.InvalidArgs,
                shared,
            );
            emitReport(report, shared.diagnostics, shared.json);
            return ExitCode.InvalidArgs;
        }

        try {
            await assertReadableInput(inputPath);
        } catch {
            const reason = inputReadFailureReason(inputPath);
            const report = buildBlockedReport(
                ctx,
                "plan",
                inputPath,
                resolved,
                reason,
                ExitCode.InputUnsupported,
                shared,
                isRemoteUri(inputPath)
                    ? [
                          createDiagnostic({
                              code: REMOTE_INPUT_DIAGNOSTIC,
                              component: "input",
                              severity: "error",
                              fallback: reason,
                          }),
                      ]
                    : undefined,
            );
            emitReport(report, shared.diagnostics, shared.json);
            return ExitCode.InputUnsupported;
        }

        const source = resolved.from ? findFormat(ctx.registry, resolved.from) : undefined;
        const target = findFormat(ctx.registry, resolved.to);
        if (!target) {
            const report = buildBlockedReport(
                ctx,
                "plan",
                inputPath,
                resolved,
                `unknown target format ${resolved.to}`,
                ExitCode.InputUnsupported,
                shared,
            );
            emitReport(report, shared.diagnostics, shared.json);
            return ExitCode.InputUnsupported;
        }

        const routeReady = hasPipeline(ctx.bindings, resolved.from, resolved.to);

        if (!routeListed(ctx.registry, resolved.from, resolved.to)) {
            const report = buildUnlistedRouteReport(ctx, "plan", inputPath, resolved, shared);
            emitReport(report, shared.diagnostics, shared.json);
            return ExitCode.InputUnsupported;
        }

        if (!routeReady) {
            if (source && !isOperationReady(source, "read")) {
                const report = createReport({
                    operation: "plan",
                    toolVersion: ctx.toolVersion,
                    status: "blocked",
                    inputs: [{ path: inputPath, format: source.format }],
                    detection: { format: source.format, confidence: "hint", hints: resolved.hints },
                    pipeline: {
                        reader: source.format,
                        writer: target.format,
                        stages: ["detect", "read", "ir", "write", "publish"],
                        blocked_reason: `reader state is ${source.reader.state}`,
                    },
                    coverage: target.coverage,
                    diagnostics: [
                        createDiagnostic({
                            code: PANDUCK_DIAGNOSTIC_CODES.readerNotReady,
                            component: "plan",
                            severity: "error",
                            fallback: `source format ${source.format} is not ready for read`,
                            args: [["format", textArg(source.format)]],
                        }),
                    ],
                    determinism: { profile: shared.profile ?? "default", config: shared.config ?? null },
                    statusPolicy: { loss: shared.loss, strict: shared.strict, exit_code: ExitCode.InputUnsupported },
                });
                emitReport(report, shared.diagnostics, shared.json);
                return ExitCode.InputUnsupported;
            }

            if (!isOperationReady(target, "write")) {
                const report = createReport({
                    operation: "plan",
                    toolVersion: ctx.toolVersion,
                    status: "blocked",
                    inputs: [{ path: inputPath, format: resolved.from }],
                    detection: { format: resolved.from, confidence: "hint", hints: resolved.hints },
                    pipeline: {
                        reader: source?.format,
                        writer: target.format,
                        stages: ["detect", "read", "ir", "write", "publish"],
                        blocked_reason: `writer state is ${target.writer.state}`,
                    },
                    coverage: target.coverage,
                    diagnostics: [
                        createDiagnostic({
                            code: PANDUCK_DIAGNOSTIC_CODES.writerNotReady,
                            component: "plan",
                            severity: "error",
                            fallback: `target format ${target.format} is not ready for write`,
                            args: [["format", textArg(target.format)]],
                        }),
                    ],
                    determinism: { profile: shared.profile ?? "default", config: shared.config ?? null },
                    statusPolicy: { loss: shared.loss, strict: shared.strict, exit_code: ExitCode.InputUnsupported },
                });
                emitReport(report, shared.diagnostics, shared.json);
                return ExitCode.InputUnsupported;
            }
        }

        const route = routeCoverage(ctx.registry, resolved.from, resolved.to);
        const report = createReport({
            operation: "plan",
            toolVersion: ctx.toolVersion,
            status: routeReady && route.state === "ready" ? "success" : "partial",
            inputs: [{ path: inputPath, format: resolved.from }],
            detection: { format: resolved.from, confidence: resolved.from ? "hint" : undefined, hints: resolved.hints },
            pipeline: {
                reader: source?.format ?? resolved.from,
                writer: target.format,
                stages: ["detect", "read", "ir", "write", "publish"],
            },
            coverage: route,
            budgets: shared.budgets,
            determinism: { profile: shared.profile ?? "default", config: shared.config ?? null },
            statusPolicy: { loss: shared.loss, strict: shared.strict, exit_code: ExitCode.Success },
        });
        emitReport(report, shared.diagnostics, shared.json);
        return ExitCode.Success;
    } catch (error) {
        if (error instanceof CliOptionError) {
            console.error(error.message);
            return error.exitCode;
        }
        console.error(error instanceof Error ? error.message : String(error));
        return ExitCode.InvalidArgs;
    }
}
