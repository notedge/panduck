import type { Cli, ParsedOptions } from "@vmz/commander";

import {
    assertReadableInput,
    buildBlockedReport,
    createContext,
    resolveFormats,
} from "../context.js";
import { emitReport } from "../diagnostics.js";
import { ExitCode } from "../exit-codes.js";
import { findFormat } from "../format-registry.js";
import { CliOptionError, readSharedOptions, readString } from "../options.js";
import { createReport } from "../report.js";
import {
    emitPipelineResult,
    hasPipeline,
    inferCheckTarget,
    runCheckPipeline,
} from "../pipeline.js";

export function registerCheckCommand(cli: Cli): void {
    cli.command("check", "cli.cmd.check")
        .option("--from <format>", "cli.opt.from")
        .option("--format <format>", "cli.opt.to")
        .option("--strict", "cli.opt.strict")
        .option("--loss <policy>", "cli.opt.loss")
        .option("--diagnostics <mode>", "cli.opt.diagnostics")
        .option("--budget <limit>", "cli.opt.budget")
        .option("--json", "cli.opt.json")
        .action((options: ParsedOptions) => runCheck(options));
}

async function runCheck(options: ParsedOptions): Promise<number> {
    try {
        const shared = readSharedOptions(options);
        const inputPath = options._[0] ?? readString(options, "input");
        const outputFormat = readString(options, "format");
        if (!inputPath) {
            return ExitCode.InvalidArgs;
        }

        const ctx = createContext();
        const resolved = resolveFormats(inputPath, undefined, {
            ...shared,
            to: outputFormat ?? shared.to,
        });

        if (resolved.ambiguous) {
            const report = buildBlockedReport(
                ctx,
                "check",
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
                "check",
                inputPath,
                resolved,
                "input is not readable",
                ExitCode.InputUnsupported,
                shared,
            );
            emitReport(report, shared.diagnostics, shared.json);
            return ExitCode.InputUnsupported;
        }

        const source = resolved.from ? findFormat(ctx.registry, resolved.from) : undefined;
        let targetFormat = resolved.to;
        if (!targetFormat && resolved.from && ctx.bindings) {
            targetFormat = inferCheckTarget(ctx.bindings, resolved.from);
        }
        const target = targetFormat ? findFormat(ctx.registry, targetFormat) : undefined;
        const checkResolved = { ...resolved, to: targetFormat };

        if (!targetFormat) {
            const report = buildBlockedReport(
                ctx,
                "check",
                inputPath,
                checkResolved,
                "target format is required for check",
                ExitCode.InvalidArgs,
                shared,
            );
            emitReport(report, shared.diagnostics, shared.json);
            return ExitCode.InvalidArgs;
        }

        const routeReady = hasPipeline(ctx.bindings, resolved.from, targetFormat);

        if (routeReady && ctx.bindings) {
            try {
                const result = await runCheckPipeline({
                    ctx,
                    bindings: ctx.bindings,
                    inputPath,
                    resolved: checkResolved,
                    shared,
                });
                return emitPipelineResult(result, shared);
            } catch (error) {
                const message = error instanceof Error ? error.message : String(error);
                const report = buildBlockedReport(
                    ctx,
                    "check",
                    inputPath,
                    checkResolved,
                    message,
                    ExitCode.IrOrWriterFailure,
                    shared,
                );
                emitReport(report, shared.diagnostics, shared.json);
                return ExitCode.IrOrWriterFailure;
            }
        }

        const report = createReport({
            operation: "check",
            toolVersion: ctx.toolVersion,
            status: "partial",
            inputs: [{ path: inputPath, format: resolved.from }],
            detection: { format: resolved.from, confidence: resolved.from ? "hint" : undefined, hints: resolved.hints },
            pipeline: {
                reader: source?.format,
                writer: target?.format,
                stages: ["validate"],
                blocked_reason: routeReady
                    ? "native bindings are unavailable"
                    : "conversion route is not wired yet",
            },
            coverage: source?.coverage ?? target?.coverage,
            diagnostics: [
                {
                    owner: "panduck",
                    severity: "warning",
                    message: routeReady
                        ? "check requires native bindings for wired routes"
                        : "check currently validates readability and registry presence only",
                    code: "panduck.check.partial",
                },
            ],
            budgets: shared.budgets,
            determinism: { profile: shared.profile ?? "default", config: shared.config ?? null },
            statusPolicy: {
                loss: shared.loss,
                strict: shared.strict,
                exit_code: shared.strict ? ExitCode.LossPolicyViolation : ExitCode.Success,
            },
        });

        emitReport(report, shared.diagnostics, shared.json);
        return shared.strict ? ExitCode.LossPolicyViolation : ExitCode.Success;
    } catch (error) {
        if (error instanceof CliOptionError) {
            console.error(error.message);
            return error.exitCode;
        }
        console.error(error instanceof Error ? error.message : String(error));
        return ExitCode.InvalidArgs;
    }
}
