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
 * All three enter the same Notedown HTML import + Markdown export in Rust.
 * Panduck does NOT ship its own HTML parser or Markdown writer.
 *
 * Converter produces a structured DiagnosticRecord[] that the Pandark Crawler
 * can aggregate into the run report without parsing free-form message text.
 */

import { loadPanduckNode } from "./node/load.js";
import type {
    RulePlan,
    RuleStep,
    RuleOp,
    InputOp,
    TransformOp,
    OutputOp,
} from "./rule-builder.js";
import { RuleBuilder } from "./rule-builder.js";

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
 * Loss policy for conversion.
 *
 * - `"deny"`  — any known structural loss results in a failing ConverterResult.
 * - `"allow"` — conversion succeeds with a `success_with_loss` status and the
 *               loss record is preserved in diagnostics.
 * - `"warn"`  — default. Conversion succeeds with `success_with_loss` when
 *               loss is known, diagnostics carry all loss records.
 *
 * **None of these close the diagnostic gate.** Even `"allow"` and `"warn"`
 * must record every loss with target, reason, and typed args.
 */
export type LossPolicy = "deny" | "allow" | "warn";

/**
 * Markdown conversion options.
 *
 * The options here map to Notedown's conversion policies. Some — `remove`,
 * `rules` — are deferred to a Rust-side policy builder; until they land,
 * Converter records them as "requested but not executed" diagnostics so users
 * can see what was requested and adjust.
 */
export type ToMarkdownOptions = {
    /** Format identifier passed to Panduck ("html", "docx", "epub", ...). Default "html". */
    from?: string;
    /** Label written into the report provenance when the input is anonymous. */
    label?: string;

    // --- Selection & removal ------------------------------------------------
    /** CSS selector for the root element to convert. */
    select?: string;
    /** In-HTML selector for the root (alternate spelling of `select`). */
    root?: string;
    /** Elements or selectors to strip before conversion. */
    remove?: readonly string[];

    // --- Required structure -------------------------------------------------
    /**
     * Selectors that must match in the selected input. When a key is declared
     * as required and its selector does not match, Converter produces a
     * `panduck.convert.required-missing` diagnostic and the page fails.
     *
     * Example:
     *   required: { title: "h1", content: ".article-body" }
     */
    required?: Record<string, string>;

    // --- Loss policy --------------------------------------------------------
    /** What to do when known structural loss is detected during conversion. */
    loss?: LossPolicy;

    // --- Declarative rule plan ----------------------------------------------
    /**
     * Declarative rule plan produced by `new RuleBuilder().input(...).transform(...).output(...).build()`.
     * Can also be a RuleBuilder itself — Converter will call `.build()` for you.
     *
     * The rule engine that actually executes these plans lives in a Rust-side
     * policy builder that is not wired yet. Until it lands, Converter records
     * every requested operation as a structured diagnostic so the full intent
     * is preserved in the run report and Rust can catch up without guessing.
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
export type ConverterStatus = "success" | "success_with_loss" | "failed";

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

/** Public façade over Panduck's Rust conversion pipeline. */
export class Converter {
    private binding: ReturnType<typeof loadPanduckNode>;

    constructor() {
        this.binding = loadPanduckNode();
    }

    /**
     * Convert HTML (or any Panduck-supported reader format) to Markdown.
     *
     * The input is any of:
     *   - `Page` — from Pandark Crawler handlePage
     *   - `FetchArtifact` — from Pandark Crawler.fetch()
     *   - raw `string` / `Uint8Array`
     */
    async toMarkdown(input: ConverterInput, options?: ToMarkdownOptions): Promise<ConverterResult> {
        const from = options?.from ?? "html";
        const loss: LossPolicy = options?.loss ?? "warn";
        const { bytes, label, urlHint } = await materializeInput(input, options?.label);

        const diagnostics: DiagnosticRecord[] = [];

        // (1) Flatten RuleBuilder → RulePlan and walk every step/op so the full
        // declarative intent is preserved as structured diagnostics.
        const plan = resolvePlan(options?.rules);
        if (plan !== null) {
            diagnostics.push(...planToDiagnostics(plan, urlHint));
        }

        // (2) Legacy flat options — still honored while the Rust side migrates.
        // They each collapse into a single diagnostic so the author can see
        // what was requested alongside any RulePlan.
        if (options?.select || options?.root) {
            diagnostics.push({
                code: "panduck.rule.select-legacy",
                fallback: "Legacy flat `select` / `root` option — prefer RuleBuilder.input(i => i.select(...))",
                messageKey: "panduck.diagnostic.selectLegacy",
                args: { select: options.select ?? options.root },
                origin: { namespace: "panduck", component: "convert" },
                severity: "info",
                stage: "input",
                url: urlHint,
            });
        }
        if (options?.remove && options.remove.length > 0) {
            diagnostics.push({
                code: "panduck.rule.remove-legacy",
                fallback: `Legacy flat \`remove\` option with ${options.remove.length} selector(s) — prefer RuleBuilder.transform(t => t.remove(...))`,
                messageKey: "panduck.diagnostic.removeLegacy",
                args: { selectors: [...options.remove] },
                origin: { namespace: "panduck", component: "convert" },
                severity: "info",
                stage: "transform",
                url: urlHint,
            });
        }
        if (options?.required) {
            const keys = Object.keys(options.required);
            for (const key of keys) {
                diagnostics.push({
                    code: "panduck.rule.require-legacy",
                    fallback: `Legacy flat required "${key}" → prefer RuleBuilder.input(i => i.require("${key}", "${options.required![key]}"))`,
                    messageKey: "panduck.diagnostic.requireLegacy",
                    args: { key, selector: options.required[key] },
                    origin: { namespace: "panduck", component: "convert" },
                    severity: "info",
                    stage: "structure",
                    target: options.required[key],
                    url: urlHint,
                });
            }
        }

        // (3) Empty input short-circuit — no Rust conversion needed.
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

        // (4) Rust conversion — NEVER throws now. Failures come back as
        // `exit_code: 1` with a full report_json that carries the structured
        // DiagnosticEnvelope. We parse that the same way as the success path
        // so the two modes never diverge on what diagnostics get kept.
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

        let response: {
            exitCode: number;
            markdown?: string;
            reportJson: string;
        } | undefined = undefined;
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
                messageKey: "panduck.diagnostic.nativePanic",
                args: { message: msg },
                origin: { namespace: "panduck", component: "convert" },
                severity: "error",
                stage: "convert",
                url: urlHint,
            });
            return makeResult("failed", "", diagnostics, label, urlHint);
        }

        // (5) Parse Rust report for status, losses, and DiagnosticEnvelope.
        //     Both success and failure paths enter here — the envelope is
        //     the single source of truth for what actually happened.
        const parsedReport = safeParseReport(response.reportJson);
        diagnostics.push(...parsedReport.diagnostics);

        // (6) Derive status from Rust report + diagnostics + loss policy.
        const rustStatus = parsedReport.rawStatus;
        const hasRustLoss = parsedReport.losses.length > 0;
        if (hasRustLoss) {
            diagnostics.push({
                code: "panduck.convert.rust-loss",
                fallback: `Rust reported ${parsedReport.losses.length} known structural loss(es) for ${label}`,
                messageKey: "panduck.diagnostic.rustLoss",
                args: { count: parsedReport.losses.length },
                origin: { namespace: "panduck", component: "convert" },
                severity: "warn",
                stage: "loss",
                url: urlHint,
            });
        }

        const markdown = response.markdown ?? "";
        const status = deriveStatus(diagnostics, loss, {
            rustStatus,
            hasRustLoss,
            exitCode: response.exitCode,
        });

        return {
            status,
            markdown,
            diagnostics,
            reportJson: response.reportJson,
        };
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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
                const html = await page.html();
                bytes = new TextEncoder().encode(html);
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

/**
 * Convert TS-side flat DiagnosticRecord into the shape Rust's
 * DiagnosticEnvelope expects (snake_case + origin namespace/component).
 * This lets failure-path reportJson match the success-path exactly.
 */
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
        // Rust uses snake_case for wire — read both names defensively.
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
 *   3. Error-severity diagnostics → failed, no matter the policy.
 *   4. Diagnostics + Rust losses → has known structural loss; policy decides.
 *   5. Otherwise → success.
 *
 * We deliberately do NOT trust a raw `status` field inside a ConverterResult
 * that the caller hand-built — diagnostics + exit_code are the wire truth.
 */
function deriveStatus(
    diags: DiagnosticRecord[],
    loss: LossPolicy,
    rustReport: { rustStatus?: string; hasRustLoss: boolean; exitCode: number },
): ConverterStatus {
    if (rustReport.exitCode !== 0) return "failed";
    if (rustReport.rustStatus === "failed") return "failed";
    if (diags.some((d) => d.severity === "error")) return "failed";
    const hasLoss = rustReport.hasRustLoss || diags.some((d) =>
        /loss|not-validated|required-missing|unsupported|rule\./i.test(d.code),
    );
    if (!hasLoss) return "success";
    if (loss === "deny") return "failed";
    return "success_with_loss";
}

// ---------------------------------------------------------------------------
// Rule-plan helpers — bridge RuleBuilder / RulePlan into DiagnosticRecord[]
// so Panduck preserves the full declarative intent even before the Rust
// executor is wired.
// ---------------------------------------------------------------------------

function resolvePlan(rules: RulePlan | RuleBuilder | undefined): RulePlan | null {
    if (!rules) return null;
    if (rules instanceof RuleBuilder) return rules.build();
    return rules;
}

/**
 * Walk every step/op in a RulePlan and produce a DiagnosticRecord per op.
 * All current diagnostics are severity=info because Rust still has to
 * execute the plan; they carry `stage` (input/structure/transform/output/rule)
 * so a future host can tell them apart and replace with real execution
 * outcomes.
 */
function planToDiagnostics(plan: RulePlan, urlHint?: string): DiagnosticRecord[] {
    const out: DiagnosticRecord[] = [];
    for (let stepIdx = 0; stepIdx < plan.steps.length; stepIdx++) {
        const step = plan.steps[stepIdx];
        for (let opIdx = 0; opIdx < step.ops.length; opIdx++) {
            const op = step.ops[opIdx];
            out.push(opToDiagnostic(op, step, stepIdx, opIdx, urlHint));
        }
    }
    return out;
}

function opToDiagnostic(
    op: RuleOp,
    step: RuleStep,
    stepIdx: number,
    opIdx: number,
    urlHint?: string,
): DiagnosticRecord {
    const common: Pick<DiagnosticRecord, "origin" | "severity" | "url"> = {
        origin: { namespace: "panduck", component: "rule" },
        severity: "info",
        url: urlHint,
    };
    const commonArgs = { step: stepIdx + 1, op: opIdx + 1 };
    const commonStage = domainToStage(step.domain);

    switch (op.op) {
        case "select":
            return {
                code: "panduck.rule.select.requested",
                fallback: `Rule step ${stepIdx + 1} input.select("${op.selector}") — executor not yet wired`,
                messageKey: "panduck.rule.select.requested",
                args: { ...commonArgs, selector: op.selector },
                target: op.selector,
                stage: commonStage,
                ...common,
            };
        case "require":
            return {
                code: "panduck.rule.require.requested",
                fallback: `Rule step ${stepIdx + 1} input.require("${op.key}", "${op.selector}") — executor not yet wired`,
                messageKey: "panduck.rule.require.requested",
                args: { ...commonArgs, key: op.key, selector: op.selector },
                target: op.selector,
                stage: commonStage,
                ...common,
            };
        case "remove":
            return {
                code: "panduck.rule.remove.requested",
                fallback: `Rule step ${stepIdx + 1} transform.remove("${op.selector}") — executor not yet wired`,
                messageKey: "panduck.rule.remove.requested",
                args: { ...commonArgs, selector: op.selector },
                target: op.selector,
                stage: commonStage,
                ...common,
            };
        case "unwrap":
            return {
                code: "panduck.rule.unwrap.requested",
                fallback: `Rule step ${stepIdx + 1} transform.unwrap("${op.selector}") — executor not yet wired`,
                messageKey: "panduck.rule.unwrap.requested",
                args: { ...commonArgs, selector: op.selector },
                target: op.selector,
                stage: commonStage,
                ...common,
            };
        case "replace":
            return {
                code: "panduck.rule.replace.requested",
                fallback: `Rule step ${stepIdx + 1} output.replace("${op.selector}", handler="${op.handlerKey}") — executor not yet wired`,
                messageKey: "panduck.rule.replace.requested",
                args: { ...commonArgs, selector: op.selector, handlerKey: op.handlerKey },
                target: op.selector,
                stage: commonStage,
                ...common,
            };
        case "require-no-loss":
            return {
                code: "panduck.rule.require-no-loss.requested",
                fallback: `Rule step ${stepIdx + 1} output.requireNoLoss() — executor not yet wired`,
                messageKey: "panduck.rule.requireNoLoss.requested",
                args: commonArgs,
                stage: commonStage,
                ...common,
            };
    }
}

function domainToStage(domain: RuleStep["domain"]): NonNullable<DiagnosticRecord["stage"]> {
    switch (domain) {
        case "input":
            return "input";
        case "transform":
            return "transform";
        case "output":
            return "output";
    }
}
