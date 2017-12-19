/**
 * Panduck Converter — TS authoring surface for format conversion.
 *
 * The single contract: `Converter.toMarkdown(input, options?)`.
 *
 * input can be any of the three projections that design doc §freeze identifies:
 *   - Page (from Pandark Engine — has html()/bytes())
 *   - FetchArtifact (from Pandark Engine.fetch())
 *   - string | Uint8Array | { html: string; bytes?: Uint8Array }
 *
 * All three enter the same Notedown HTML import + Markdown export in Rust.
 * Panduck does NOT ship its own HTML parser or Markdown writer.
 */

import { loadPanduckNode } from "./node/load.js";

// ---------------------------------------------------------------------------
// Input shapes
// ---------------------------------------------------------------------------

/** Anything Converter accepts as source material. */
export type ConverterInput =
    | ConverterPageLike
    | ConverterArtifactLike
    | string
    | Uint8Array;

/** Minimal shape Pandark Page must satisfy for Converter to consume it. */
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
 * These map to Notedown HTML import + Markdown export. The full Panduck
 * rule-engine (select/remove/rules) is deferred — the contract frozen in the
 * design doc lands them in a follow-up Rust-side policy builder. Until then,
 * Converter.toMarkdown returns the full-page Markdown and the user can apply
 * lightweight post-processing.
 */
export type ToMarkdownOptions = {
    /** Format identifier passed to Panduck ("html", "docx", "epub", ...). Default "html". */
    from?: string;
    /** Label written into the report provenance when the input is anonymous. */
    label?: string;
};

/** Result of a Panduck conversion. */
export type ConverterResult = {
    /** Primary Markdown output. Empty when the source produced no text. */
    markdown: string;
    /** Report JSON from Panduck — carries provenance, loss envelope, diagnostics. */
    reportJson: string;
};

// ---------------------------------------------------------------------------
// Converter
// ---------------------------------------------------------------------------

/** Thin public façade over Panduck's Rust conversion pipeline. */
export class Converter {
    private binding: ReturnType<typeof loadPanduckNode>;

    constructor() {
        this.binding = loadPanduckNode();
    }

    /**
     * Convert HTML (or any Panduck-supported reader format) to Markdown.
     *
     * The input is any of:
     *   - `Page` — from Pandark `engine.crawl()` handler
     *   - `FetchArtifact` — from Pandark `engine.fetch()`
     *   - raw `string` / `Uint8Array` — free-form HTML bytes
     */
    async toMarkdown(input: ConverterInput, options?: ToMarkdownOptions): Promise<ConverterResult> {
        const from = options?.from ?? "html";
        const { bytes, label } = await materializeInput(input, options?.label);

        if (bytes.length === 0) {
            return { markdown: "", reportJson: emptyReport("empty-input", label) };
        }

        const fn = this.binding.convertBytesMemory;
        if (!fn) {
            throw new Error(
                `Panduck binding does not expose convertBytesMemory — rebuild @notedge/panduck-native with the new N-API entry`,
            );
        }
        const response = fn(from, "markdown", label, bytes);

        return {
            markdown: response.markdown ?? "",
            reportJson: response.reportJson,
        };
    }
}

// ---------------------------------------------------------------------------
// Materialization — turn any ConverterInput into (bytes, provenance label)
// ---------------------------------------------------------------------------

async function materializeInput(
    input: ConverterInput,
    explicitLabel?: string,
): Promise<{ bytes: Uint8Array; label: string }> {
    if (typeof input === "string") {
        return {
            bytes: new TextEncoder().encode(input),
            label: explicitLabel ?? "<string>",
        };
    }
    if (input instanceof Uint8Array) {
        return {
            bytes: input,
            label: explicitLabel ?? "<bytes>",
        };
    }
    // Duck-type check for PageLike vs ArtifactLike.
    if (typeof input === "object" && input !== null) {
        const candidate = input as Record<string, unknown>;
        // PageLike has bytes() method — prefer it over ArtifactLike
        if (typeof candidate.bytes === "function") {
            const page = input as ConverterPageLike;
            let bytes = page.bytes();
            if (!bytes || bytes.length === 0) {
                // bytes() may have returned empty when the body was Reference-stored.
                // Fall back to re-encoding html().
                const html = await page.html();
                bytes = new TextEncoder().encode(html);
            }
            return {
                bytes,
                label: explicitLabel ?? (page.url ?? "<page>"),
            };
        }
        if ("bodyBytes" in candidate) {
            const artifact = input as ConverterArtifactLike;
            return {
                bytes: artifact.bodyBytes,
                label: explicitLabel ?? (artifact.finalUrl ?? "<artifact>"),
            };
        }
    }
    return { bytes: new Uint8Array(), label: explicitLabel ?? "<unknown>" };
}

// ---------------------------------------------------------------------------
// Tiny report stub for edge cases (no Rust conversion needed).
// Keeps reportJson well-formed so downstream consumers always get JSON.
// ---------------------------------------------------------------------------

function emptyReport(code: string, label: string): string {
    return JSON.stringify({
        schemaVersion: "v0",
        format: "html",
        from: "html",
        to: "markdown",
        inputLabel: label,
        exitCode: 0,
        diagnostics: {
            envelope: [
                {
                    code: `panduck.convert.${code}`,
                    fallback: `Converter: ${code} (input ${label})`,
                    origin: { namespace: "panduck", component: "convert" },
                },
            ],
        },
        artifacts: [],
    });
}
