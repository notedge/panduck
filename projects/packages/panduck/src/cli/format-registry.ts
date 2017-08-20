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
        textFormat("notedown", "planned", "planned", ["paragraph", "block", "inline"], ["macro"]),
        textFormat("org", "partial", "partial", ["headline", "paragraph"], ["drawer", "babel"]),
        textFormat("rst", "partial", "partial", ["paragraph", "directive"], ["table"]),
        textFormat("tex", "partial", "partial", ["paragraph", "command"], ["environment"]),
        textFormat("html", "planned", "planned", ["paragraph", "heading"], ["script", "style"]),
        containerFormat(
            "docx",
            ["zip", "opc"],
            "partial",
            "unavailable",
            ["oak-xml"],
            ["paragraph", "heading", "style", "hyperlink", "image", "bold", "italic"],
            ["tracked_changes", "comment"],
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
            "planned",
            "planned",
            [],
            ["page", "text_run", "image"],
            ["annotation", "form", "encryption"],
        ),
        containerFormat(
            "doc",
            ["ole"],
            "planned",
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

export function findFormat(registry: FormatRecord[], id: string): FormatRecord | undefined {
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

export function ebookAliasTarget(): string {
    return "epub";
}
