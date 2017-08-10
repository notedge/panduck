import type { Cli, ParsedOptions } from "@vmz/commander";

import {
    assertReadableInput,
    buildBlockedReport,
    createContext,
    resolveFormats,
    validateIoPaths,
} from "../context.js";
import { emitReport } from "../diagnostics.js";
import { ExitCode } from "../exit-codes.js";
import { findFormat, isOperationReady } from "../format-registry.js";
import { CliOptionError, readSharedOptions, readString } from "../options.js";
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
        const inputPath = options._[0] ?? readString(options, "input");
        if (!inputPath) {
            return ExitCode.InvalidArgs;
        }
        const ctx = createContext();
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
            const report = buildBlockedReport(
                ctx,
                "plan",
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
                    {
                        owner: "panduck",
                        severity: "error",
                        message: `source format ${source.format} is not ready for read`,
                        code: "panduck.reader.not_ready",
                    },
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
                    {
                        owner: "panduck",
                        severity: "error",
                        message: `target format ${target.format} is not ready for write`,
                        code: "panduck.writer.not_ready",
                    },
                ],
                determinism: { profile: shared.profile ?? "default", config: shared.config ?? null },
                statusPolicy: { loss: shared.loss, strict: shared.strict, exit_code: ExitCode.InputUnsupported },
            });
            emitReport(report, shared.diagnostics, shared.json);
            return ExitCode.InputUnsupported;
        }

        const report = createReport({
            operation: "plan",
            toolVersion: ctx.toolVersion,
            status: "success",
            inputs: [{ path: inputPath, format: resolved.from }],
            detection: { format: resolved.from, confidence: resolved.from ? "hint" : undefined, hints: resolved.hints },
            pipeline: {
                reader: source?.format ?? resolved.from,
                writer: target.format,
                stages: ["detect", "read", "ir", "write", "publish"],
            },
            coverage: target.coverage,
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
