/**
 * `panduck.report/v0` payloads assembled in TS.
 *
 * Conversion primitives return coverage, losses and diagnostics separately. This
 * module joins them into the report envelope the CLI writes for Markdown projects
 * and the library returns from `Converter`. Keys are sorted to keep the serialized
 * shape stable across runs.
 */

import type { DiagnosticRecord, LossRecord } from "./diagnostic.js";

export type ReportStatusValue = "success" | "success_with_loss" | "blocked" | "failed";

export type ConvertReportInput = {
    from: string;
    to: string;
    label: string;
    status: ReportStatusValue;
    passes: unknown[];
    coverage: unknown;
    losses: LossRecord[];
    diagnostics: DiagnosticRecord[];
};

export type ProjectReportInput = {
    from: string;
    label: string;
    status: ReportStatusValue;
    passes: unknown[];
    coverage: unknown;
    losses: LossRecord[];
    diagnostics: DiagnosticRecord[];
    chapters: string[];
    assetCount: number;
    unresolvedAssets: string[];
};

/** Serialize with recursively sorted object keys and no extra whitespace. */
export function stableStringify(value: unknown): string {
    return JSON.stringify(sortValue(value));
}

/** Build the native-equivalent report JSON for a plain conversion. */
export function buildConvertReportJson(input: ConvertReportInput): string {
    const pipeline: Record<string, unknown> = {
        reader: input.from,
        writer: input.to,
        stages: ["read", "ir", "write"],
    };
    if (input.passes.length > 0) {
        pipeline.transforms = input.passes;
    }
    return stableStringify({
        schema_version: "panduck.report/v0",
        operation: "convert",
        status: input.status,
        inputs: [{ path: input.label, format: input.from }],
        pipeline,
        coverage: input.coverage,
        losses: input.losses,
        diagnostics: diagnosticEnvelope(input.diagnostics),
        outputs: input.status === "blocked" || input.status === "failed" ? [] : [{ format: input.to, published: true }],
    });
}

/** Build the native-equivalent report JSON for a Markdown project export. */
export function buildProjectReportJson(input: ProjectReportInput): string {
    const outputs: Array<Record<string, unknown>> = [
        { path: "index.md", format: "markdown", published: true },
        { path: "assets/", format: "assets", published_asset_count: input.assetCount },
    ];
    for (const chapter of input.chapters) {
        outputs.push({ path: chapter, format: "markdown", published: true });
    }
    outputs.push({ path: "panduck.report.json", format: "report", published: true });
    const pipeline: Record<string, unknown> = {
        reader: input.from,
        writer: "markdown-project",
        stages: ["read", "ir", "write", "materialize"],
    };
    if (input.passes.length > 0) {
        pipeline.transforms = input.passes;
    }
    return stableStringify({
        schema_version: "panduck.report/v0",
        operation: "convert_project",
        status: input.status,
        inputs: [{ path: input.label, format: input.from }],
        pipeline,
        coverage: input.coverage,
        losses: input.losses,
        diagnostics: diagnosticEnvelope(input.diagnostics),
        outputs,
        unresolved_assets: input.unresolvedAssets,
    });
}

function diagnosticEnvelope(diagnostics: DiagnosticRecord[]): Record<string, unknown> {
    return {
        diagnostics,
        dropped: 0,
        schemaVersion: 1,
        truncated: false,
    };
}

function sortValue(value: unknown): unknown {
    if (Array.isArray(value)) {
        return value.map(sortValue);
    }
    if (value !== null && typeof value === "object") {
        const sorted: Record<string, unknown> = {};
        for (const key of Object.keys(value).sort()) {
            sorted[key] = sortValue((value as Record<string, unknown>)[key]);
        }
        return sorted;
    }
    return value;
}
