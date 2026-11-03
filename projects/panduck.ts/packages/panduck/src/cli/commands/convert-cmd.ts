import { access } from "node:fs/promises";
import { constants } from "node:fs";

import type { Cli, ParsedOptions } from "@vmz/commander";

import {
    assertReadableInput,
    buildBlockedReport,
    createContext,
    resolveFormats,
    validateConversionPaths,
} from "../context.js";
import { createDiagnostic } from "../../diagnostic.js";
import { emitJson, emitReport, writeReportFile } from "../diagnostics.js";
import { ExitCode } from "../exit-codes.js";
import { findFormat, isOperationReady } from "../format-registry.js";
import {
    CliOptionError,
    readSharedOptions,
    readString,
    unsupportedFlags,
    type SharedCliOptions,
} from "../options.js";
import { createReport, type PanduckReport } from "../report.js";
import { isStdoutPath } from "../paths.js";
import { configureConsoleBackend } from "../console-backend.js";
import {
    hasPipeline,
    runConversionPipeline,
    runMarkdownProjectPipeline,
} from "../pipeline.js";

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
        .option("--output-dir <dir>", "cli.opt.output-dir")
        .option("--report <file>", "cli.opt.report")
        .option("--overwrite", "cli.opt.overwrite")
        .option("--budget <limit>", "cli.opt.budget")
        .option("--dry-run", "cli.opt.dry-run")
        .action((options: ParsedOptions) => runConvert(options));
}

async function emitConvertReport(
    report: PanduckReport,
    shared: SharedCliOptions,
    reportPath: string | undefined,
): Promise<void> {
    if (reportPath === "-") {
        emitJson(report);
        return;
    }
    if (reportPath) {
        await writeReportFile(report, reportPath);
    }
    emitReport(report, shared.diagnostics, shared.json);
}

async function runConvert(options: ParsedOptions): Promise<number> {
    try {
        const shared = readSharedOptions(options);
        const unsupported = unsupportedFlags(options);
        if (unsupported.length > 0) {
            console.error(`these flags are not supported in v0: ${unsupported.join(", ")}`);
            return ExitCode.InvalidArgs;
        }
        const inputPath = options._[0] ?? readString(options, "input");
        const outputOption = readString(options, "output");
        const outputDirOption = readString(options, "output-dir");
        const reportPath = readString(options, "report");
        const dryRun = options["dry-run"] === true;
        const markdownProjectTarget = shared.to === "markdown-project";

        if (!inputPath) {
            return ExitCode.InvalidArgs;
        }
        if (options._.length > 1) {
            console.error("convert accepts one input; batch conversion is not implemented");
            return ExitCode.InvalidArgs;
        }
        if (outputDirOption && outputOption) {
            console.error("use either -o or --output-dir, not both");
            return ExitCode.InvalidArgs;
        }
        if (outputDirOption && !markdownProjectTarget) {
            console.error("--output-dir requires --to markdown-project; use -o for a single output file");
            return ExitCode.InvalidArgs;
        }
        const outputPath = outputOption ?? outputDirOption;

        const overlapCode = validateConversionPaths(inputPath, outputPath, reportPath);
        if (overlapCode !== null) {
            console.error("input, output and report paths must not overlap");
            return overlapCode;
        }

        if (isStdoutPath(outputPath) && !shared.to) {
            console.error("writing to stdout requires an explicit --to format");
            return ExitCode.InvalidArgs;
        }

        if (isStdoutPath(outputPath) && reportPath === "-") {
            console.error("stdout cannot carry both the document and the report");
            return ExitCode.InvalidArgs;
        }

        if (isStdoutPath(outputPath) && (shared.diagnostics === "json" || shared.json)) {
            console.error(
                "stdout cannot carry both the document and the JSON report; write the document to a file or use --report -",
            );
            return ExitCode.InvalidArgs;
        }

        if (markdownProjectTarget && (!outputPath || isStdoutPath(outputPath))) {
            console.error("markdown-project requires an output directory via -o or --output-dir");
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
            await emitConvertReport(report, shared, reportPath);
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
            await emitConvertReport(report, shared, reportPath);
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
            await emitConvertReport(report, shared, reportPath);
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
                await emitConvertReport(report, shared, reportPath);
                return ExitCode.OutputFailure;
            } catch {
                // target does not exist yet
            }
        }

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
            await emitConvertReport(report, shared, reportPath);
            return ExitCode.InputUnsupported;
        }

        const routeReady = markdownProjectTarget
            ? Boolean(
                  ctx.bindings?.supportsMarkdownProject?.(resolved.from ?? "") &&
                      ctx.bindings?.convertMarkdownProject,
              )
            : hasPipeline(ctx.bindings, resolved.from, resolved.to);

        if (resolved.from === "doc" && !markdownProjectTarget) {
            const report = buildBlockedReport(
                ctx,
                "convert",
                inputPath,
                resolved,
                "legacy .doc import requires OLE reader support; convert to .docx first",
                ExitCode.InputUnsupported,
                shared,
            );
            await emitConvertReport(report, shared, reportPath);
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
                await emitConvertReport(report, shared, reportPath);
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
                await emitConvertReport(report, shared, reportPath);
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
            await emitConvertReport(report, shared, reportPath);
            return ExitCode.Success;
        }

        if (routeReady && ctx.bindings) {
            try {
                const result = markdownProjectTarget
                    ? await runMarkdownProjectPipeline({
                          ctx,
                          bindings: ctx.bindings,
                          inputPath,
                          outputDir: outputPath!,
                          resolved,
                          shared,
                          toolVersion: ctx.toolVersion,
                      })
                    : await runConversionPipeline({
                          ctx,
                          bindings: ctx.bindings,
                          inputPath,
                          outputPath,
                          resolved,
                          shared,
                          toolVersion: ctx.toolVersion,
                      });
                await emitConvertReport(result.report, shared, reportPath);
                return result.exitCode;
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
                await emitConvertReport(report, shared, reportPath);
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
                createDiagnostic({
                    code: "panduck.convert.not_implemented",
                    component: "convert",
                    severity: "error",
                    fallback: "conversion pipeline is not wired yet",
                }),
            ],
            outputs: outputPath ? [{ path: outputPath, published: false }] : [],
            budgets: shared.budgets,
            determinism: { profile: shared.profile ?? "default", config: shared.config ?? null },
            statusPolicy: { loss: shared.loss, strict: shared.strict, exit_code: ExitCode.IrOrWriterFailure },
        });

        await emitConvertReport(report, shared, reportPath);
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
