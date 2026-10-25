/**
 * Panduck Converter — TS authoring surface for format conversion.
 *
 * Contract: `Converter.toMarkdown(input, plan: ConversionPlan)`.
 *
 * Input can be any of three projections:
 *   - Page (from Pandark Crawler — has html()/bytes())
 *   - FetchArtifact (from Pandark Crawler.fetch())
 *   - string | Uint8Array
 *
 * All three enter the Notedown Rust lowering directly. Panduck does NOT ship
 * its own HTML parser, DOM executor, or Markdown writer — Rust owns the
 * lowering.
 *
 * Converter produces a structured DiagnosticRecord[] that the Pandark Crawler
 * can aggregate into the run report without parsing free-form message text.
 */

import { resolve } from "node:path";

import { loadPanduckNode } from "./node/load.js";
import type { ConversionPlan } from "./conversion-plan.js";
import { serializeConversionPlan } from "./conversion-plan.js";
import {
    createDiagnostic,
    parseReportEnvelope,
    textArg,
    u64Arg,
    withUrl,
} from "./diagnostic.js";
import type { DiagnosticRecord, LossRecord } from "./diagnostic.js";

export type { DiagnosticRecord, DiagnosticSeverity, LossRecord } from "./diagnostic.js";

// ---------------------------------------------------------------------------
// Input shapes
// ---------------------------------------------------------------------------

/** Anything Converter accepts as source material. */
export type ConverterInput =
    | ConverterPageLike
    | ConverterArtifactLike
    | string
    | Uint8Array;

/** Minimal shape Pandark Page must satisfy. */
export type ConverterPageLike = {
    readonly url: string;
    html(): Promise<string>;
    bytes(): Uint8Array;
};

/** Minimal shape Pandark FetchArtifact must satisfy. */
export type ConverterArtifactLike = {
    readonly finalUrl?: string;
    readonly bodyText?: string;
    readonly bodyBytes: Uint8Array;
};

// ---------------------------------------------------------------------------
// Result
// ---------------------------------------------------------------------------

/**
 * Status of a conversion.
 *
 * `success_with_loss` is **not** a shortcut for "everything went fine" — it
 * means the converter finished but produced structural loss records that the
 * caller should inspect. Only `success` means no required structures were
 * missing and no known loss was detected.
 */
export type ConverterStatus = "success" | "success_with_loss" | "failed";

/** Result of a Panduck conversion — consumed directly by Pandark Crawler. */
export type ConverterResult = {
    /** Conversion status. */
    status: ConverterStatus;
    /** Primary converted text output — always present even on partial failure. */
    text: string;
    /** Unified diagnostic array covering input, structure, conversion, loss, and rules. */
    diagnostics: DiagnosticRecord[];
    /** Raw report JSON from Panduck — preserved for auditing. */
    reportJson: string;
    /** Published output directory — present when `publish.kind === "markdown-project"`. */
    publishPath?: string;
};

// ---------------------------------------------------------------------------
// Converter
// ---------------------------------------------------------------------------

/** Public façade over Panduck's Rust conversion pipeline, driven by `ConversionPlan`. */
export class Converter {
    private binding: ReturnType<typeof loadPanduckNode>;

    constructor() {
        this.binding = loadPanduckNode();
    }

    // -----------------------------------------------------------------------
    // Single entry — ConversionPlan
    // -----------------------------------------------------------------------

    /**
     * Convert input according to a `ConversionPlan` from `defineConversion`.
     *
     * The plan owns input routing, IR transforms and publish strategy. There is
     * no option-based fallback: callers must construct a plan.
     */
    async toMarkdown(input: ConverterInput, plan: ConversionPlan): Promise<ConverterResult> {
        return this.runConversionPlan(input, plan);
    }

    // -----------------------------------------------------------------------
    // ConversionPlan path — 面向 IR 的产品级转换编排
    // -----------------------------------------------------------------------

    /**
     * ConversionPlan 入口。
     *
     * 关键差异：
     *   - **不跑 HTML DOM executor**。没有 CSS selector / DOM mutation / re-serialize。
     *     scope 选择（如果有）由 Pandark ExtractionProfile 在进入 Panduck 之前做掉。
     *   - **transforms 真实执行**。Rust 侧 `panduck-convert::transform` 解析
     *     plan descriptor 里的 transforms，在 `DocumentGraph` 上跑内置 pass
     *     （`replace-text` / `remove-blocks`），结果写入 report 的
     *     `pipeline.transforms`。未知 pass 名只在诊断里报告，不中断转换。
     *   - **publish 真实执行**。`publish.kind === "markdown-project"` 时走
     *     `convertMarkdownProjectBytes`，把 index / chapters / assets materialize
     *     到 `publish.directory`，返回的 `publishPath` 指向实际输出目录。
     *
     * ConversionPlan 是完整的产品级转换合同，覆盖 input → transforms → output → publish。
     */
    private async runConversionPlan(
        input: ConverterInput,
        plan: ConversionPlan,
    ): Promise<ConverterResult> {
        const from = plan.input.format;
        const { bytes, label, urlHint } = await materializeInput(input, plan.input.label);

        const diagnostics: DiagnosticRecord[] = [];

        if (!isExecutablePublishKind(plan.publish.kind)) {
            const publishKind = String(plan.publish.kind);
            diagnostics.push(createDiagnostic({
                code: "panduck.convert.unsupported-publish-kind",
                fallback: `ConversionPlan "${plan.id}" declares unsupported publish.kind ${JSON.stringify(publishKind)} — v0 executes only "none" and "markdown-project"`,
                messageKey: "panduck.diagnostic.unsupportedPublishKind",
                component: "convert",
                severity: "error",
                stage: "output",
                args: withUrl([
                    ["planId", textArg(plan.id)],
                    ["publishKind", textArg(publishKind)],
                ], urlHint),
            }));
            return makeResult("failed", "", diagnostics, label, urlHint);
        }

        if (bytes.length === 0) {
            diagnostics.push(createDiagnostic({
                code: "panduck.convert.empty-input",
                fallback: `Empty input for ${label}`,
                messageKey: "panduck.diagnostic.emptyInput",
                component: "convert",
                severity: "error",
                stage: "input",
                args: urlHint ? [["url", textArg(urlHint)]] : [],
            }));
            return makeResult("failed", "", diagnostics, label, urlHint);
        }

        // (1) 序列化 ConversionPlan 的 transforms → Rust plan descriptor。
        const handlersJson = serializeConversionPlan(plan);
        diagnostics.push(createDiagnostic({
            code: "panduck.convert.conversion-plan-active",
            fallback: `ConversionPlan "${plan.id}": ${from} → ${plan.output.format} (${plan.transforms.length} transform${plan.transforms.length === 1 ? "" : "s"}, publish: ${plan.publish.kind})`,
            component: "convert",
            severity: "info",
            stage: "convert",
            args: withUrl([
                ["planId", textArg(plan.id)],
                ["fromFormat", textArg(from)],
                ["toFormat", textArg(plan.output.format)],
                ["transformCount", u64Arg(plan.transforms.length)],
                ["publishKind", textArg(plan.publish.kind)],
            ], urlHint),
        }));

        reportNonExecutableHandlers(diagnostics, plan, urlHint);

        // (2) 确定 requireNoLoss（由 plan.output.lossPolicy 决定）。
        const requireNoLoss = plan.output.lossPolicy === "deny";

        if (plan.publish.kind === "markdown-project") {
            return this.publishMarkdownProject({
                from,
                bytes,
                label,
                urlHint: plan.input.sourceUrl ?? urlHint,
                diagnostics,
                handlersJson,
                requireNoLoss,
                directory: plan.publish.directory,
            });
        }

        return this.runRustConversion({
            from,
            bytes,
            label,
            urlHint: plan.input.sourceUrl ?? urlHint,
            diagnostics,
            handlersJson,
            requireNoLoss,
        });
    }

    // -----------------------------------------------------------------------
    // Shared — Rust Markdown project publish
    // -----------------------------------------------------------------------

    /**
     * Materialize a Markdown project through `convertMarkdownProjectBytes`.
     *
     * The Rust pipeline applies `plan.transforms` before the project writer, so
     * index/chapter markdown and materialized assets reflect the transform
     * result. `publishPath` is the resolved output directory reported by Rust.
     */
    private publishMarkdownProject(ctx: {
        from: string;
        bytes: Uint8Array;
        label: string;
        urlHint?: string;
        diagnostics: DiagnosticRecord[];
        handlersJson?: string;
        requireNoLoss: boolean;
        directory: string;
    }): Promise<ConverterResult> {
        const { from, bytes, label, urlHint, diagnostics, handlersJson, requireNoLoss, directory } = ctx;
        const fn = this.binding.convertMarkdownProjectBytes;
        if (!fn) {
            diagnostics.push(createDiagnostic({
                code: "panduck.convert.binding-missing",
                fallback: "Panduck binding does not expose convertMarkdownProjectBytes — rebuild @notedge/panduck-native",
                messageKey: "panduck.diagnostic.bindingMissing",
                component: "convert",
                severity: "error",
                stage: "convert",
                args: withUrl([], urlHint),
            }));
            return Promise.resolve(makeResult("failed", "", diagnostics, label, urlHint));
        }

        const outputDir = resolve(directory);
        let response: ReturnType<typeof fn> | undefined = undefined;
        let nativePanic: unknown = undefined;
        try {
            response = fn(from, label, bytes, outputDir, handlersJson);
        } catch (err) {
            nativePanic = err;
        }

        if (nativePanic !== undefined || !response) {
            const msg = nativePanic instanceof Error ? nativePanic.message : String(nativePanic);
            diagnostics.push(createDiagnostic({
                code: "panduck.convert.native-panic",
                fallback: `Panduck native binary panicked while publishing ${label}: ${msg}`,
                component: "convert",
                severity: "error",
                stage: "output",
                args: withUrl([["message", textArg(msg)]], urlHint),
            }));
            return Promise.resolve(makeResult("failed", "", diagnostics, label, urlHint));
        }

        const parsedReport = parseReportEnvelope(response.reportJson, { component: "convert", url: urlHint });
        diagnostics.push(...parsedReport.diagnostics);
        reportRustLoss(diagnostics, parsedReport.losses, label, urlHint);

        const status = deriveStatus(diagnostics, {
            lossCount: parsedReport.losses.length,
            exitCode: response.exitCode,
            requireNoLoss,
        });

        return Promise.resolve({
            status,
            text: response.indexMarkdown,
            diagnostics,
            reportJson: response.reportJson,
            publishPath: response.outputDir || outputDir,
        });
    }

    // -----------------------------------------------------------------------
    // Shared — Rust conversion
    // -----------------------------------------------------------------------

    private runRustConversion(ctx: {
        from: string;
        bytes: Uint8Array;
        label: string;
        urlHint?: string;
        diagnostics: DiagnosticRecord[];
        handlersJson?: string;
        requireNoLoss: boolean;
    }): Promise<ConverterResult> {
        const { from, bytes, label, urlHint, diagnostics, handlersJson, requireNoLoss } = ctx;
        const fn = this.binding.convertBytesMemory;
        if (!fn) {
            diagnostics.push(createDiagnostic({
                code: "panduck.convert.binding-missing",
                fallback: "Panduck binding does not expose convertBytesMemory — rebuild @notedge/panduck-native",
                messageKey: "panduck.diagnostic.bindingMissing",
                component: "convert",
                severity: "error",
                stage: "convert",
                args: withUrl([], urlHint),
            }));
            return Promise.resolve(makeResult("failed", "", diagnostics, label, urlHint));
        }

        let response: { exitCode: number; text?: string; reportJson: string } | undefined = undefined;
        let nativePanic: unknown = undefined;
        try {
            response = fn(from, "markdown", label, bytes, handlersJson);
        } catch (err) {
            nativePanic = err;
        }

        if (nativePanic !== undefined || !response) {
            const msg = nativePanic instanceof Error ? nativePanic.message : String(nativePanic);
            diagnostics.push(createDiagnostic({
                code: "panduck.convert.native-panic",
                fallback: `Panduck native binary panicked for ${label}: ${msg}`,
                component: "convert",
                severity: "error",
                stage: "convert",
                args: withUrl([["message", textArg(msg)]], urlHint),
            }));
            return Promise.resolve(makeResult("failed", "", diagnostics, label, urlHint));
        }

        const parsedReport = parseReportEnvelope(response.reportJson, { component: "convert", url: urlHint });
        diagnostics.push(...parsedReport.diagnostics);
        reportRustLoss(diagnostics, parsedReport.losses, label, urlHint);

        const text = response.text ?? "";
        const status = deriveStatus(diagnostics, {
            lossCount: parsedReport.losses.length,
            exitCode: response.exitCode,
            requireNoLoss,
        });

        return Promise.resolve({ status, text, diagnostics, reportJson: response.reportJson });
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** v0 only executes these two publish strategies — anything else is an author error. */
function isExecutablePublishKind(kind: string): kind is "none" | "markdown-project" {
    return kind === "none" || kind === "markdown-project";
}

/**
 * Report declared in-process handlers instead of silently dropping them.
 *
 * `TransformPass.pass` cannot cross the N-API wire and is never executed. If a
 * caller still supplies one it is a no-op, so surface an explicit warning
 * rather than pretending the handler ran.
 */
function reportNonExecutableHandlers(
    diagnostics: DiagnosticRecord[],
    plan: ConversionPlan,
    urlHint?: string,
): void {
    for (const transform of plan.transforms) {
        if (transform.pass === undefined) continue;
        diagnostics.push(createDiagnostic({
            code: "panduck.transform.non-executable-handler",
            fallback: `Transform "${transform.name}" declares a \`pass\` handler that cannot cross the N-API boundary and was not executed`,
            messageKey: "panduck.diagnostic.nonExecutableHandler",
            component: "transform",
            severity: "warning",
            stage: "transform",
            args: withUrl([["passName", textArg(transform.name)]], urlHint),
        }));
    }
}

async function materializeInput(
    input: ConverterInput,
    explicitLabel?: string,
): Promise<{ bytes: Uint8Array; label: string; urlHint?: string }> {
    if (typeof input === "string") {
        return { bytes: new TextEncoder().encode(input), label: explicitLabel ?? "<string>" };
    }
    if (input instanceof Uint8Array) {
        return { bytes: input, label: explicitLabel ?? "<bytes>" };
    }
    if (typeof input === "object" && input !== null) {
        const candidate = input as Record<string, unknown>;
        if (typeof candidate.bytes === "function") {
            const page = input as ConverterPageLike;
            let bytes = page.bytes();
            if (!bytes || bytes.length === 0) {
                bytes = new TextEncoder().encode(await page.html());
            }
            return { bytes, label: explicitLabel ?? (page.url ?? "<page>"), urlHint: page.url };
        }
        if ("bodyBytes" in candidate) {
            const artifact = input as ConverterArtifactLike;
            return {
                bytes: artifact.bodyBytes,
                label: explicitLabel ?? (artifact.finalUrl ?? "<artifact>"),
                urlHint: artifact.finalUrl,
            };
        }
    }
    return { bytes: new Uint8Array(), label: explicitLabel ?? "<unknown>" };
}

/** Report one warning when the Rust report carries real IR loss markers. */
function reportRustLoss(
    diagnostics: DiagnosticRecord[],
    losses: readonly LossRecord[],
    label: string,
    urlHint?: string,
): void {
    if (losses.length === 0) return;
    diagnostics.push(createDiagnostic({
        code: "panduck.convert.rust-loss",
        fallback: `Rust reported ${losses.length} known structural loss(es) for ${label}`,
        component: "convert",
        severity: "warning",
        stage: "loss",
        args: withUrl([["count", u64Arg(losses.length)]], urlHint),
    }));
}

function makeResult(
    status: ConverterStatus,
    text: string,
    diags: DiagnosticRecord[],
    label: string,
    urlHint?: string,
): ConverterResult {
    const reportStatus = status === "failed" ? "blocked" : status;
    const reportJson = JSON.stringify({
        schema_version: "panduck.report/v0",
        operation: "convert",
        status: reportStatus,
        inputs: [{ path: label, format: "html" }],
        pipeline: {
            reader: "html",
            writer: "markdown",
            stages: ["read", "ir", "write"],
            ...(reportStatus === "blocked" ? { blocked_reason: "conversion failed" } : {}),
        },
        diagnostics: {
            schemaVersion: 1,
            diagnostics: diags,
            truncated: false,
            dropped: 0,
        },
        losses: [],
        outputs: [],
    });
    return { status, text, diagnostics: diags, reportJson };
}

/**
 * Derive ConverterStatus from real run data only:
 *   1. exitCode from Rust → non-zero means hard failure.
 *   2. Error-severity diagnostics → failed.
 *   3. `requireNoLoss` (ConversionPlan deny) → any IR loss is failed.
 *   4. Real IR loss markers → success_with_loss.
 *   5. Otherwise → success.
 *
 * Diagnostic code names are never inspected to guess loss.
 */
function deriveStatus(
    diags: DiagnosticRecord[],
    ctx: { lossCount: number; exitCode: number; requireNoLoss: boolean },
): ConverterStatus {
    if (ctx.exitCode !== 0) return "failed";
    if (diags.some((d) => d.severity === "error")) return "failed";
    if (ctx.lossCount > 0) {
        if (ctx.requireNoLoss) return "failed";
        return "success_with_loss";
    }
    return "success";
}
