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
 * All three enter: Rule-executor (TS, linkedom — scope selection only, NO
 * handler execution) → Notedown Rust lowering + handler dispatch.
 * Panduck does NOT ship its own HTML parser or Markdown writer — linkedom is
 * the structural scope-select pass, Rust owns the lowering + handler dispatch.
 *
 * Converter produces a structured DiagnosticRecord[] that the Pandark Crawler
 * can aggregate into the run report without parsing free-form message text.
 */

import { loadPanduckNode } from "./node/load.js";
import type { RulePlan } from "./rule-builder.js";
import { RuleBuilder } from "./rule-builder.js";
import type { ReplaceHandlerFn } from "./rule-builder.js";
import { executePlan } from "./rule-executor.js";
import type {
    ConverterConfig,
    MarkdownPolicy,
    PolicyHandler,
    PolicyHandlerResult,
} from "./renderer-policy.js";

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
 * Markdown conversion options.
 *
 * **Only RuleBuilder / RulePlan lives here.** The legacy flat options
 * (`select`, `root`, `remove`, `required`, `loss`) were removed in favor of
 * declaring everything through RuleBuilder domains — `input.select(...)`,
 * `transform.remove(...)`, `output.requireNoLoss()`, etc.
 *
 * Loss policy is **not** a standalone option anymore — `output.requireNoLoss()`
 * inside a RuleBuilder converts to `deny` semantics for that plan, while a
 * plan that does NOT call `requireNoLoss()` defaults to `warn` semantics
 * (loss records stay, but don't push the status to failed). This keeps the
 * policy inside the same declarative body as the rest of the conversion rules.
 */
export type ToMarkdownOptions = {
    /** Format identifier passed to Panduck ("html", "docx", "epub", ...). Default "html". */
    from?: string;
    /** Label written into the report provenance when the input is anonymous. */
    label?: string;

    /**
     * Declarative rule plan — either a `RulePlan` (already built) or a live
     * `RuleBuilder` instance. If you pass a `RuleBuilder`, `.build()` is called
     * internally and its `handlers` registry is used for `.replace()` callbacks.
     */
    rules?: RulePlan | RuleBuilder;
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
    /** Primary Markdown output — always present even on partial failure. */
    markdown: string;
    /** Unified diagnostic array covering input, structure, conversion, loss, and rules. */
    diagnostics: DiagnosticRecord[];
    /** Raw report JSON from Panduck — preserved for auditing. */
    reportJson: string;
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
    // Overloads — 两个入口并存。
    // -----------------------------------------------------------------------

    /** Legacy entry: RuleBuilder / RulePlan 链式配置。向后兼容。 */
    async toMarkdown(input: ConverterInput, options?: ToMarkdownOptions): Promise<ConverterResult>;
    /** New entry: Profile 化 ConverterConfig（含 RendererPolicy）。 */
    async toMarkdown(input: ConverterInput, config: ConverterConfig): Promise<ConverterResult>;

    // -----------------------------------------------------------------------
    // Implementation — 通过 presence of `rendererPolicy` 分流。
    // -----------------------------------------------------------------------

    async toMarkdown(
        input: ConverterInput,
        arg?: ToMarkdownOptions | ConverterConfig,
    ): Promise<ConverterResult> {
        if (arg && isConverterConfig(arg)) {
            return this.toMarkdownWithPolicy(input, arg);
        }
        return this.toMarkdownLegacy(input, arg as ToMarkdownOptions | undefined);
    }

    // -----------------------------------------------------------------------
    // Legacy path — RuleBuilder.input().transform().output()
    // -----------------------------------------------------------------------

    /**
     * Legacy entry — 保持 RuleBuilder 链式不变。
     * Pipeline:
     *   1. Materialize input → HTML string (for Rule executor) + bytes (for Rust)
     *   2. Execute RulePlan via linkedom DOM executor: input/transform/output ops
     *      modify the working document.
     *   3. Serialize post-plan DOM → HTML → bytes → send to Rust.
     *   4. Rust lowering + handler dispatch.
     *   5. Derive status from executor diags + Rust report + requireNoLoss flag.
     */
    private async toMarkdownLegacy(
        input: ConverterInput,
        options?: ToMarkdownOptions,
    ): Promise<ConverterResult> {
        const from = options?.from ?? "html";
        const { html: inputHtml, bytes, label, urlHint } = await materializeInput(input, options?.label);

        const diagnostics: DiagnosticRecord[] = [];

        // (1) Resolve RulePlan + handler registry.
        const { plan, handlers } = resolveRules(options?.rules);

        // (2) Empty input short-circuit.
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

        // (3) Rule-executor pass — DOM-level input/transform/output ops.
        let planHtmlForRust = inputHtml;
        let requireNoLoss = false;
        if (plan) {
            const exec = executePlan(inputHtml, plan, handlers, urlHint);
            diagnostics.push(...exec.diagnostics);
            planHtmlForRust = exec.html;
            requireNoLoss = exec.requireNoLoss;

            const handlerKeys = Object.keys(exec.handlers);
            const handlersJson =
                handlerKeys.length > 0
                    ? JSON.stringify({
                          version: "panduck-rule-v0",
                          steps: plan.steps,
                          handlerKeys,
                      })
                    : undefined;

            // (4) Convert the post-plan HTML to bytes for Rust.
            const planBytes = new TextEncoder().encode(planHtmlForRust);

            return this.runRustConversion({
                from,
                bytes: planBytes,
                label,
                urlHint,
                diagnostics,
                handlersJson,
                requireNoLoss,
            });
        }

        // --- No RulePlan path — direct Rust conversion, no DOM executor ---
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
     *   - **不跑 RuleBuilder DOM executor**。没有 `.select()/.remove()/.unwrap()`
     *     这些 DOM 级动作。scope 选择（如果有）将来会由 Pandark ExtractionProfile
     *     在进入 Panduck 之前就做掉；renderer 这边只读 Notedown IR。
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
        if (handlerKeys.length > 0 && result.markdown) {
            result.markdown = applyPolicyHandlers(result.markdown, policyHandlers, diagnostics, urlHint);
        }

        return result;
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

        let response: { exitCode: number; markdown?: string; reportJson: string } | undefined = undefined;
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

        const markdown = response.markdown ?? "";
        const status = deriveStatus(diagnostics, {
            rustStatus,
            hasRustLoss,
            exitCode: response.exitCode,
            requireNoLoss,
        });

        return Promise.resolve({ status, markdown, diagnostics, reportJson: response.reportJson });
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Type guard: true if arg is a ConverterConfig (has `rendererPolicy`). */
function isConverterConfig(arg: object): arg is ConverterConfig {
    return "rendererPolicy" in arg && (arg as { rendererPolicy?: unknown }).rendererPolicy != null;
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

type ResolvedRules = {
    plan: RulePlan | null;
    handlers: Record<string, ReplaceHandlerFn>;
};

/** Resolve RuleBuilder / RulePlan → executable plan + handler registry. */
function resolveRules(rules: RulePlan | RuleBuilder | undefined): ResolvedRules {
    if (!rules) return { plan: null, handlers: {} };
    if (rules instanceof RuleBuilder) {
        return { plan: rules.build(), handlers: { ...rules.handlers } };
    }
    // Direct RulePlan — handlers are attached on the plan itself.
    return { plan: rules, handlers: { ...rules.handlers } };
}

async function materializeInput(
    input: ConverterInput,
    explicitLabel?: string,
): Promise<{ html: string; bytes: Uint8Array; label: string; urlHint?: string }> {
    if (typeof input === "string") {
        return { html: input, bytes: new TextEncoder().encode(input), label: explicitLabel ?? "<string>" };
    }
    if (input instanceof Uint8Array) {
        return {
            html: new TextDecoder().decode(input),
            bytes: input,
            label: explicitLabel ?? "<bytes>",
        };
    }
    if (typeof input === "object" && input !== null) {
        const candidate = input as Record<string, unknown>;
        if (typeof candidate.bytes === "function") {
            const page = input as ConverterPageLike;
            let bytes = page.bytes();
            let html: string;
            if (!bytes || bytes.length === 0) {
                html = await page.html();
                bytes = new TextEncoder().encode(html);
            } else {
                html = await page.html();
            }
            return { html, bytes, label: explicitLabel ?? (page.url ?? "<page>"), urlHint: page.url };
        }
        if ("bodyBytes" in candidate) {
            const artifact = input as ConverterArtifactLike;
            const html = artifact.bodyText ?? new TextDecoder().decode(artifact.bodyBytes);
            return {
                html,
                bytes: artifact.bodyBytes,
                label: explicitLabel ?? (artifact.finalUrl ?? "<artifact>"),
                urlHint: artifact.finalUrl,
            };
        }
    }
    return { html: "", bytes: new Uint8Array(), label: explicitLabel ?? "<unknown>" };
}

function makeResult(
    status: ConverterStatus,
    markdown: string,
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
    return { status, markdown, diagnostics: diags, reportJson };
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
 *   4. requireNoLoss from RuleBuilder → deny semantics. Any loss = failed.
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
