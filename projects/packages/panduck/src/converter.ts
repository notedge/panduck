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

// ---------------------------------------------------------------------------
// Diagnostic record (shared shape with Pandark Crawler)
// ---------------------------------------------------------------------------

export type DiagnosticSeverity = "error" | "warn" | "info";

/**
 * Structured diagnostic record. Mirrors boost-diagnostic::Diagnostic but kept
 * as a plain TS interface so both Pandark and Panduck can agree on the
 * wire contract without a shared runtime crate.
 */
export type DiagnosticRecord = {
    /** Stable dotted code — e.g. `panduck.convert.required-missing`. */
    code: string;
    /** Fallback human-readable message. */
    fallback?: string;
    /** Message key for locale resolution. */
    messageKey?: string;
    /** Typed args — stable keys, free-form values. */
    args?: Record<string, unknown>;
    /** Origin identity. */
    origin?: {
        namespace: string;
        component: string;
    };
    severity?: DiagnosticSeverity;
    /** Which converter stage produced this record. */
    stage?: "input" | "structure" | "transform" | "convert" | "loss" | "rule" | "output";
    /** URL or label the diagnostic belongs to. */
    url?: string;
    /** Affected selector / source span / asset. */
    target?: string;
};

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
            diagnostics.push({
                code: "panduck.convert.empty-input",
                fallback: `Empty input for ${label}`,
                messageKey: "panduck.diagnostic.emptyInput",
                origin: { namespace: "panduck", component: "convert" },
                severity: "error",
                stage: "input",
                url: urlHint,
            });
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
            diagnostics.push({
                code: "panduck.convert.empty-input",
                fallback: `Empty input for ${label}`,
                messageKey: "panduck.diagnostic.emptyInput",
                origin: { namespace: "panduck", component: "convert" },
                severity: "error",
                stage: "input",
                url: urlHint,
            });
            return makeResult("failed", "", diagnostics, label, urlHint);
        }

        diagnostics.push({
            code: "panduck.convert.policy-active",
            fallback: `Using RendererPolicy "${policy.name}"`,
            args: { policyName: policy.name },
            origin: { namespace: "panduck", component: "convert" },
            severity: "info",
            stage: "convert",
            url: urlHint,
        });
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
            diagnostics.push({
                code: "panduck.convert.empty-input",
                fallback: `Empty input for ${label}`,
                messageKey: "panduck.diagnostic.emptyInput",
                origin: { namespace: "panduck", component: "convert" },
                severity: "error",
                stage: "input",
                url: urlHint,
            });
            return makeResult("failed", "", diagnostics, label, urlHint);
        }

        // (1) 序列化 ConversionPlan 的 transforms → Rust plan descriptor。
        const handlersJson = serializeConversionPlan(plan);
        diagnostics.push({
            code: "panduck.convert.conversion-plan-active",
            fallback: `ConversionPlan "${plan.id}": ${from} → ${plan.output.format} (${plan.transforms.length} transform${plan.transforms.length === 1 ? "" : "s"}, publish: ${plan.publish.kind})`,
            args: {
                planId: plan.id,
                fromFormat: from,
                toFormat: plan.output.format,
                transformCount: plan.transforms.length,
                publishKind: plan.publish.kind,
            },
            origin: { namespace: "panduck", component: "convert" },
            severity: "info",
            stage: "convert",
            url: urlHint,
        });

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
            diagnostics.push({
                code: "panduck.convert.binding-missing",
                fallback: "Panduck binding does not expose convertMarkdownProjectBytes — rebuild @notedge/panduck-native",
                messageKey: "panduck.diagnostic.bindingMissing",
                origin: { namespace: "panduck", component: "convert" },
                severity: "error",
                stage: "convert",
                url: urlHint,
            });
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
            diagnostics.push({
                code: "panduck.convert.native-panic",
                fallback: `Panduck native binary panicked while publishing ${label}: ${msg}`,
                args: { message: msg },
                origin: { namespace: "panduck", component: "convert" },
                severity: "error",
                stage: "output",
                url: urlHint,
            });
            return Promise.resolve(makeResult("failed", "", diagnostics, label, urlHint));
        }

        const parsedReport = safeParseReport(response.reportJson);
        diagnostics.push(...parsedReport.diagnostics);

        const hasRustLoss = parsedReport.losses.length > 0;
        if (hasRustLoss) {
            diagnostics.push({
                code: "panduck.convert.rust-loss",
                fallback: `Rust reported ${parsedReport.losses.length} known structural loss(es) for ${label}`,
                args: { count: parsedReport.losses.length },
                origin: { namespace: "panduck", component: "convert" },
                severity: "warn",
                stage: "loss",
                url: urlHint,
            });
        }

        const status = deriveStatus(diagnostics, {
            rustStatus: parsedReport.rawStatus,
            hasRustLoss,
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
            diagnostics.push({
                code: "panduck.convert.binding-missing",
                fallback: "Panduck binding does not expose convertBytesMemory — rebuild @notedge/panduck-native",
                messageKey: "panduck.diagnostic.bindingMissing",
                origin: { namespace: "panduck", component: "convert" },
                severity: "error",
                stage: "convert",
                url: urlHint,
            });
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
            diagnostics.push({
                code: "panduck.convert.native-panic",
                fallback: `Panduck native binary panicked for ${label}: ${msg}`,
                args: { message: msg },
                origin: { namespace: "panduck", component: "convert" },
                severity: "error",
                stage: "convert",
                url: urlHint,
            });
            return Promise.resolve(makeResult("failed", "", diagnostics, label, urlHint));
        }

        const parsedReport = safeParseReport(response.reportJson);
        diagnostics.push(...parsedReport.diagnostics);

        const rustStatus = parsedReport.rawStatus;
        const hasRustLoss = parsedReport.losses.length > 0;
        if (hasRustLoss) {
            diagnostics.push({
                code: "panduck.convert.rust-loss",
                fallback: `Rust reported ${parsedReport.losses.length} known structural loss(es) for ${label}`,
                args: { count: parsedReport.losses.length },
                origin: { namespace: "panduck", component: "convert" },
                severity: "warn",
                stage: "loss",
                url: urlHint,
            });
        }

        const text = response.text ?? "";
        const status = deriveStatus(diagnostics, {
            rustStatus,
            hasRustLoss,
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
    diagnostics.push({
        code: "panduck.policy.handlers-unsupported",
        fallback:
            `RendererPolicy "${name}" declares ${handlerKeys.length} handler(s) [${keysDisplay}] — ` +
            "handler execution is not available in the TS layer and the Rust converter does not execute handlers. " +
            "Conversion continues without applying them.",
        args: { policyName: policyName ?? null, handlerKeys: [...handlerKeys] },
        origin: { namespace: "panduck", component: "policy" },
        severity: "warn",
        stage: "rule",
        url: urlHint,
    });
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

function makeResult(
    status: ConverterStatus,
    text: string,
    diags: DiagnosticRecord[],
    label: string,
    urlHint?: string,
): ConverterResult {
    const reportStatus = status === "failed" ? "failed" : status === "success_with_loss" ? "partial" : "ok";
    const reportJson = JSON.stringify({
        schema_version: "panduck.report/v0",
        operation: "convert",
        status: reportStatus,
        inputs: [{ path: label, format: "html" }],
        pipeline: { reader: "html", writer: "markdown", stages: ["read", "ir", "write"] },
        diagnostics: {
            schema_version: "panduck.diagnostic/v0",
            diagnostics: diags.map(toRustDiagnosticShape),
            truncated: 0,
            dropped: 0,
        },
        losses: [],
        outputs: [],
    });
    return { status, text, diagnostics: diags, reportJson };
}

function toRustDiagnosticShape(d: DiagnosticRecord): Record<string, unknown> {
    return {
        code: d.code,
        fallback: d.fallback ?? "",
        message_key: d.messageKey,
        args: d.args,
        origin: d.origin,
        severity: d.severity ?? "error",
        stage: d.stage,
        url: d.url,
        target: d.target,
    };
}

/**
 * Parse Rust-side report_json. The Rust DiagnosticEnvelope serializes as
 *   { schema_version: "...", diagnostics: Diagnostic[], truncated, dropped }
 * and sits under the top-level `diagnostics` key. So we read
 * `parsed.diagnostics.diagnostics` (array), NOT the old buggy path
 * `parsed.diagnostics.envelope`.
 *
 * If the envelope is missing, corrupt, or JSON itself unparseable we still
 * return a DiagnosticRecord describing *that* — we never silently return an
 * empty array from this function.
 */
function safeParseReport(reportJson: string, urlHint?: string): {
    diagnostics: DiagnosticRecord[];
    rawStatus?: string;
    losses: unknown[];
} {
    const errDiags: DiagnosticRecord[] = [];
    let parsed: any;
    try {
        parsed = JSON.parse(reportJson);
    } catch (e) {
        errDiags.push({
            code: "panduck.convert.report-corrupt",
            fallback: "Failed to parse Rust report JSON",
            args: { error: e instanceof Error ? e.message : String(e) },
            origin: { namespace: "panduck", component: "convert" },
            severity: "error",
            stage: "convert",
            url: urlHint,
        });
        return { diagnostics: errDiags, losses: [] };
    }

    const envelope = parsed?.diagnostics;
    if (!envelope || typeof envelope !== "object") {
        errDiags.push({
            code: "panduck.convert.report-missing",
            fallback: "Rust report missing `diagnostics` envelope",
            origin: { namespace: "panduck", component: "convert" },
            severity: "error",
            stage: "convert",
            url: urlHint,
        });
        return { diagnostics: errDiags, rawStatus: parsed?.status, losses: [] };
    }

    const rawDiags = envelope.diagnostics;
    if (!Array.isArray(rawDiags)) {
        errDiags.push({
            code: "panduck.convert.report-corrupt",
            fallback: "Rust diagnostics envelope missing `diagnostics` array",
            origin: { namespace: "panduck", component: "convert" },
            severity: "error",
            stage: "convert",
            url: urlHint,
        });
        return { diagnostics: errDiags, rawStatus: parsed?.status, losses: [] };
    }

    const mapped = rawDiags.map((raw): DiagnosticRecord => {
        const r = raw as Record<string, unknown>;
        const severity = (r.severity as DiagnosticSeverity | undefined) ?? "error";
        return {
            code: String(r.code ?? "panduck.convert.unknown"),
            fallback: typeof r.fallback === "string" ? r.fallback : undefined,
            messageKey: typeof r.message_key === "string" ? r.message_key :
                typeof r.messageKey === "string" ? r.messageKey : undefined,
            args: (r.args as Record<string, unknown> | undefined) ?? undefined,
            origin: (r.origin as { namespace: string; component: string } | undefined) ?? undefined,
            severity,
            stage: (r.stage as DiagnosticRecord["stage"] | undefined) ?? "convert",
            url: typeof r.url === "string" ? r.url : undefined,
            target: typeof r.target === "string" ? r.target : undefined,
        };
    });

    return {
        diagnostics: mapped,
        rawStatus: typeof parsed.status === "string" ? parsed.status : undefined,
        losses: Array.isArray(parsed.losses) ? parsed.losses : [],
    };
}

/**
 * Derive ConverterStatus. Trust hierarchy:
 *   1. exitCode from Rust → 1 means the binary considers this a hard failure.
 *   2. Rust report.status === "failed" → same, hard failure.
 *   3. Error-severity diagnostics (executor OR Rust OR post-process) → failed.
 *   4. requireNoLoss from ConversionPlan → deny semantics. Any loss = failed.
 *   5. Otherwise, known loss → success_with_loss.
 *   6. Otherwise → success.
 */
function deriveStatus(
    diags: DiagnosticRecord[],
    ctx: { rustStatus?: string; hasRustLoss: boolean; exitCode: number; requireNoLoss: boolean },
): ConverterStatus {
    if (ctx.exitCode !== 0) return "failed";
    if (ctx.rustStatus === "failed") return "failed";
    if (diags.some((d) => d.severity === "error")) return "failed";

    // v0 only — no regex. Loss via severity + explicit prefix.
    const hasLoss = ctx.hasRustLoss || diags.some((d) =>
        d.severity === "warn" && isLossCode(d.code),
    );
    if (hasLoss) {
        if (ctx.requireNoLoss) return "failed";
        return "success_with_loss";
    }

    // Not-validated via explicit prefix.
    const hasUnvalidated = diags.some((d) =>
        d.severity === "warn" && isNotValidatedCode(d.code),
    );
    if (hasUnvalidated) return "success_unvalidated";

    return "success";
}

function isLossCode(code: string): boolean {
    if (code.endsWith(".loss")) return true;
    if (code.includes(".loss.")) return true;
    if (code === "panduck.convert.structure-loss") return true;
    return false;
}

function isNotValidatedCode(code: string): boolean {
    if (code.endsWith(".not-validated")) return true;
    if (code.includes(".not-validated.")) return true;
    return false;
}
