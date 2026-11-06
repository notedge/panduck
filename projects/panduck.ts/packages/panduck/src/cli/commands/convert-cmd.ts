import { access, mkdir } from "node:fs/promises";
import { constants } from "node:fs";
import { basename, extname, join } from "node:path";

import type { Cli, ParsedOptions } from "@vmz/commander";

import {
    assertReadableInput,
    buildBlockedReport,
    buildUnlistedRouteReport,
    createContext,
    inputReadFailure,
    resolveFormats,
    validateConversionPaths,
} from "../context.js";
import { createDiagnostic } from "../../diagnostic.js";
import { PANDUCK_DIAGNOSTIC_CODES } from "../../diagnostic-codes.js";
import { emitJson, emitReport, writeReportFile } from "../diagnostics.js";
import { ExitCode } from "../exit-codes.js";
import {
    findFormat,
    isOperationReady,
    routeCoverage,
    routeListed,
    type CapabilityState,
} from "../format-registry.js";
import {
    CliOptionError,
    describeUnsupportedFlags,
    readSharedOptions,
    readString,
    type SharedCliOptions,
} from "../options.js";
import { createReport, type PanduckReport } from "../report.js";
import {
    isStdinPath,
    isStdoutPath,
} from "../paths.js";
import { configureConsoleBackend } from "../console-backend.js";
import {
    hasPipeline,
    runConversionPipeline,
    runMarkdownProjectPipeline,
} from "../pipeline.js";

/** Target format to the extension used when a batch derives one file per input. */
const TARGET_EXTENSIONS: Record<string, string> = {
    markdown: "md",
    notedown: "notedown",
    org: "org",
    rst: "rst",
    tex: "tex",
    html: "html",
    docx: "docx",
    epub: "epub",
    pdf: "pdf",
    doc: "doc",
};

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
        const unsupported = describeUnsupportedFlags(options);
        if (unsupported.length > 0) {
            for (const line of unsupported) {
                console.error(line);
            }
            return ExitCode.InvalidArgs;
        }
        const inputs: string[] = Array.isArray(options._) ? options._ : [];
        const inputPath = inputs[0] ?? readString(options, "input");
        const outputOption = readString(options, "output");
        const outputDirOption = readString(options, "output-dir");
        const reportPath = readString(options, "report");
        const dryRun = options["dry-run"] === true;
        const markdownProjectTarget = shared.to === "markdown-project";

        if (!inputPath) {
            return ExitCode.InvalidArgs;
        }
        if (outputDirOption && outputOption) {
            console.error("use either -o or --output-dir, not both");
            return ExitCode.InvalidArgs;
        }
        if (inputs.length > 1) {
            if (!outputDirOption) {
                console.error("multiple inputs require --output-dir");
                return ExitCode.InvalidArgs;
            }
            if (inputs.some((path) => isStdinPath(path))) {
                console.error("batch conversion cannot read standard input; pass real file paths");
                return ExitCode.InvalidArgs;
            }
            if (markdownProjectTarget) {
                const colliding = inputs
                    .map((path) => basename(path, extname(path)))
                    .filter((stem, index, stems) => stems.indexOf(stem) !== index);
                if (colliding.length > 0) {
                    console.error(
                        `two batch inputs share one markdown-project output directory: ${colliding.join(", ")}`,
                    );
                    return ExitCode.InvalidArgs;
                }
            }
        } else if (outputDirOption && !markdownProjectTarget) {
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

        if (inputs.length > 1) {
            const batch = await runBatchConvert({
                ctx,
                inputs,
                outputDir: outputDirOption!,
                shared,
                dryRun,
            });
            await emitConvertReport(batch.report, shared, reportPath);
            return batch.exitCode;
        }

        const result = await convertSingleFile({
            ctx,
            inputPath,
            outputPath,
            shared,
            dryRun,
        });
        await emitConvertReport(result.report, shared, reportPath);
        return result.exitCode;
    } catch (error) {
        if (error instanceof CliOptionError) {
            console.error(error.message);
            return error.exitCode;
        }
        console.error(error instanceof Error ? error.message : String(error));
        return ExitCode.InvalidArgs;
    }
}

type FileResult = {
    exitCode: number;
    report: PanduckReport;
};

/** Converts one input with the already-resolved CLI context. */
async function convertSingleFile(input: {
    ctx: ReturnType<typeof createContext>;
    inputPath: string;
    outputPath: string | undefined;
    shared: SharedCliOptions;
    dryRun: boolean;
}): Promise<FileResult> {
    const { ctx, inputPath, outputPath, shared, dryRun } = input;
    const resolved = resolveFormats(inputPath, outputPath, shared);
    const markdownProjectTarget = shared.to === "markdown-project";

    try {
        if (resolved.ambiguous) {
            return {
                exitCode: ExitCode.InputUnsupported,
                report: buildBlockedReport(
                    ctx,
                    "convert",
                    inputPath,
                    resolved,
                    "ambiguous input format",
                    ExitCode.InputUnsupported,
                    shared,
                ),
            };
        }

        if (!resolved.to) {
            return {
                exitCode: ExitCode.InvalidArgs,
                report: buildBlockedReport(
                    ctx,
                    "convert",
                    inputPath,
                    resolved,
                    "target format is required",
                    ExitCode.InvalidArgs,
                    shared,
                ),
            };
        }

        try {
            await assertReadableInput(inputPath);
        } catch (error) {
            const failure = inputReadFailure(inputPath, error);
            return {
                exitCode: failure.exitCode,
                report: buildBlockedReport(
                    ctx,
                    "convert",
                    inputPath,
                    resolved,
                    failure.reason,
                    failure.exitCode,
                    shared,
                    [failure.diagnostic],
                ),
            };
        }

        if (outputPath && outputPath !== "-" && !shared.overwrite && !markdownProjectTarget) {
            try {
                await access(outputPath, constants.F_OK);
                return {
                    exitCode: ExitCode.OutputFailure,
                    report: buildBlockedReport(
                        ctx,
                        "convert",
                        inputPath,
                        resolved,
                        "output already exists",
                        ExitCode.OutputFailure,
                        shared,
                    ),
                };
            } catch {
                // target does not exist yet
            }
        }

        const source = resolved.from ? findFormat(ctx.registry, resolved.from) : undefined;
        const target = findFormat(ctx.registry, resolved.to);
        if (!target) {
            return {
                exitCode: ExitCode.InputUnsupported,
                report: buildBlockedReport(
                    ctx,
                    "convert",
                    inputPath,
                    resolved,
                    `unknown target format ${resolved.to}`,
                    ExitCode.InputUnsupported,
                    shared,
                ),
            };
        }

        const routeReady = markdownProjectTarget
            ? Boolean(
                  ctx.bindings?.supportsMarkdownProject?.(resolved.from ?? "") &&
                      ctx.bindings?.exportMarkdownProject,
              )
            : hasPipeline(ctx.bindings, resolved.from, resolved.to);

        if (!routeListed(ctx.registry, resolved.from, resolved.to)) {
            return {
                exitCode: ExitCode.InputUnsupported,
                report: buildUnlistedRouteReport(ctx, "convert", inputPath, resolved, shared),
            };
        }

        if (!routeReady) {
            if (source && !isOperationReady(source, "read")) {
                return {
                    exitCode: ExitCode.InputUnsupported,
                    report: buildBlockedReport(
                        ctx,
                        "convert",
                        inputPath,
                        resolved,
                        `source format ${source.format} is not ready for read`,
                        ExitCode.InputUnsupported,
                        shared,
                    ),
                };
            }

            if (!isOperationReady(target, "write")) {
                return {
                    exitCode: ExitCode.InputUnsupported,
                    report: buildBlockedReport(
                        ctx,
                        "convert",
                        inputPath,
                        resolved,
                        `target format ${target.format} is not ready for write`,
                        ExitCode.InputUnsupported,
                        shared,
                    ),
                };
            }
        }

        if (dryRun) {
            return {
                exitCode: ExitCode.Success,
                report: createReport({
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
                    coverage: routeCoverage(ctx.registry, resolved.from, resolved.to),
                    outputs: outputPath ? [{ path: outputPath, published: false }] : [],
                    budgets: shared.budgets,
                    determinism: { profile: shared.profile ?? "default", config: shared.config ?? null },
                    statusPolicy: { loss: shared.loss, strict: shared.strict, exit_code: ExitCode.Success },
                }),
            };
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
                return { exitCode: result.exitCode, report: result.report };
            } catch (error) {
                const message = error instanceof Error ? error.message : String(error);
                return {
                    exitCode: ExitCode.IrOrWriterFailure,
                    report: buildBlockedReport(
                        ctx,
                        "convert",
                        inputPath,
                        resolved,
                        message,
                        ExitCode.IrOrWriterFailure,
                        shared,
                    ),
                };
            }
        }

        return {
            exitCode: ExitCode.IrOrWriterFailure,
            report: createReport({
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
                        code: PANDUCK_DIAGNOSTIC_CODES.notImplemented,
                        component: "convert",
                        severity: "error",
                        fallback: "conversion pipeline is not wired yet",
                    }),
                ],
                outputs: outputPath ? [{ path: outputPath, published: false }] : [],
                budgets: shared.budgets,
                determinism: { profile: shared.profile ?? "default", config: shared.config ?? null },
                statusPolicy: { loss: shared.loss, strict: shared.strict, exit_code: ExitCode.IrOrWriterFailure },
            }),
        };
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        return {
            exitCode: ExitCode.IrOrWriterFailure,
            report: buildBlockedReport(
                ctx,
                "convert",
                inputPath,
                resolved,
                message,
                ExitCode.IrOrWriterFailure,
                shared,
            ),
        };
    }
}

/** One aggregated report over every input, in the order the inputs were given. */
async function runBatchConvert(input: {
    ctx: ReturnType<typeof createContext>;
    inputs: string[];
    outputDir: string;
    shared: SharedCliOptions;
    dryRun: boolean;
}): Promise<FileResult> {
    const { ctx, inputs, outputDir, shared, dryRun } = input;
    const to = shared.to;
    const markdownProjectTarget = to === "markdown-project";
    if (!to) {
        const reason = "batch conversion requires a target format via --to";
        console.error(reason);
        return {
            exitCode: ExitCode.InvalidArgs,
            report: createReport({
                operation: "convert-batch",
                toolVersion: ctx.toolVersion,
                status: "blocked",
                inputs: inputs.map((path) => ({ path })),
                pipeline: { blocked_reason: reason },
                diagnostics: [
                    createDiagnostic({
                        code: PANDUCK_DIAGNOSTIC_CODES.cliBlocked,
                        component: "cli",
                        severity: "error",
                        fallback: reason,
                    }),
                ],
                budgets: shared.budgets,
                determinism: { profile: shared.profile ?? "default", config: shared.config ?? null },
                statusPolicy: { loss: shared.loss, strict: shared.strict, exit_code: ExitCode.InvalidArgs },
            }),
        };
    }

    // Single-file targets derive one file per input. A markdown project owns a
    // directory, so every input gets its own subdirectory under `--output-dir`.
    const extension = TARGET_EXTENSIONS[to] ?? to;
    const plans = inputs.map((inputPath) => {
        const stem = basename(inputPath, extname(inputPath));
        return {
            inputPath,
            outputPath: markdownProjectTarget ? join(outputDir, stem) : join(outputDir, `${stem}.${extension}`),
        };
    });

    const seen = new Set<string>();
    for (const plan of plans) {
        if (seen.has(plan.outputPath)) {
            console.error(`two batch inputs map to the same output: ${plan.outputPath}`);
            return {
                exitCode: ExitCode.InvalidArgs,
                report: createReport({
                    operation: "convert-batch",
                    toolVersion: ctx.toolVersion,
                    status: "blocked",
                    inputs: inputs.map((path) => ({ path })),
                    pipeline: { blocked_reason: `two batch inputs map to the same output: ${plan.outputPath}` },
                    diagnostics: [
                        createDiagnostic({
                            code: PANDUCK_DIAGNOSTIC_CODES.cliBlocked,
                            component: "cli",
                            severity: "error",
                            fallback: `two batch inputs map to the same output: ${plan.outputPath}`,
                        }),
                    ],
                    budgets: shared.budgets,
                    determinism: { profile: shared.profile ?? "default", config: shared.config ?? null },
                    statusPolicy: { loss: shared.loss, strict: shared.strict, exit_code: ExitCode.InvalidArgs },
                }),
            };
        }
        seen.add(plan.outputPath);
    }

    // Dry runs only plan: they report unpublished outputs and must not create the site root.
    if (!dryRun) {
        await mkdir(outputDir, { recursive: true });
    }

    let exitCode: number = ExitCode.Success;
    let blocked = false;
    let withLoss = false;
    const inputsReport: PanduckReport["inputs"] = [];
    const outputs: NonNullable<PanduckReport["outputs"]> = [];
    const losses: NonNullable<PanduckReport["losses"]> = [];
    const diagnostics: NonNullable<PanduckReport["diagnostics"]> = [];

    for (const plan of plans) {
        const result = await convertSingleFile({
            ctx,
            inputPath: plan.inputPath,
            outputPath: plan.outputPath,
            shared,
            dryRun,
        });
        exitCode = Math.max(exitCode, result.exitCode);
        blocked = blocked || result.report.status === "blocked";
        withLoss = withLoss || result.report.status === "success_with_loss";
        for (const item of result.report.inputs) {
            inputsReport.push(item);
        }
        for (const item of result.report.outputs ?? []) {
            outputs.push(item);
        }
        for (const item of result.report.losses ?? []) {
            losses.push(item);
        }
        for (const item of result.report.diagnostics ?? []) {
            diagnostics.push(item);
        }
    }

    const status: PanduckReport["status"] = blocked ? "blocked" : withLoss ? "success_with_loss" : "success";
    const routeStates = inputs.map((inputPath) => {
        const resolved = resolveFormats(inputPath, undefined, shared);
        return routeCoverage(ctx.registry, resolved.from, to);
    });
    const coverageState: CapabilityState = routeStates.every((route) => route.state === "ready")
        ? "ready"
        : routeStates.some((route) => route.state === "unavailable")
          ? "unavailable"
          : "partial";
    // Every input shares one target writer, so the declared coverage is the
    // same record `routeCoverage` reads for `plan`, `check` and single `convert`.
    const declared = routeStates.find((route) => route.state !== "unavailable");
    const coverage: PanduckReport["coverage"] =
        coverageState === "unavailable"
            ? { state: coverageState }
            : { state: coverageState, known: declared?.known, unknown: declared?.unknown };
    const report = createReport({
        operation: "convert-batch",
        toolVersion: ctx.toolVersion,
        status,
        inputs: inputsReport,
        pipeline: { writer: to, stages: ["read", "ir", "write", "publish"] },
        coverage,
        losses,
        diagnostics,
        outputs,
        budgets: shared.budgets,
        determinism: { profile: shared.profile ?? "default", config: shared.config ?? null },
        statusPolicy: { loss: shared.loss, strict: shared.strict, exit_code: exitCode },
    });

    return { exitCode, report };
}
