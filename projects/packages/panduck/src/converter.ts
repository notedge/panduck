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
    stage?: "input" | "structure" | "convert" | "loss" | "rule";
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

    // --- Custom rules (deferred) --------------------------------------------
    /**
     * Custom node-to-Markdown rules. The full rule-engine lands in a follow-up
     * Rust-side policy builder; until then, Converter records each entry as a
     * "rule not executed" diagnostic so users can see what they expected.
     */
    rules?: Record<string, unknown>;
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

        // (1) Requested-but-not-yet-executed option diagnostics.
        if (options?.select || options?.root) {
            diagnostics.push({
                code: "panduck.convert.select-deferred",
                fallback: "`select` / `root` option requested but selector engine is not yet wired",
                messageKey: "panduck.diagnostic.selectDeferred",
                args: { select: options.select ?? options.root },
                origin: { namespace: "panduck", component: "convert" },
                severity: "info",
                stage: "convert",
                url: urlHint,
            });
        }
        if (options?.remove && options.remove.length > 0) {
            diagnostics.push({
                code: "panduck.convert.remove-deferred",
                fallback: `\`remove\` option with ${options.remove.length} selector(s) requested but not yet executed`,
                messageKey: "panduck.diagnostic.removeDeferred",
                args: { selectors: [...options.remove] },
                origin: { namespace: "panduck", component: "convert" },
                severity: "info",
                stage: "convert",
                url: urlHint,
            });
        }
        if (options?.rules && Object.keys(options.rules).length > 0) {
            diagnostics.push({
                code: "panduck.convert.rules-deferred",
                fallback: "Custom `rules` requested but rule engine is not yet wired",
                messageKey: "panduck.diagnostic.rulesDeferred",
                args: { selectors: Object.keys(options.rules) },
                origin: { namespace: "panduck", component: "convert" },
                severity: "info",
                stage: "rule",
                url: urlHint,
            });
        }

        // (2) Required-structure enforcement — lightweight selector-less gate.
        // Full CSS matching lands in the Rust side; until then we check the
        // input is non-empty and mark each declared required field as "not
        // validated" so users know it was requested but not enforced.
        if (options?.required) {
            const keys = Object.keys(options.required);
            for (const key of keys) {
                diagnostics.push({
                    code: "panduck.convert.required-not-validated",
                    fallback: `Required structure "${key}" was requested but selector matching is not yet wired`,
                    messageKey: "panduck.diagnostic.requiredNotValidated",
                    args: { key, selector: options.required[key] },
                    origin: { namespace: "panduck", component: "convert" },
                    severity: "warn",
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
            return makeResult("failed", "", diagnostics, label);
        }

        // (4) Rust conversion.
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
            return makeResult("failed", "", diagnostics, label);
        }

        let response: { markdown?: string; reportJson: string } | undefined;
        let rustError: unknown = undefined;
        try {
            response = fn(from, "markdown", label, bytes);
        } catch (err) {
            rustError = err;
        }

        if (rustError !== undefined || !response) {
            const msg = rustError instanceof Error ? rustError.message : String(rustError);
            diagnostics.push({
                code: "panduck.convert.failed",
                fallback: `Rust conversion failed for ${label}: ${msg}`,
                messageKey: "panduck.diagnostic.convertFailed",
                args: { message: msg },
                origin: { namespace: "panduck", component: "convert" },
                severity: "error",
                stage: "convert",
                url: urlHint,
            });
            return makeResult("failed", "", diagnostics, label);
        }

        // (5) Parse Rust report for loss / diagnostics envelope.
        const reportDiags = tryParseReportDiagnostics(response.reportJson);
        diagnostics.push(...reportDiags);

        const markdown = response.markdown ?? "";
        const status = deriveStatus(diagnostics, loss);

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

function makeResult(status: ConverterStatus, markdown: string, diags: DiagnosticRecord[], label: string): ConverterResult {
    const reportJson = JSON.stringify({
        schemaVersion: "v0",
        format: "html",
        from: "html",
        to: "markdown",
        inputLabel: label,
        diagnostics: { envelope: diags },
        artifacts: [],
    });
    return { status, markdown, diagnostics: diags, reportJson };
}

/** Parse the Rust-side reportJson.diagnostics.envelope into DiagnosticRecord[]. */
function tryParseReportDiagnostics(reportJson: string): DiagnosticRecord[] {
    try {
        const parsed = JSON.parse(reportJson);
        const envelope = parsed?.diagnostics?.envelope;
        if (!Array.isArray(envelope)) return [];
        return envelope.map((raw): DiagnosticRecord => {
            const r = raw as Record<string, unknown>;
            const messageKey = r.messageKey ?? r.message_key;
            return {
                code: String(r.code ?? "panduck.convert.unknown"),
                fallback: typeof r.fallback === "string" ? r.fallback : undefined,
                messageKey: typeof messageKey === "string" ? messageKey : undefined,
                args: (r.args as Record<string, unknown> | undefined) ?? undefined,
                origin: r.origin as { namespace: string; component: string } | undefined,
                severity: (r.severity as DiagnosticSeverity | undefined) ?? "error",
                stage: (r.stage as DiagnosticRecord["stage"]) ?? "convert",
            };
        });
    } catch {
        return [];
    }
}

/** Derive ConverterStatus from the diagnostics + loss policy. */
function deriveStatus(diags: DiagnosticRecord[], loss: LossPolicy): ConverterStatus {
    // Any error-severity diagnostic → failed, no matter the policy.
    if (diags.some((d) => d.severity === "error")) return "failed";
    const hasLoss = diags.some((d) =>
        /loss|deferred|not-validated|required-missing|unsupported/i.test(d.code),
    );
    if (!hasLoss) return "success";
    // Loss detected — policy decides.
    if (loss === "deny") return "failed";
    return "success_with_loss";
}
