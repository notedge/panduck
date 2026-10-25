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
import type {
    ConverterConfig,
    MarkdownPolicy,
    PolicyHandler,
    PolicyHandlerResult,
} from "./renderer-policy.js";
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

/** Public façade over Panduck's Rust conversion pipeline + TS Rule executor. */
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
     *   - `handlers` 挂在 MarkdownPolicy 上，在 writer 阶段触发，接受 IR block
     *     kind + attributes，返回 Markdown writer fragment。
     *   - 当前 Rust 侧还没有真正调用 policy handlers —— 我们把 policy 序列化成
     *     handlersJson 传给 Rust，让 wire contract 先就位，handler 真正执行留待
     *     Rust 端落地。TS 端在这里先 **模拟** 一遍 handlers，以便 e2e 能跑。
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

        // Build handlersJson from MarkdownPolicy — 把 handlerKeys 传给 Rust，
        // 让 wire contract 就位。TS 侧保留 handlers closure 用于后续 writer 模拟。
        const policyHandlers = (policy.handlers ?? {}) as Record<string, PolicyHandler>;
        const handlerKeys = Object.keys(policyHandlers);
        const handlersJson =
            handlerKeys.length > 0
                ? JSON.stringify({
                      version: "panduck-policy-v0",
                      policyName: policy.name,
                      handlers: handlerKeys,
                      config: {
                          heading: policy.heading,
                          bullet: policy.bullet,
                          fencedCode: policy.fencedCode,
                          unsupported: policy.unsupported,
                      },
                  })
                : undefined;

        diagnostics.push({
            code: "panduck.convert.policy-active",
            fallback: `Using RendererPolicy "${policy.name}" with ${handlerKeys.length} handler(s)`,
            args: { policyName: policy.name, handlerCount: handlerKeys.length },
            origin: { namespace: "panduck", component: "convert" },
            severity: "info",
            stage: "convert",
            url: urlHint,
        });

        const result = await this.runRustConversion({
            from,
            bytes,
            label,
            urlHint,
            diagnostics,
            handlersJson,
            requireNoLoss: false,
        });

        // --- TS 侧 writer 模拟：在 markdown 输出上跑一遍 policy handlers ---
        // 这不是最终形态（最终在 Rust writer 里触发），但能让 policy
        // 的 handlers 在 TS 端先产生可见效果，让 e2e 能跑。
        if (handlerKeys.length > 0 && result.text) {
            result.text = applyPolicyHandlers(result.text, policyHandlers, diagnostics, urlHint);
        }

        return result;
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
     *   - **不跑 policy handler 哨兵模拟**。当前 v0 Rust writer 还没真正触发
     *     handler —— 我们把 ConversionPlan 序列化成 handlersJson 传给 Rust，
     *     让 wire contract 先就位。handler 真正执行留待 Rust 端落地。
     *   - **transforms 真实执行**。Rust 侧 `panduck-convert::transform` 解析
     *     handlersJson 里的 transforms，在 `DocumentGraph` 上跑内置 pass
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

        // (1) 序列化 ConversionPlan → handlersJson。
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

        // (2) 如果 output 带 RendererPolicy，也序列化进去。
        const policyHandlers = (plan.output.policy?.handlers ?? {}) as Record<string, PolicyHandler>;
        if (Object.keys(policyHandlers).length > 0) {
            diagnostics.push({
                code: "panduck.convert.policy-attached",
                fallback: `ConversionPlan "${plan.id}" 附加 RendererPolicy "${plan.output.policy?.name}"`,
                args: { planId: plan.id, policyName: plan.output.policy?.name },
                origin: { namespace: "panduck", component: "convert" },
                severity: "info",
                stage: "output",
                url: urlHint,
            });
        }

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
 * 在 TS 侧模拟 writer-level policy handler 执行。
 *
 * 不是最终形态（Rust writer 会直接在 IR block 上触发 handlers），但在
 * Rust 端真正落地之前，它能让自定义 handler 在 markdown 输出里产生
 * 可见效果，让 e2e 能跑。
 *
 * 做法：Rust HTML→Markdown lowering 会保留 HTML 注释作为文本。我们约定
 * 一个哨兵格式：`<!-- panduck-policy:block-kind -->`，然后在 markdown 里
 * 找到这些哨兵，对每个跑一遍对应 handler。
 *
 * 输入 HTML 如果想触发某个 handler，写：
 *   <div><!-- panduck-policy:highlight -->important</div>
 * Rust 会把注释当普通文本带进 markdown。TS 端在这里替换。
 */
function applyPolicyHandlers(
    markdown: string,
    handlers: Record<string, PolicyHandler>,
    diagnostics: DiagnosticRecord[],
    urlHint?: string,
): string {
    const handlerKeys = Object.keys(handlers);
    if (handlerKeys.length === 0) return markdown;

    // Match a policy sentinel comment and capture both the kind and the text following it
    // until the next sentinel or end of string.
    //  pattern: <!-- panduck-policy:<kind> --> followed by anything up to the next sentinel or EOF
    const sentinelRe = /<!--\s*panduck-policy:([A-Za-z0-9_\-\.]+)\s*-->/g;

    let out = markdown;
    const matches: Array<{ idx: number; key: string; matchLength: number }> = [];
    let m: RegExpExecArray | null;
    while ((m = sentinelRe.exec(out)) !== null) {
        matches.push({ idx: m.index, key: m[1], matchLength: m[0].length });
    }

    if (matches.length === 0) return out;

    // Walk from end to start so offsets stay stable.
    for (let i = matches.length - 1; i >= 0; i--) {
        const match = matches[i];
        const handler = handlers[match.key];
        if (!handler) {
            diagnostics.push({
                code: "panduck.convert.policy-handler-missing",
                fallback: `Policy handler "${match.key}" not found — pass-through default behavior applies`,
                args: { key: match.key },
                origin: { namespace: "panduck", component: "convert" },
                severity: "info",
                stage: "convert",
                url: urlHint,
            });
            continue;
        }

        // Sentinel 之后到下一个 sentinel（或 EOF）的内容，当作 "原始 markdown 片段" 传给 handler。
        const nextSentinelStart = i > 0 ? matches[i - 1].idx + matches[i - 1].matchLength : out.length;
        // 注意：matches 按正向记录，i = matches.length - 1 是最后一个 sentinel，
        // 它之后的文本跑到 EOF。我们需要下一个 **更晚** 的 sentinel，即 matches[i+1]。
        // 修正：matches 数组是正序的；当前元素 match 之后的下一个 sentinel 是 matches[i+1]。
        // 这里的循环是倒序的，所以正确的边界是：
        const endBoundary = i + 1 < matches.length ? matches[i + 1].idx : out.length;

        const rawFragment = out.substring(match.idx + match.matchLength, endBoundary);

        let result: PolicyHandlerResult;
        try {
            result = handler({ blockKind: match.key });
        } catch (err) {
            diagnostics.push({
                code: "panduck.convert.policy-handler-threw",
                fallback: `Policy handler "${match.key}" threw: ${err instanceof Error ? err.message : String(err)}`,
                args: { key: match.key },
                origin: { namespace: "panduck", component: "convert" },
                severity: "error",
                stage: "convert",
                url: urlHint,
            });
            continue;
        }

        const replacement = (() => {
            switch (result.kind) {
                case "suppress":
                    return "";
                case "fragment":
                    return result.markdown;
                case "pass-through":
                default:
                    // 去掉 sentinel，保留原始片段
                    return rawFragment;
            }
        })();

        out = out.substring(0, match.idx) + replacement + out.substring(endBoundary);
    }

    return out;
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
