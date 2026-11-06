import type { PanduckBindings } from "../types.js";

export type CapabilityState =
    | "registered"
    | "planned"
    | "partial"
    | "ready"
    | "unavailable"
    | "blocked";

export type OperationKind = "read" | "write";

export type OperationCapability = {
    state: CapabilityState;
    stages: string[];
};

export type FormatRecord = {
    format: string;
    container: string[];
    aliases?: string[];
    reader: OperationCapability;
    writer: OperationCapability;
    embeddedText?: string[];
    coverage: {
        state: CapabilityState;
        known: string[];
        unknown: string[];
    };
};

const TEXT_READ_STAGES = ["probe", "decode", "validate"];
const TEXT_WRITE_STAGES = ["ir", "validate"];
const CONTAINER_READ_STAGES = ["probe", "index", "decode", "validate"];
const CONTAINER_WRITE_STAGES = ["ir", "package", "validate"];

function textFormat(
    format: string,
    reader: CapabilityState,
    writer: CapabilityState,
    known: string[],
    unknown: string[],
    aliases?: string[],
): FormatRecord {
    return {
        format,
        container: ["text"],
        aliases,
        reader: { state: reader, stages: TEXT_READ_STAGES },
        writer: { state: writer, stages: TEXT_WRITE_STAGES },
        coverage: { state: reader === "ready" && writer === "ready" ? "ready" : "partial", known, unknown },
    };
}

function containerFormat(
    format: string,
    container: string[],
    reader: CapabilityState,
    writer: CapabilityState,
    embeddedText: string[],
    known: string[],
    unknown: string[],
): FormatRecord {
    return {
        format,
        container,
        reader: { state: reader, stages: CONTAINER_READ_STAGES },
        writer: { state: writer, stages: CONTAINER_WRITE_STAGES },
        embeddedText,
        coverage: {
            state:
                reader === "ready" && writer === "ready"
                    ? "ready"
                    : reader === "planned" && writer === "planned"
                      ? "planned"
                      : "partial",
            known,
            unknown,
        },
    };
}

/** Static capability registry aligned with the Panduck Living CLI matrix. */
export function baseFormatRegistry(): FormatRecord[] {
    return [
        textFormat("markdown", "partial", "ready", ["paragraph", "heading", "code_block", "link"], ["table", "footnote"]),
        textFormat(
            "markdown-project",
            "unavailable",
            "partial",
            [],
            ["index", "assets", "report"],
        ),
        textFormat("notedown", "partial", "planned", ["paragraph", "heading", "list", "link"], ["macro", "table"]),
        textFormat("org", "partial", "partial", ["headline", "paragraph"], ["drawer", "babel"]),
        textFormat("rst", "partial", "partial", ["paragraph", "directive"], ["table"]),
        textFormat("tex", "partial", "partial", ["paragraph", "command"], ["environment"]),
        textFormat("typst", "partial", "planned", ["paragraph", "heading", "code_block"], ["math", "table", "environment"]),
        textFormat("html", "planned", "planned", ["paragraph", "heading"], ["script", "style"]),
        containerFormat(
            "docx",
            ["zip", "opc"],
            "partial",
            "partial",
            ["oak-xml"],
            ["paragraph", "heading", "list", "table", "style", "hyperlink", "image", "bold", "italic", "strike", "code_block", "footnote_reference", "footnote_body", "diagram_source", "chart_source"],
            ["tracked_changes", "comment", "math", "inline_math", "raw_html", "opaque"],
        ),
        containerFormat(
            "epub",
            ["zip", "ocf"],
            "planned",
            "planned",
            ["oak-xml", "oak-xhtml"],
            ["spine", "nav", "image"],
            ["fixed_layout"],
        ),
        containerFormat(
            "pdf",
            ["pdf"],
            "partial",
            "partial",
            [],
            ["page", "text_run", "heading", "list", "table", "code_block", "math", "footnote_reference"],
            ["diagram_render", "chart_render", "image", "footnote_body", "strike", "hyperlink_annotation", "annotation", "form", "encryption"],
        ),
        containerFormat(
            "doc",
            ["ole"],
            "partial",
            "unavailable",
            ["oak-xml"],
            ["paragraph"],
            ["field", "revision"],
        ),
    ];
}

/** Merges N-API `supportedFormats()` names into the static registry as `registered` floors. */
export function getFormatRegistry(bindings?: PanduckBindings | null): FormatRecord[] {
    const registered = new Set((bindings?.supportedFormats() ?? []).map((name) => name.toLowerCase()));
    return baseFormatRegistry().map((record) => {
        if (!registered.has(record.format)) {
            return record;
        }
        const readerState = bumpRegistered(record.reader.state);
        const writerState = bumpRegistered(record.writer.state);
        return {
            ...record,
            reader: { ...record.reader, state: readerState },
            writer: { ...record.writer, state: writerState },
        };
    });
}

function bumpRegistered(state: CapabilityState): CapabilityState {
    if (state === "planned" || state === "registered") {
        return "registered";
    }
    return state;
}

export function findFormat(
    registry: readonly FormatRecord[],
    id: string,
): FormatRecord | undefined {
    const needle = id.toLowerCase();
    return registry.find(
        (record) =>
            record.format === needle || record.aliases?.some((alias) => alias.toLowerCase() === needle),
    );
}

export function isOperationReady(record: FormatRecord, operation: OperationKind): boolean {
    const capability = operation === "read" ? record.reader : record.writer;
    return capability.state === "ready";
}

/**
 * Whether the static registry advertises `from` → `to` as a conversion route.
 *
 * `formats` lists routes with this predicate and `convert`/`plan` gate on the
 * same predicate, so the advertised matrix and the executable matrix cannot
 * drift. A format registered only for rendering (for example `xlsx`) is absent
 * here even when the native layer can import it.
 */
export function routeListed(
    registry: readonly FormatRecord[],
    from: string | undefined,
    to: string | undefined,
): boolean {
    if (!from || !to) {
        return false;
    }
    const source = findFormat(registry, from);
    const target = findFormat(registry, to);
    return Boolean(
        source &&
            target &&
            source.reader.state !== "unavailable" &&
            target.writer.state !== "unavailable",
    );
}

/** Route-level capability state derived from the same records as the formats report. */
export function routeCoverage(
    registry: readonly FormatRecord[],
    from: string | undefined,
    to: string | undefined,
): { state: CapabilityState; known?: string[]; unknown?: string[] } {
    const source = from ? findFormat(registry, from) : undefined;
    const target = to ? findFormat(registry, to) : undefined;
    if (!source || !target) {
        return { state: "unavailable" };
    }
    if (source.reader.state === "unavailable" || target.writer.state === "unavailable") {
        return { state: "unavailable" };
    }
    return {
        state: source.reader.state === "ready" && target.writer.state === "ready" ? "ready" : "partial",
        // The target writer's declared coverage is what the route cannot express.
        known: target.coverage.known,
        unknown: target.coverage.unknown,
    };
}

export function ebookAliasTarget(): string {
    return "epub";
}
