import { existsSync } from "node:fs";

import { readExpectationJson, readFixtureJson, readFixtureText, resolveExpectation, resolveFixture } from "./index.js";

/** One imported `document.formats` sample with the expectation its upstream test asserted. */
export type FormatCase = {
    readonly id: string;
    readonly path: string;
    readonly format: string;
    readonly container: "plain" | "package-part" | "xhtml";
    readonly classification: "import" | "export" | "inspect" | "roundtrip";
    readonly expect: Readonly<Record<string, unknown>>;
};

/** The `document.formats` corpus index at `fixtures/formats/manifest.json`. */
export type FormatManifest = {
    readonly schema: "panduck.fixtures.formats";
    readonly provenance: Readonly<Record<string, string>>;
    readonly cases: readonly FormatCase[];
};

/** One `plotter.chart` spec or rejection sample with its classification. */
export type ChartCase = {
    readonly id: string;
    readonly path: string;
    readonly language: "vega-lite";
    readonly classification: "supported" | "rejected";
    readonly expect: Readonly<Record<string, unknown>>;
};

/** One `document.ir` rule row: what the contract accepts or rejects. */
export type IrCase = {
    readonly id: string;
    readonly area: "inline" | "graph" | "contract";
    readonly classification: "accept" | "reject";
    readonly contract: string;
    readonly issues: readonly string[];
    readonly issueCount?: number;
};

/** One accepted diagram source that must lower and render. */
export type DiagramSpecCase = {
    readonly id: string;
    readonly language: "dot" | "mermaid" | "plantuml";
    readonly classification: "accept";
    readonly source: string;
    readonly expect: Readonly<Record<string, unknown>>;
};

/** One rejected diagram source with its documented code and authored span. */
export type DiagramRejectionCase = {
    readonly id: string;
    readonly language: "dot" | "mermaid" | "plantuml";
    readonly source: string;
    readonly code: string;
    readonly severity: "error" | "warning";
    readonly spanText: string;
};

/** One `plotter.runtime` pass derivation row. */
export type RuntimePassCase = {
    readonly id: string;
    readonly trigger: "action" | "scope";
    readonly action?: string;
    readonly scope?: string;
    readonly passes: readonly string[];
};

function objectRow(row: unknown, label: string): Record<string, unknown> {
    if (typeof row !== "object" || row === null) {
        throw new Error(`${label} must be an object`);
    }
    return row as Record<string, unknown>;
}

function stringField(row: Record<string, unknown>, key: string, label: string): string {
    const value = row[key];
    if (typeof value !== "string" || value.length === 0) {
        throw new Error(`${label}: \`${key}\` must be a non-empty string`);
    }
    return value;
}

function optionalStringField(row: Record<string, unknown>, key: string, label: string): string | undefined {
    const value = row[key];
    if (value === undefined) {
        return undefined;
    }
    if (typeof value !== "string" || value.length === 0) {
        throw new Error(`${label}: \`${key}\` must be a non-empty string`);
    }
    return value;
}

function stringArrayField(row: Record<string, unknown>, key: string, label: string): readonly string[] {
    const value = row[key];
    if (!Array.isArray(value) || value.some((entry) => typeof entry !== "string" || entry.length === 0)) {
        throw new Error(`${label}: \`${key}\` must be a string array`);
    }
    return value as readonly string[];
}

function expectObject(row: Record<string, unknown>, label: string): Readonly<Record<string, unknown>> {
    const value = row.expect;
    if (typeof value !== "object" || value === null || Array.isArray(value)) {
        throw new Error(`${label}: \`expect\` must be an object`);
    }
    return value as Readonly<Record<string, unknown>>;
}

function manifestCases(raw: unknown, schema: string, file: string): readonly unknown[] {
    const manifest = objectRow(raw, `manifest ${file}`);
    if (manifest.schema !== schema) {
        throw new Error(`unexpected manifest schema in ${file}: ${String(manifest.schema)}`);
    }
    if (!Array.isArray(manifest.cases) || manifest.cases.length === 0) {
        throw new Error(`${file} must carry at least one case`);
    }
    return manifest.cases;
}

function assertExists(path: string, label: string): void {
    if (!existsSync(path)) {
        throw new Error(`${label}: missing file: ${path}`);
    }
}

function uniqueIds<T extends { readonly id: string }>(rows: readonly T[], label: string): readonly T[] {
    const seen = new Set<string>();
    for (const row of rows) {
        if (seen.has(row.id)) {
            throw new Error(`${label}: duplicate case id \`${row.id}\``);
        }
        seen.add(row.id);
    }
    return rows;
}

/** Loads `fixtures/formats/manifest.json` and verifies every sample exists. */
export function loadFormatManifest(): FormatManifest {
    const file = "formats/manifest.json";
    const raw = readFixtureJson<FormatManifest>(file);
    const cases = manifestCases(raw, "panduck.fixtures.formats", file).map((entry, index) => {
        const row = objectRow(entry, `format case ${index}`);
        const id = stringField(row, "id", `format case ${index}`);
        const label = `format case \`${id}\``;
        const path = stringField(row, "path", label);
        assertExists(resolveFixture(`formats/${path}`), label);
        return {
            id,
            path,
            format: stringField(row, "format", label),
            container: stringField(row, "container", label) as FormatCase["container"],
            classification: stringField(row, "classification", label) as FormatCase["classification"],
            expect: expectObject(row, label),
        } satisfies FormatCase;
    });
    return { schema: "panduck.fixtures.formats", provenance: raw.provenance, cases: uniqueIds(cases, "formats") };
}

/** Reads one format sample as text through the guarded fixture resolver. */
export function readFormatText(entry: FormatCase): string {
    return readFixtureText(`formats/${entry.path}`);
}

/** Loads `fixtures/chart/manifest.json` and checks every rejected span sits in its source. */
export function loadChartCases(): readonly ChartCase[] {
    const file = "chart/manifest.json";
    const raw = readFixtureJson<unknown>(file);
    const cases = manifestCases(raw, "panduck.fixtures.chart", file).map((entry, index) => {
        const row = objectRow(entry, `chart case ${index}`);
        const id = stringField(row, "id", `chart case ${index}`);
        const label = `chart case \`${id}\``;
        const path = stringField(row, "path", label);
        const text = readFixtureText(`chart/${path}`);
        const classification = stringField(row, "classification", label);
        if (classification !== "supported" && classification !== "rejected") {
            throw new Error(`${label} has unsupported classification: ${classification}`);
        }
        const expect = expectObject(row, label);
        if (classification === "rejected") {
            const spanText = stringField(expect as Record<string, unknown>, "spanText", label);
            if (!text.includes(spanText)) {
                throw new Error(`${label}: \`spanText\` is absent from chart/${path}`);
            }
        }
        return { id, path, language: "vega-lite", classification, expect } satisfies ChartCase;
    });
    return uniqueIds(cases, "chart");
}

/** Loads `expectations/ir/cases.json`. */
export function loadIrCases(): readonly IrCase[] {
    const file = "ir/cases.json";
    const raw = readExpectationJson<unknown>(file);
    const cases = manifestCases(raw, "panduck.fixtures.ir", file).map((entry, index) => {
        const row = objectRow(entry, `ir case ${index}`);
        const id = stringField(row, "id", `ir case ${index}`);
        const label = `ir case \`${id}\``;
        const classification = stringField(row, "classification", label);
        if (classification !== "accept" && classification !== "reject") {
            throw new Error(`${label} has unsupported classification: ${classification}`);
        }
        const issues = stringArrayField(row, "issues", label);
        if (classification === "reject" && issues.length === 0) {
            throw new Error(`${label} is rejected but names no issue`);
        }
        const issueCount = row.issueCount;
        if (issueCount !== undefined && (typeof issueCount !== "number" || issueCount <= 0)) {
            throw new Error(`${label}: \`issueCount\` must be a positive number`);
        }
        return {
            id,
            area: stringField(row, "area", label) as IrCase["area"],
            classification,
            contract: stringField(row, "contract", label),
            issues,
            ...(issueCount === undefined ? {} : { issueCount }),
        } satisfies IrCase;
    });
    return uniqueIds(cases, "ir");
}

/** Loads `expectations/plotter/diagram/cases.json` and validates accepted and rejected rows. */
export function loadDiagramCases(): {
    readonly specs: readonly DiagramSpecCase[];
    readonly rejections: readonly DiagramRejectionCase[];
} {
    const file = "plotter/diagram/cases.json";
    const raw = objectRow(readExpectationJson<unknown>(file), `manifest ${file}`);
    if (raw.schema !== "panduck.fixtures.plotter.diagram") {
        throw new Error(`unexpected manifest schema in ${file}: ${String(raw.schema)}`);
    }
    const languages = new Set(["dot", "mermaid", "plantuml"]);
    const specsRaw = Array.isArray(raw.specs) ? raw.specs : [];
    const rejectionsRaw = Array.isArray(raw.rejections) ? raw.rejections : [];
    if (specsRaw.length === 0 || rejectionsRaw.length === 0) {
        throw new Error(`${file} must carry both accepted and rejected diagrams`);
    }
    const specs = uniqueIds(
        specsRaw.map((entry, index) => {
            const row = objectRow(entry, `diagram spec ${index}`);
            const id = stringField(row, "id", `diagram spec ${index}`);
            const label = `diagram spec \`${id}\``;
            const language = stringField(row, "language", label);
            if (!languages.has(language)) {
                throw new Error(`${label} has unsupported language: ${language}`);
            }
            return {
                id,
                language: language as DiagramSpecCase["language"],
                classification: "accept" as const,
                source: stringField(row, "source", label),
                expect: expectObject(row, label),
            } satisfies DiagramSpecCase;
        }),
        "diagram specs",
    );
    const rejections = uniqueIds(
        rejectionsRaw.map((entry, index) => {
            const row = objectRow(entry, `diagram rejection ${index}`);
            const id = stringField(row, "id", `diagram rejection ${index}`);
            const label = `diagram rejection \`${id}\``;
            const language = stringField(row, "language", label);
            if (!languages.has(language)) {
                throw new Error(`${label} has unsupported language: ${language}`);
            }
            const source = stringField(row, "source", label);
            const spanText = stringField(row, "spanText", label);
            if (!source.includes(spanText)) {
                throw new Error(`${label}: \`spanText\` is absent from its source`);
            }
            return {
                id,
                language: language as DiagramRejectionCase["language"],
                source,
                code: stringField(row, "code", label),
                severity: stringField(row, "severity", label) as DiagramRejectionCase["severity"],
                spanText,
            } satisfies DiagramRejectionCase;
        }),
        "diagram rejections",
    );
    return { specs, rejections };
}

/** Loads `expectations/plotter/runtime/passes.json`. */
export function loadRuntimePasses(): readonly RuntimePassCase[] {
    const file = "plotter/runtime/passes.json";
    const raw = readExpectationJson<unknown>(file);
    const cases = manifestCases(raw, "panduck.fixtures.plotter.runtime", file).map((entry, index) => {
        const row = objectRow(entry, `runtime case ${index}`);
        const id = stringField(row, "id", `runtime case ${index}`);
        const label = `runtime case \`${id}\``;
        const trigger = stringField(row, "trigger", label);
        if (trigger !== "action" && trigger !== "scope") {
            throw new Error(`${label} has unsupported trigger: ${trigger}`);
        }
        const passes = stringArrayField(row, "passes", label);
        if (passes.length === 0) {
            throw new Error(`${label} derives no pass`);
        }
        const action = optionalStringField(row, "action", label);
        const scope = optionalStringField(row, "scope", label);
        if (trigger === "action" && action === undefined) {
            throw new Error(`${label} is an action case and needs \`action\``);
        }
        if (trigger === "scope" && scope === undefined) {
            throw new Error(`${label} is a scope case and needs \`scope\``);
        }
        return { id, trigger, action, scope, passes } satisfies RuntimePassCase;
    });
    return uniqueIds(cases, "runtime");
}

/** Confirms every expectated file referenced by the corpus exists. */
export function verifyExpectation(relativePath: string): string {
    const path = resolveExpectation(relativePath);
    assertExists(path, `expectation ${relativePath}`);
    return path;
}
