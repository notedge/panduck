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

    /**
     * Convert HTML (or any Panduck-supported reader format) to Markdown.
     *
     * Pipeline:
     *   1. Materialize input → HTML string (for Rule executor) + bytes (for Rust)
     *   2. Execute RulePlan via linkedom DOM executor: input/transform/output ops
     *      modify the working document; handler callbacks embed sentinels
     *   3. Serialize post-plan DOM → HTML → convert to bytes → send to Rust
     *   4. Rust lowering → parse report → post-process Markdown (sentinel substitution)
     *   5. Derive status from executor diags + Rust report + requireNoLoss flag
     */
    async toMarkdown(input: ConverterInput, options?: ToMarkdownOptions): Promise<ConverterResult> {
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
        //     This is where `.select()`, `.require()`, `.remove()`, `.unwrap()`,
        //     and `.replace()` callbacks actually run against the document.
        let planHtmlForRust = inputHtml;
        let requireNoLoss = false;
        if (plan) {
            const exec = executePlan(inputHtml, plan, handlers, urlHint);
            diagnostics.push(...exec.diagnostics);
            planHtmlForRust = exec.html;
            requireNoLoss = exec.requireNoLoss;
            // Handler registry from RuleBuilder — Rust lowering consumes these
            // to execute custom `.replace()` handlers. We do NOT post-process
            // Markdown with sentinel regex here anymore.
            const registeredHandlers = exec.handlers;

            // (4) Convert the post-plan HTML to bytes for Rust.
            const planBytes = new TextEncoder().encode(planHtmlForRust);

            // (5) Rust conversion — NEVER throws.
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
                return makeResult("failed", "", diagnostics, label, urlHint);
            }

            let response: { exitCode: number; markdown?: string; reportJson: string } | undefined = undefined;
            let nativePanic: unknown = undefined;
            try {
                response = fn(from, "markdown", label, planBytes);
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
                return makeResult("failed", "", diagnostics, label, urlHint);
            }

            // (6) Parse Rust report + accumulate loss/warn diags.
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

            // (7) No sentinel post-process — handler dispatch happens on Rust side.
            const markdown = response.markdown ?? "";

            // (8) Derive status. requireNoLoss from RuleBuilder overrides the
            //     default "warn" policy — it is equivalent to loss policy "deny".
            const status = deriveStatus(diagnostics, {
                rustStatus,
                hasRustLoss,
                exitCode: response.exitCode,
                requireNoLoss,
            });

            return { status, markdown, diagnostics, reportJson: response.reportJson };
        }

        // --- No RulePlan path — direct Rust conversion, no DOM executor ---
        const fn = this.binding.convertBytesMemory;
        if (!fn) {
            diagnostics.push({
                code: "panduck.convert.binding-missing",
                fallback: "Panduck binding does not expose convertBytesMemory — rebuild @notedge/panduck-native",
                origin: { namespace: "panduck", component: "convert" },
                severity: "error",
                stage: "convert",
                url: urlHint,
            });
            return makeResult("failed", "", diagnostics, label, urlHint);
        }

        let response: { exitCode: number; markdown?: string; reportJson: string } | undefined = undefined;
        let nativePanic: unknown = undefined;
        try {
            response = fn(from, "markdown", label, bytes);
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
            return makeResult("failed", "", diagnostics, label, urlHint);
        }

        const parsedReport = safeParseReport(response.reportJson);
        diagnostics.push(...parsedReport.diagnostics);
        if (parsedReport.losses.length > 0) {
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
            rustStatus: parsedReport.rawStatus,
            hasRustLoss: parsedReport.losses.length > 0,
            exitCode: response.exitCode,
            requireNoLoss: false,
        });

        return { status, markdown, diagnostics, reportJson: response.reportJson };
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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
