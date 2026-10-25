/**
 * Panduck Converter — TS authoring surface for format conversion.
 *
 * Contract: `Converter.toMarkdown(input, options?)`.
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
import type { ConverterConfig } from "./renderer-policy.js";
import type { ConversionPlan } from "./conversion-plan.js";
import { serializeConversionPlan } from "./conversion-plan.js";
import {
    createDiagnostic,
    parseReportEnvelope,
    textArg,
    textListArg,
    u64Arg,
    withUrl,
} from "./diagnostic.js";
import type { DiagnosticMessageArg, DiagnosticRecord, LossRecord } from "./diagnostic.js";

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
// Options
// ---------------------------------------------------------------------------

/**
 * Conversion options.
 *
 * Scope selection belongs to Pandark's ExtractionProfile before bytes reach
 * Panduck. The removed DOM rule chain (`select` / `remove` / `unwrap` /
 * `replace`) is not part of this surface.
 */
export type ToMarkdownOptions = {
    /** Format identifier passed to Panduck ("html", "docx", "epub", ...). Default "html". */
    from?: string;
    /** Label written into the report provenance when the input is anonymous. */
    label?: string;
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
export type ConverterStatus = "success" | "success_with_loss" | "success_unvalidated" | "failed";

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

/** Public façade over Panduck's Rust conversion pipeline and policy declaration surface. */
export class Converter {
    private binding: ReturnType<typeof loadPanduckNode>;

    constructor() {
        this.binding = loadPanduckNode();
    }

    // -----------------------------------------------------------------------
    // Overloads — 三个入口并存。
    // -----------------------------------------------------------------------

    /** Legacy entry: format + label options. */
    async toMarkdown(input: ConverterInput, options?: ToMarkdownOptions): Promise<ConverterResult>;
    /** New entry: Profile 化 ConverterConfig（含 RendererPolicy）。 */
    async toMarkdown(input: ConverterInput, config: ConverterConfig): Promise<ConverterResult>;
    /** Newest entry: ConversionPlan — 面向 IR 的产品级转换编排。 */
    async toMarkdown(input: ConverterInput, plan: ConversionPlan): Promise<ConverterResult>;

    // -----------------------------------------------------------------------
    // Implementation — 通过 arg shape 分流。
    // -----------------------------------------------------------------------

    async toMarkdown(
        input: ConverterInput,
        arg?: ToMarkdownOptions | ConverterConfig | ConversionPlan,
    ): Promise<ConverterResult> {
        if (arg && isConversionPlan(arg)) {
            return this.toMarkdownWithPlan(input, arg);
        }
        if (arg && isConverterConfig(arg)) {
            return this.toMarkdownWithPolicy(input, arg);
        }
        return this.toMarkdownLegacy(input, arg as ToMarkdownOptions | undefined);
    }

    // -----------------------------------------------------------------------
    // Legacy path — format + label
    // -----------------------------------------------------------------------

    /**
     * Legacy entry — format + label only.
     * Pipeline:
     *   1. Materialize input → bytes.
     *   2. Rust lowering + writer.
     *   3. Derive status from Rust report.
     */
    private async toMarkdownLegacy(
        input: ConverterInput,
        options?: ToMarkdownOptions,
    ): Promise<ConverterResult> {
        const from = options?.from ?? "html";
        const { bytes, label, urlHint } = await materializeInput(input, options?.label);

        const diagnostics: DiagnosticRecord[] = [];

        // Empty input short-circuit.
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

        return this.runRustConversion({
            from,
            bytes,
            label,
            urlHint,
            diagnostics,
            handlersJson: undefined,
            requireNoLoss: false,
        });
    }

    // -----------------------------------------------------------------------
    // Policy path — ConverterConfig + RendererPolicy
    // -----------------------------------------------------------------------

    /**
     * New entry — Profile 化 ConverterConfig。
     *
     * 关键差异：
     *   - 输入直接进入 Rust lowering，没有 TS 侧 DOM 解析、选择或重序列化。
     *     scope 选择（如果有）由 Pandark ExtractionProfile 在进入 Panduck 之前做掉；
     *     renderer 这边只读 Notedown IR。
     *   - `policy.handlers` 只是声明面。TS 侧不执行 handler，Rust lowering
     *     也不执行 handler —— 跨语言闭包无法通过 wire 传递。带 handlers 时
     *     产出一条 Warning 诊断，转换继续但不会应用 handler。
     */
    private async toMarkdownWithPolicy(
        input: ConverterInput,
        config: ConverterConfig,
    ): Promise<ConverterResult> {
        const from = "html"; // policy 入口只服务于 Pandark 场景，输入是 HTML page
        const policy = config.rendererPolicy;
        const { bytes, label, urlHint } = await materializeInput(input, undefined);

        const diagnostics: DiagnosticRecord[] = [];

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

        diagnostics.push(createDiagnostic({
            code: "panduck.convert.policy-active",
            fallback: `Using RendererPolicy "${policy.name}"`,
            component: "convert",
            severity: "info",
            stage: "convert",
            args: withUrl([["policyName", textArg(policy.name)]], urlHint),
        }));
        reportUnsupportedHandlers(
            diagnostics,
            policy.name,
            Object.keys(policy.handlers ?? {}),
            urlHint,
        );

        return this.runRustConversion({
            from,
            bytes,
            label,
            urlHint,
            diagnostics,
            requireNoLoss: false,
        });
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
     *   - **不执行 policy handler**。handler 是 TS 闭包，Rust converter 不执行，
     *     也无法通过 wire 传递。plan 携带 handlers 时产出一条 Warning 诊断，
     *     转换继续但不应用 handler。
     *   - **transforms 真实执行**。Rust 侧 `panduck-convert::transform` 解析
     *     plan descriptor 里的 transforms，在 `DocumentGraph` 上跑内置 pass
     *     （`replace-text` / `remove-blocks`），结果写入 report 的
     *     `pipeline.transforms`。未知 pass 名只在诊断里报告，不中断转换。
     *   - **publish 真实执行**。`publish.kind === "markdown-project"` 时走
     *     `convertMarkdownProjectBytes`，把 index / chapters / assets materialize
     *     到 `publish.directory`，返回的 `publishPath` 指向实际输出目录。
     *
     * 与 Policy 路径的对比：
     *   - Policy 只有 output.policy，没有 input.format / transforms / publish。
     *   - ConversionPlan 是完整的产品级转换合同，覆盖 input → transforms → output → publish。
     */
    private async toMarkdownWithPlan(
        input: ConverterInput,
        plan: ConversionPlan,
    ): Promise<ConverterResult> {
        const from = plan.input.format;
        const { bytes, label, urlHint } = await materializeInput(input, plan.input.label);

        const diagnostics: DiagnosticRecord[] = [];

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

        // (2) output 带 RendererPolicy 时，handlers 无法执行 —— 只报告，不应用。
        reportUnsupportedHandlers(
            diagnostics,
            plan.output.policy?.name,
            Object.keys(plan.output.policy?.handlers ?? {}),
            urlHint,
        );

        // (3) 确定 requireNoLoss（由 plan.output.lossPolicy 决定）。
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

/** Type guard: true if arg is a ConverterConfig (has `rendererPolicy`). */
function isConverterConfig(arg: object): arg is ConverterConfig {
    return "rendererPolicy" in arg && (arg as { rendererPolicy?: unknown }).rendererPolicy != null;
}

/** Type guard: true if arg is a ConversionPlan (has `version: "panduck-conversion-v0"`). */
function isConversionPlan(arg: object): arg is ConversionPlan {
    return "version" in arg
        && (arg as { version?: unknown }).version === "panduck-conversion-v0"
        && "input" in arg
        && "output" in arg;
}

/**
 * 报告 policy handlers 当前无法执行。
 *
 * handler 是 TS 闭包，跨语言 wire 无法传递；Rust converter 也不执行 handler。
 * 这里只产出一条 Warning，转换继续，但不会应用任何 handler。不要在 TS 侧
 * 用 sentinel 或其他方式伪造执行。
 */
function reportUnsupportedHandlers(
    diagnostics: DiagnosticRecord[],
    policyName: string | undefined,
    handlerKeys: readonly string[],
    urlHint?: string,
): void {
    if (handlerKeys.length === 0) return;

    const name = policyName ?? "<unnamed>";
    const keysDisplay = handlerKeys.map((key) => `"${key}"`).join(", ");
    const args: Array<[string, DiagnosticMessageArg]> = [];
    if (policyName) args.push(["policyName", textArg(policyName)]);
    args.push(["handlerKeys", textListArg(handlerKeys)]);
    diagnostics.push(createDiagnostic({
        code: "panduck.policy.handlers-unsupported",
        fallback:
            `RendererPolicy "${name}" declares ${handlerKeys.length} handler(s) [${keysDisplay}] — ` +
            "handler execution is not available in the TS layer and the Rust converter does not execute handlers. " +
            "Conversion continues without applying them.",
        component: "policy",
        severity: "warning",
        stage: "rule",
        args: withUrl(args, urlHint),
    }));
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
    const reportStatus = status === "failed" ? "failed" : status === "success_with_loss" ? "success_with_loss" : "success";
    const reportJson = JSON.stringify({
        schema_version: "panduck.report/v0",
        operation: "convert",
        status: reportStatus,
        inputs: [{ path: label, format: "html" }],
        pipeline: { reader: "html", writer: "markdown", stages: ["read", "ir", "write"] },
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
