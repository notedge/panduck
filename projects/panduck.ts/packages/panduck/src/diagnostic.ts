/**
 * Boost diagnostic wire model.
 *
 * Mirrors `boost_diagnostic::Diagnostic` exactly (serde wire names included):
 * `code`, `severity`, `origin`, `message { key, args, fallback }`, `primary`,
 * `secondary`, `notes`, `helps`, `cause`, `actions`. No flattened
 * `message_key` / top-level `args` / top-level `fallback` aliases exist.
 */

export type DiagnosticSeverity = "bug" | "error" | "warning" | "info" | "hint";

export type DiagnosticMessageArg = Record<string, unknown>;

export type DiagnosticMessage = {
    key: string;
    /** `Vec<(String, MessageArg)>` serializes as an array of `[name, value]` pairs. */
    args: Array<[string, DiagnosticMessageArg]>;
    fallback: string | null;
};

export type DiagnosticOrigin = {
    namespace: string;
    component: string;
    stage: string | null;
};

export type DiagnosticLocation = {
    kind: "text" | "binary" | "member" | "object" | "semantic" | "virtual";
    [key: string]: unknown;
};

export type DiagnosticLabel = {
    location: DiagnosticLocation;
    message: DiagnosticMessage;
    role: "primary" | "secondary" | "context";
};

export type DiagnosticAction = {
    id: string;
    args: Array<[string, DiagnosticMessageArg]>;
};

export type DiagnosticCause = {
    code: string;
    message: DiagnosticMessage;
};

/** One diagnostic record on the boost wire. */
export type DiagnosticRecord = {
    code: string;
    severity: DiagnosticSeverity;
    origin: DiagnosticOrigin;
    message: DiagnosticMessage;
    primary: DiagnosticLabel | null;
    secondary: DiagnosticLabel[];
    notes: DiagnosticMessage[];
    helps: DiagnosticMessage[];
    cause: DiagnosticCause | null;
    actions: DiagnosticAction[];
};

/** IR loss marker recorded by Notedown (`notedown_ir::LossMarker`). */
export type LossRecord = {
    code: string;
    message: string;
    status: "Resolved" | "Unresolved" | "Partial" | "Unsupported" | "Inferred" | "Lossy";
};

/** Parsed result of a `panduck.report/v0` payload. */
export type ParsedReport = {
    diagnostics: DiagnosticRecord[];
    losses: LossRecord[];
    truncated: boolean;
    dropped: number;
};

const SEVERITIES: ReadonlySet<string> = new Set([
    "bug",
    "error",
    "warning",
    "info",
    "hint",
]);

const SEMANTIC_STATUSES: ReadonlySet<string> = new Set([
    "Resolved",
    "Unresolved",
    "Partial",
    "Unsupported",
    "Inferred",
    "Lossy",
]);

/** Text message argument (`MessageArg::Text`). */
export function textArg(value: string): DiagnosticMessageArg {
    return { text: value };
}

/** Unsigned integer message argument (`MessageArg::U64`). */
export function u64Arg(value: number): DiagnosticMessageArg {
    return { u64: value };
}

/** Text-list message argument (`MessageArg::TextList`). */
export function textListArg(values: readonly string[]): DiagnosticMessageArg {
    return { "text-list": [...values] };
}

/** Append a `url` text argument when a source URL is known. */
export function withUrl(
    args: Array<[string, DiagnosticMessageArg]>,
    url?: string,
): Array<[string, DiagnosticMessageArg]> {
    return url ? [...args, ["url", textArg(url)]] : args;
}

export type DiagnosticInit = {
    code: string;
    severity: DiagnosticSeverity;
    component: string;
    namespace?: string;
    stage?: string | null;
    messageKey?: string;
    args?: Array<[string, DiagnosticMessageArg]>;
    fallback?: string;
};

/** Build one diagnostic that already matches the boost wire shape. */
export function createDiagnostic(init: DiagnosticInit): DiagnosticRecord {
    return {
        code: init.code,
        severity: init.severity,
        origin: {
            namespace: init.namespace ?? "panduck",
            component: init.component,
            stage: init.stage ?? null,
        },
        message: {
            key: init.messageKey ?? init.code,
            args: init.args ?? [],
            fallback: init.fallback ?? null,
        },
        primary: null,
        secondary: [],
        notes: [],
        helps: [],
        cause: null,
        actions: [],
    };
}

export type ReportParseContext = {
    component: string;
    stage?: string | null;
    url?: string;
};

/**
 * Parse the Rust report JSON and read only the boost diagnostic envelope.
 *
 * The only accepted shape is `{ schemaVersion, diagnostics, truncated, dropped }`
 * with diagnostics nested under `message` / `origin`. Flattened legacy keys are
 * rejected and surfaced as a `panduck.convert.report-corrupt` error instead of
 * being silently accepted.
 */
export function parseReportEnvelope(reportJson: string, context: ReportParseContext): ParsedReport {
    const empty: ParsedReport = { diagnostics: [], losses: [], truncated: false, dropped: 0 };
    let parsed: unknown;
    try {
        parsed = JSON.parse(reportJson);
    } catch (error) {
        return {
            ...empty,
            diagnostics: [
                reportError(context, "panduck.convert.report-corrupt", "Failed to parse Rust report JSON", {
                    error: error instanceof Error ? error.message : String(error),
                }),
            ],
        };
    }

    if (!isRecord(parsed)) {
        return {
            ...empty,
            diagnostics: [
                reportError(context, "panduck.convert.report-corrupt", "Rust report JSON must be an object"),
            ],
        };
    }

    const envelope = parsed.diagnostics;
    if (!isRecord(envelope) || !Array.isArray(envelope.diagnostics)) {
        return {
            ...empty,
            diagnostics: [
                reportError(
                    context,
                    "panduck.convert.report-missing",
                    "Rust report is missing the boost diagnostic envelope",
                ),
            ],
        };
    }

    const diagnostics: DiagnosticRecord[] = [];
    for (const [index, raw] of envelope.diagnostics.entries()) {
        const diagnostic = parseDiagnostic(raw);
        if (diagnostic) {
            diagnostics.push(diagnostic);
        } else {
            diagnostics.push(
                reportError(
                    context,
                    "panduck.convert.report-corrupt",
                    "Rust diagnostic entry does not match the boost diagnostic shape",
                    { index: index.toString() },
                ),
            );
        }
    }

    const losses: LossRecord[] = [];
    if (Array.isArray(parsed.losses)) {
        for (const raw of parsed.losses) {
            const loss = parseLoss(raw);
            if (!loss) {
                diagnostics.push(
                    reportError(
                        context,
                        "panduck.convert.report-corrupt",
                        "Rust loss entry does not match the notedown loss marker shape",
                    ),
                );
                break;
            }
            losses.push(loss);
        }
    }

    return {
        diagnostics,
        losses,
        truncated: envelope.truncated === true,
        dropped: typeof envelope.dropped === "number" ? envelope.dropped : 0,
    };
}

function parseDiagnostic(raw: unknown): DiagnosticRecord | null {
    if (!isRecord(raw)) return null;
    const code = raw.code;
    const severity = raw.severity;
    if (typeof code !== "string" || typeof severity !== "string" || !SEVERITIES.has(severity)) {
        return null;
    }
    const origin = parseOrigin(raw.origin);
    const message = parseMessage(raw.message);
    if (!origin || !message) return null;

    return {
        code,
        severity: severity as DiagnosticSeverity,
        origin,
        message,
        primary: isRecord(raw.primary) ? (raw.primary as DiagnosticLabel) : null,
        secondary: Array.isArray(raw.secondary) ? (raw.secondary as DiagnosticLabel[]) : [],
        notes: parseMessageList(raw.notes),
        helps: parseMessageList(raw.helps),
        cause: parseCause(raw.cause),
        actions: parseActions(raw.actions),
    };
}

function parseOrigin(raw: unknown): DiagnosticOrigin | null {
    if (!isRecord(raw)) return null;
    const namespace = raw.namespace;
    const component = raw.component;
    if (typeof namespace !== "string" || typeof component !== "string") return null;
    return {
        namespace,
        component,
        stage: typeof raw.stage === "string" ? raw.stage : null,
    };
}

function parseMessage(raw: unknown): DiagnosticMessage | null {
    if (!isRecord(raw)) return null;
    const key = raw.key;
    if (typeof key !== "string" || !Array.isArray(raw.args)) return null;
    const args: Array<[string, DiagnosticMessageArg]> = [];
    for (const entry of raw.args) {
        if (!Array.isArray(entry) || entry.length !== 2 || typeof entry[0] !== "string" || !isRecord(entry[1])) {
            return null;
        }
        args.push([entry[0], entry[1]]);
    }
    return {
        key,
        args,
        fallback: typeof raw.fallback === "string" ? raw.fallback : null,
    };
}

function parseMessageList(raw: unknown): DiagnosticMessage[] {
    if (!Array.isArray(raw)) return [];
    const messages: DiagnosticMessage[] = [];
    for (const entry of raw) {
        const message = parseMessage(entry);
        if (message) messages.push(message);
    }
    return messages;
}

function parseCause(raw: unknown): DiagnosticCause | null {
    if (!isRecord(raw)) return null;
    const code = raw.code;
    const message = parseMessage(raw.message);
    if (typeof code !== "string" || !message) return null;
    return { code, message };
}

function parseActions(raw: unknown): DiagnosticAction[] {
    if (!Array.isArray(raw)) return [];
    const actions: DiagnosticAction[] = [];
    for (const entry of raw) {
        if (!isRecord(entry) || typeof entry.id !== "string" || !Array.isArray(entry.args)) continue;
        const args: Array<[string, DiagnosticMessageArg]> = [];
        for (const arg of entry.args) {
            if (!Array.isArray(arg) || arg.length !== 2 || typeof arg[0] !== "string" || !isRecord(arg[1])) {
                args.length = 0;
                break;
            }
            args.push([arg[0], arg[1]]);
        }
        actions.push({ id: entry.id, args });
    }
    return actions;
}

function parseLoss(raw: unknown): LossRecord | null {
    if (!isRecord(raw)) return null;
    const code = raw.code;
    const message = raw.message;
    const status = raw.status;
    if (
        typeof code !== "string"
        || typeof message !== "string"
        || typeof status !== "string"
        || !SEMANTIC_STATUSES.has(status)
    ) {
        return null;
    }
    return { code, message, status: status as LossRecord["status"] };
}

function reportError(
    context: ReportParseContext,
    code: string,
    fallback: string,
    args?: Record<string, string>,
): DiagnosticRecord {
    const messageArgs: Array<[string, DiagnosticMessageArg]> = Object.entries(args ?? {}).map(([key, value]) => [
        key,
        textArg(value),
    ]);
    if (context.url) {
        messageArgs.push(["url", textArg(context.url)]);
    }
    return createDiagnostic({
        code,
        severity: "error",
        component: context.component,
        stage: context.stage ?? null,
        fallback,
        args: messageArgs,
    });
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === "object" && value !== null && !Array.isArray(value);
}
