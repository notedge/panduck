/**
 * TypeScript-side orchestration of the orthogonal conversion primitives.
 *
 * The Node-API surface only imports, transforms and exports. This module wires
 * those stages in memory and joins their losses and diagnostics, so report
 * shape, exit codes and presentation stay in the CLI layer.
 */

import {
    parseDiagnosticEnvelope,
    parseLossArrayJson,
    type DiagnosticRecord,
    type LossRecord,
} from "./diagnostic.js";
import { buildConvertReportJson } from "./native-report.js";
import type { PanduckBindings } from "./types.js";

export type ConversionStageInput = {
    from: string;
    to: string;
    label: string;
    bytes: Uint8Array;
    transformsJson?: string;
    budgetsJson?: string;
};

export type ProjectStageInput = {
    from: string;
    label: string;
    bytes: Uint8Array;
    transformsJson?: string;
    budgetsJson?: string;
};

export type StageOutcome = {
    /** True when every stage completed. */
    ok: boolean;
    /** Serialized transform pass reports, parsed. */
    passes: unknown[];
    /** Loss records after export. */
    losses: LossRecord[];
    /** Diagnostics in report order. */
    diagnostics: DiagnosticRecord[];
    /** Parsed coverage records after export. */
    coverage: unknown;
    /** Text output when the target is textual. */
    text?: string;
    /** HTML output when the target is `html`. */
    html?: string;
    /** Binary output when the target is a container. */
    binary?: Uint8Array;
};

export type ProjectOutcome = {
    /** True when every stage completed. */
    ok: boolean;
    /** Serialized transform pass reports, parsed. */
    passes: unknown[];
    /** Loss records after export. */
    losses: LossRecord[];
    /** Diagnostics in report order. */
    diagnostics: DiagnosticRecord[];
    /** Parsed coverage records after export. */
    coverage: unknown;
    /** Primary Markdown body for `index.md`. */
    indexMarkdown: string;
    /** Additional chapters. */
    chapters: Array<{ path: string; markdown: string }>;
    /** Materialized assets. */
    assets: Array<{ path: string; bytes: Uint8Array }>;
    /** Sources that could not be materialized. */
    unresolvedAssets: string[];
};

const MISSING_BINDINGS_CODE = "panduck.convert.binding-missing";

/** Run import, transform and export in memory. */
export function convertInMemory(bindings: PanduckBindings, input: ConversionStageInput): StageOutcome {
    const { importDocument, transformDocument, exportDocument } = bindings;
    if (!importDocument || !transformDocument || !exportDocument) {
        return failure(missingBindingDiagnostics(input.from));
    }

    const imported = importDocument(input.from, input.label, input.bytes, input.budgetsJson);
    const importedDiagnostics = parseDiagnosticEnvelope(imported.diagnosticsJson, { component: "convert", stage: "read" });
    if (!imported.success || imported.graphJson === null) {
        return failure(importedDiagnostics.diagnostics);
    }

    const transformed = transformDocument(imported.graphJson, input.transformsJson);
    const transformDiagnostics = parseDiagnosticEnvelope(transformed.diagnosticsJson, {
        component: "convert",
        stage: "transform",
    });
    if (!transformed.success) {
        return failure(transformDiagnostics.diagnostics);
    }

    const exported = exportDocument(input.to, transformed.graphJson);
    const exportDiagnostics = parseDiagnosticEnvelope(exported.diagnosticsJson, { component: "convert", stage: "write" });
    if (!exported.success) {
        return failure([...exportDiagnostics.diagnostics, ...transformDiagnostics.diagnostics]);
    }

    const parsedLosses = parseLossArrayJson(exported.lossesJson, { component: "convert", stage: "write" });
    const passes = parseJsonArray(transformed.passesJson);
    return {
        ok: true,
        passes,
        losses: parsedLosses.losses,
        diagnostics: [...parsedLosses.diagnostics, ...exportDiagnostics.diagnostics, ...transformDiagnostics.diagnostics],
        coverage: parseJson(exported.coverageJson),
        text: exported.text ?? undefined,
        html: exported.html ?? undefined,
        binary: exported.binary ?? undefined,
    };
}

/** Aggregated in-memory conversion: joins the primitives into a CLI-shaped report. */
export type InMemoryConversion = {
    /** Joined `panduck.report/v0` payload. */
    reportJson: string;
    /** Text output when the target is textual. */
    text?: string;
    /** HTML output when the target is `html`. */
    html?: string;
    /** Binary output when the target is a container. */
    binary?: Uint8Array;
};

/**
 * Runs import, transform and export in memory and joins the stages into one
 * report. This is the synchronous counterpart of `Converter` for consumers that
 * already hold bytes and only need the conversion outcome, such as the harness.
 * Exit codes stay with the consumer and are derived from the report status.
 */
export function convertInMemoryReport(
    bindings: PanduckBindings,
    input: ConversionStageInput,
): InMemoryConversion {
    const outcome = convertInMemory(bindings, input);
    const status = outcome.ok ? (outcome.losses.length > 0 ? "success_with_loss" : "success") : "failed";
    const reportJson = buildConvertReportJson({
        from: input.from,
        to: input.to,
        label: input.label,
        status,
        passes: outcome.passes,
        coverage: outcome.coverage,
        losses: outcome.losses,
        diagnostics: outcome.diagnostics,
    });
    return {
        reportJson,
        text: outcome.text,
        html: outcome.html,
        binary: outcome.binary,
    };
}

/** Run import, transform and Markdown project export in memory. */
export function exportProjectInMemory(bindings: PanduckBindings, input: ProjectStageInput): ProjectOutcome {
    const { importDocument, transformDocument, exportMarkdownProject } = bindings;
    if (!importDocument || !transformDocument || !exportMarkdownProject) {
        return projectFailure(missingBindingDiagnostics(input.from));
    }

    const imported = importDocument(input.from, input.label, input.bytes, input.budgetsJson);
    const importedDiagnostics = parseDiagnosticEnvelope(imported.diagnosticsJson, { component: "convert", stage: "read" });
    if (!imported.success || imported.graphJson === null) {
        return projectFailure(importedDiagnostics.diagnostics);
    }

    const transformed = transformDocument(imported.graphJson, input.transformsJson);
    const transformDiagnostics = parseDiagnosticEnvelope(transformed.diagnosticsJson, {
        component: "convert",
        stage: "transform",
    });
    if (!transformed.success) {
        return projectFailure(transformDiagnostics.diagnostics);
    }

    const exported = exportMarkdownProject(transformed.graphJson);
    const exportDiagnostics = parseDiagnosticEnvelope(exported.diagnosticsJson, { component: "convert", stage: "write" });
    if (!exported.success) {
        return projectFailure([...exportDiagnostics.diagnostics, ...transformDiagnostics.diagnostics]);
    }

    const parsedLosses = parseLossArrayJson(exported.lossesJson, { component: "convert", stage: "write" });
    const passes = parseJsonArray(transformed.passesJson);
    return {
        ok: true,
        passes,
        losses: parsedLosses.losses,
        diagnostics: [...parsedLosses.diagnostics, ...exportDiagnostics.diagnostics, ...transformDiagnostics.diagnostics],
        coverage: parseJson(exported.coverageJson),
        indexMarkdown: exported.indexMarkdown,
        chapters: exported.chapters,
        assets: exported.assets,
        unresolvedAssets: exported.unresolvedAssets,
    };
}

function missingBindingDiagnostics(from: string): DiagnosticRecord[] {
    return [
        {
            code: MISSING_BINDINGS_CODE,
            severity: "error",
            origin: { namespace: "panduck", component: "convert", stage: "read" },
            message: {
                key: MISSING_BINDINGS_CODE,
                args: [["format", { text: from }]],
                fallback: "Native conversion primitives are missing",
            },
            primary: null,
            secondary: [],
            notes: [],
            helps: [],
            cause: null,
            actions: [],
        },
    ];
}

function failure(diagnostics: DiagnosticRecord[]): StageOutcome {
    return {
        ok: false,
        passes: [],
        losses: [],
        diagnostics,
        coverage: null,
    };
}

function projectFailure(diagnostics: DiagnosticRecord[]): ProjectOutcome {
    return {
        ok: false,
        passes: [],
        losses: [],
        diagnostics,
        coverage: null,
        indexMarkdown: "",
        chapters: [],
        assets: [],
        unresolvedAssets: [],
    };
}

function parseJsonArray(json: string): unknown[] {
    try {
        const parsed: unknown = JSON.parse(json);
        return Array.isArray(parsed) ? parsed : [];
    } catch {
        return [];
    }
}

function parseJson(json: string): unknown {
    try {
        return JSON.parse(json) as unknown;
    } catch {
        return null;
    }
}
