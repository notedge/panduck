import { access } from "node:fs/promises";
import { constants } from "node:fs";

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
import {
    CliOptionError,
    parseAssetPolicy,
    parseLinkPolicy,
    readSharedOptions,
    readString,
} from "../options.js";
import { createReport } from "../report.js";
import { isStdoutPath } from "../paths.js";
import { configureConsoleBackend } from "../console-backend.js";
import { emitPipelineResult, hasPipeline, runConversionPipeline } from "../pipeline.js";

export function registerConvertCommand(cli: Cli): void {
    cli.command("convert", "cli.cmd.convert")
        .option("--to <format>", "cli.opt.to")
        .option("-o, --output <file>", "cli.opt.output")
        .option("--from <format>", "cli.opt.from")
        .option("--profile <name>", "cli.opt.profile")
        .option("--config <file>", "cli.opt.config")
        .option("--assets-dir <dir>", "cli.opt.assets-dir")
        .option("--asset-policy <policy>", "cli.opt.asset-policy")
        .option("--link-policy <policy>", "cli.opt.link-policy")
        .option("--loss <policy>", "cli.opt.loss")
        .option("--strict", "cli.opt.strict")
        .option("--allow-partial", "cli.opt.allow-partial")
        .option("--diagnostics <mode>", "cli.opt.diagnostics")
        .option("--batch", "cli.opt.batch")
        .option("--output-dir <dir>", "cli.opt.output-dir")
        .option("--report <file>", "cli.opt.report")
        .option("--overwrite", "cli.opt.overwrite")
        .option("--budget <limit>", "cli.opt.budget")
        .option("--dry-run", "cli.opt.dry-run")
        .action((options: ParsedOptions) => runConvert(options));
}

async function runConvert(options: ParsedOptions): Promise<number> {
    try {
        const shared = readSharedOptions(options);
        const inputPath = options._[0] ?? readString(options, "input");
        const outputPath = readString(options, "output");
        const reportPath = readString(options, "report");
        const batch = options.batch === true;
        const dryRun = options["dry-run"] === true;

        if (!inputPath) {
            return ExitCode.InvalidArgs;
        }
        if (batch && inputPath === "-") {
            console.error("standard input cannot be used with --batch");
            return ExitCode.InvalidArgs;
        }
        if (batch && !readString(options, "output-dir")) {
            console.error("--batch requires --output-dir");
            return ExitCode.InvalidArgs;
        }

        const overlapCode = validateIoPaths(inputPath, outputPath);
        if (overlapCode !== null) {
            console.error(`input and output paths must not be the same: ${inputPath}`);
            return overlapCode;
        }

        if (isStdoutPath(outputPath) && !shared.to) {
            console.error("writing to stdout requires an explicit --to format");
            return ExitCode.InvalidArgs;
        }

        const ctx = createContext();
        configureConsoleBackend(ctx.bindings, shared.logFile);
        const resolved = resolveFormats(inputPath, outputPath, shared);

        if (resolved.ambiguous) {
            const report = buildBlockedReport(
                ctx,
                "convert",
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
                "convert",
                inputPath,
                resolved,
                "target format is required",
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
                "convert",
                inputPath,
                resolved,
                "input is not readable",
                ExitCode.InputUnsupported,
                shared,
            );
            emitReport(report, shared.diagnostics, shared.json);
            return ExitCode.InputUnsupported;
        }

        if (outputPath && outputPath !== "-" && !shared.overwrite) {
            try {
                await access(outputPath, constants.F_OK);
                const report = buildBlockedReport(
                    ctx,
                    "convert",
                    inputPath,
                    resolved,
                    "output already exists",
                    ExitCode.OutputFailure,
                    shared,
                );
                emitReport(report, shared.diagnostics, shared.json);
                return ExitCode.OutputFailure;
            } catch {
                // target does not exist yet
            }
        }

        parseAssetPolicy(readString(options, "asset-policy"));
        parseLinkPolicy(readString(options, "link-policy"));

        const source = resolved.from ? findFormat(ctx.registry, resolved.from) : undefined;
        const target = findFormat(ctx.registry, resolved.to);
        if (!target) {
            const report = buildBlockedReport(
                ctx,
                "convert",
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

        if (resolved.from === "doc") {
            const report = buildBlockedReport(
                ctx,
                "convert",
                inputPath,
                resolved,
                "legacy .doc import requires OLE reader support; convert to .docx first",
                ExitCode.InputUnsupported,
                shared,
            );
            emitReport(report, shared.diagnostics, shared.json);
            return ExitCode.InputUnsupported;
        }

        if (!routeReady) {
            if (source && !isOperationReady(source, "read")) {
                const report = buildBlockedReport(
                    ctx,
                    "convert",
                    inputPath,
                    resolved,
                    `source format ${source.format} is not ready for read`,
                    ExitCode.InputUnsupported,
                    shared,
                );
                emitReport(report, shared.diagnostics, shared.json);
                return ExitCode.InputUnsupported;
            }

            if (!isOperationReady(target, "write")) {
                const report = buildBlockedReport(
                    ctx,
                    "convert",
                    inputPath,
                    resolved,
                    `target format ${target.format} is not ready for write`,
                    ExitCode.InputUnsupported,
                    shared,
                );
                emitReport(report, shared.diagnostics, shared.json);
                return ExitCode.InputUnsupported;
            }
        }

        if (dryRun) {
            const report = createReport({
                operation: "convert",
                toolVersion: ctx.toolVersion,
                status: "success",
                inputs: [{ path: inputPath, format: resolved.from }],
                detection: { format: resolved.from, confidence: "hint", hints: resolved.hints },
                pipeline: {
                    reader: source?.format ?? resolved.from,
                    writer: target.format,
                    stages: ["detect", "read", "ir", "write", "publish"],
                },
                outputs: outputPath ? [{ path: outputPath, published: false }] : [],
                budgets: shared.budgets,
                determinism: { profile: shared.profile ?? "default", config: shared.config ?? null },
                statusPolicy: { loss: shared.loss, strict: shared.strict, exit_code: ExitCode.Success },
            });
            emitReport(report, shared.diagnostics, shared.json);
            return ExitCode.Success;
        }

        if (routeReady && ctx.bindings) {
            try {
                const result = await runConversionPipeline({
                    ctx,
                    bindings: ctx.bindings,
                    inputPath,
                    outputPath,
                    resolved,
                    shared,
                    toolVersion: ctx.toolVersion,
                });
                return emitPipelineResult(result, shared);
            } catch (error) {
                const message = error instanceof Error ? error.message : String(error);
                const report = buildBlockedReport(
                    ctx,
                    "convert",
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
            operation: "convert",
            toolVersion: ctx.toolVersion,
            status: "blocked",
            inputs: [{ path: inputPath, format: resolved.from }],
            detection: { format: resolved.from, confidence: "hint", hints: resolved.hints },
            pipeline: {
                reader: source?.format ?? resolved.from,
                writer: target.format,
                stages: ["detect", "read", "ir", "write", "publish"],
                blocked_reason: "conversion pipeline is not wired yet",
            },
            diagnostics: [
                {
                    owner: "panduck",
                    severity: "error",
                    message: "conversion pipeline is not wired yet",
                    code: "panduck.convert.not_implemented",
                },
            ],
            outputs: outputPath ? [{ path: outputPath, published: false }] : [],
            budgets: shared.budgets,
            determinism: { profile: shared.profile ?? "default", config: shared.config ?? null },
            statusPolicy: { loss: shared.loss, strict: shared.strict, exit_code: ExitCode.IrOrWriterFailure },
        });

        emitReport(report, shared.diagnostics, shared.json);
        return ExitCode.IrOrWriterFailure;
    } catch (error) {
        if (error instanceof CliOptionError) {
            console.error(error.message);
            return error.exitCode;
        }
        console.error(error instanceof Error ? error.message : String(error));
        return ExitCode.InvalidArgs;
    }
}
