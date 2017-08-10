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

export function registerCheckCommand(cli: Cli): void {
    cli.command("check", "cli.cmd.check")
        .option("--from <format>", "cli.opt.from")
        .option("--output <file>", "cli.opt.output")
        .option("--format <format>", "cli.opt.to")
        .option("--strict", "cli.opt.strict")
        .option("--budget <limit>", "cli.opt.budget")
        .option("--json", "cli.opt.json")
        .action((options: ParsedOptions) => runCheck(options));
}

async function runCheck(options: ParsedOptions): Promise<number> {
    try {
        const shared = readSharedOptions(options);
        const inputPath = options._[0] ?? readString(options, "input");
        const outputPath = readString(options, "output");
        const outputFormat = readString(options, "format");
        if (!inputPath) {
            return ExitCode.InvalidArgs;
        }

        const ctx = createContext();
        const resolved = resolveFormats(inputPath, outputPath, {
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
            if (outputPath && outputPath !== "-") {
                await assertReadableInput(outputPath);
            }
        } catch {
            const report = buildBlockedReport(
                ctx,
                "check",
                inputPath,
                resolved,
                "input or output is not readable",
                ExitCode.InputUnsupported,
                shared,
            );
            emitReport(report, shared.diagnostics, shared.json);
            return ExitCode.InputUnsupported;
        }

        const source = resolved.from ? findFormat(ctx.registry, resolved.from) : undefined;
        const target = resolved.to ? findFormat(ctx.registry, resolved.to) : undefined;
        const report = createReport({
            operation: "check",
            toolVersion: ctx.toolVersion,
            status: "partial",
            inputs: [{ path: inputPath, format: resolved.from }],
            outputs: outputPath ? [{ path: outputPath, published: true }] : [],
            detection: { format: resolved.from, confidence: resolved.from ? "hint" : undefined, hints: resolved.hints },
            pipeline: {
                reader: source?.format,
                writer: target?.format,
                stages: ["validate"],
                blocked_reason: "full contract validation is not implemented yet",
            },
            coverage: source?.coverage ?? target?.coverage,
            diagnostics: [
                {
                    owner: "panduck",
                    severity: "warning",
                    message: "check currently validates readability and registry presence only",
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
