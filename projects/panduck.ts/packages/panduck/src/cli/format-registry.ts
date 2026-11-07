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
        // org has a lexer upstream but no semantic parser or public import route,
        // so Panduck declares it known but unavailable instead of claiming a route.
        textFormat("org", "unavailable", "unavailable", ["headline", "paragraph"], ["drawer", "babel"]),
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

/**
 * Merges N-API `supportedFormats()` names into the static registry as
 * `registered` floors, and derives a record for every format the native route
 * table reaches but the static matrix never declared.
 *
 * The route table is the only source for those records: a format is a reader
 * when some route starts with it and a writer when some route ends with it,
 * so no format name needs a hand-written entry here.
 */
export function getFormatRegistry(bindings?: PanduckBindings | null): FormatRecord[] {
    const registered = new Set((bindings?.supportedFormats() ?? []).map((name) => name.toLowerCase()));
    const routes = (bindings?.supportedConversions?.() ?? []).map((route) => route.toLowerCase());
    const declared = baseFormatRegistry();
    return [...declared, ...derivedFormatRecords(routes, declared)].map((record) => {
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

function derivedFormatRecords(
    routes: readonly string[],
    declared: readonly FormatRecord[],
): FormatRecord[] {
    const declaredNames = new Set(declared.map((record) => record.format));
    const reached = new Set<string>();
    for (const route of routes) {
        const separator = route.indexOf(":");
        if (separator <= 0) {
            continue;
        }
        reached.add(route.slice(0, separator));
        reached.add(route.slice(separator + 1));
    }
    return [...reached]
        .filter((format) => !declaredNames.has(format))
        .sort()
        .map((format) =>
            textFormat(
                format,
                routes.some((route) => route.startsWith(`${format}:`)) ? "registered" : "unavailable",
                routes.some((route) => route.endsWith(`:${format}`)) ? "registered" : "unavailable",
                [],
                [],
            ),
        );
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
