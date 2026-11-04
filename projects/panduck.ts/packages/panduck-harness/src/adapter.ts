import { realpathSync } from "node:fs";
import { createRequire } from "node:module";

import { loadPanduckNode, type PanduckBindings } from "@notedge/panduck";
import { readFixtureBytes } from "@notedge/panduck-fixtures";

import type {
    CaseObservation,
    CaseDefinition,
    CapabilitySnapshot,
    ChartObservation,
    DiagramObservation,
    IrObservation,
    RuntimeObservation,
    LegObservation,
    LexObservation,
} from "./types.js";
import { diagnosticCodes, lossCodes, parseReportJson, reportStatus, sha256 } from "./util.js";
import { probePdfGeometry } from "./geometry.js";

type LegRun = {
    readonly observation: LegObservation;
    readonly binary?: Uint8Array;
};

let cachedBindings: PanduckBindings | undefined;

/** Loads the public Node-API binding once per process. */
export function bindings(): PanduckBindings {
    cachedBindings ??= loadPanduckNode();
    return cachedBindings;
}

/** Records the public capability advertisement and the resolved binding path. */
export function capabilities(): CapabilitySnapshot {
    const binding = bindings();
    return {
        version: binding.panduckVersion(),
        formats: binding.supportedFormats(),
        conversions: binding.supportedConversions?.() ?? [],
    };
}

/** Resolves the platform binding that Node actually loaded. */
export function bindingLocation(): { readonly path?: string; readonly insideRepo?: boolean } {
    try {
        const require = createRequire(import.meta.url);
        const panduckEntry = require.resolve("@notedge/panduck");
        const panduckRequire = createRequire(panduckEntry);
        const platformPackage = `@notedge/panduck-${process.platform}-${process.arch}`;
        const platformEntry = panduckRequire.resolve(platformPackage);
        const real = realpathSync(platformEntry);
        const normalized = real.replace(/\\/g, "/");
        return {
            path: real,
            insideRepo: normalized.includes("/projects/panduck.ts/platforms/"),
        };
    } catch {
        return {};
    }
}

function missingEntryObservation(from: string, to: string, label: string, detail: string): LegObservation {
    return {
        from,
        to,
        label,
        exitCode: 255,
        status: "failed",
        lossCodes: [],
        diagnosticCodes: [],
        error: `missing-entry: ${detail}`,
    };
}

function convertLeg(from: string, to: string, label: string, bytes: Uint8Array): LegRun {
    const convert = bindings().convertBytesMemory;
    if (typeof convert !== "function") {
        return {
            observation: missingEntryObservation(
                from,
                to,
                label,
                "`PanduckBindings.convertBytesMemory` is not exposed by this build",
            ),
        };
    }
    try {
        const response = convert(from, to, label, bytes);
        const report = parseReportJson(response.reportJson);
        const binary = response.binary ? new Uint8Array(response.binary) : undefined;
        return {
            observation: {
                from,
                to,
                label,
                exitCode: response.exitCode,
                status: reportStatus(report),
                lossCodes: lossCodes(report),
                diagnosticCodes: diagnosticCodes(report),
                text: response.text,
                html: response.html,
                binaryBytes: binary?.byteLength,
                binarySha256: binary ? sha256(binary) : undefined,
                reportSha256: sha256(response.reportJson),
            },
            binary,
        };
    } catch (error) {
        return {
            observation: {
                from,
                to,
                label,
                exitCode: 255,
                status: "failed",
                lossCodes: [],
                diagnosticCodes: [],
                error: error instanceof Error ? error.message : String(error),
            },
        };
    }
}

/**
 * Adds PDF geometry to a leg that produced a PDF.
 *
 * The probe returns the PDF and the layout runs from one binding call, so the
 * reserved widths and the painted bytes describe the same conversion. The
 * separate `convertBytesMemory` leg above supplies the report and loss codes.
 */
async function withPdfGeometry(
    definition: CaseDefinition,
    leg: LegRun,
): Promise<LegRun> {
    if (leg.observation.to !== "pdf" || leg.binary === undefined) {
        return leg;
    }
    const probe = bindings().pdfLayoutProbe;
    if (typeof probe !== "function") {
        return {
            observation: {
                ...leg.observation,
                error: "missing-entry: `PanduckBindings.pdfLayoutProbe` is not exposed by this build",
            },
        };
    }
    const response = probe(labelFor(definition), caseInputBytes(definition));
    if (response.exitCode !== 0 || response.pdf === undefined) {
        return {
            observation: {
                ...leg.observation,
                error: `pdf-layout-probe failed: ${response.error ?? "no PDF produced"}`,
            },
        };
    }
    const probeSha = sha256(response.pdf);
    if (leg.observation.binarySha256 !== undefined && leg.observation.binarySha256 !== probeSha) {
        return {
            observation: {
                ...leg.observation,
                error: `pdf-layout-probe rendered ${probeSha} but the report leg rendered ${leg.observation.binarySha256}`,
            },
        };
    }
    const geometry = await probePdfGeometry(response.pdf, response.runs);
    return {
        observation: {
            ...leg.observation,
            geometry: { pages: geometry.pages, glyphs: geometry.glyphs, runs: geometry.runs },
        },
    };
}

function caseInputBytes(definition: CaseDefinition): Uint8Array {
    return readFixtureBytes(definition.input);
}

function labelFor(definition: CaseDefinition): string {
    return definition.input.split("/").pop() ?? definition.input;
}

async function runConvertOrRejection(definition: CaseDefinition): Promise<CaseObservation> {
    const started = performance.now();
    const to = definition.to ?? "";
    const leg = await withPdfGeometry(
        definition,
        convertLeg(definition.from, to, labelFor(definition), caseInputBytes(definition)),
    );
    return {
        caseId: definition.id,
        kind: definition.kind,
        legs: [leg.observation],
        durationMs: Math.round(performance.now() - started),
        capabilities: capabilities(),
    };
}

async function runRoundtrip(definition: CaseDefinition): Promise<CaseObservation> {
    const started = performance.now();
    const via = definition.via ?? "";
    const forward = convertLeg("markdown", via, labelFor(definition), caseInputBytes(definition));
    if (!forward.binary) {
        return {
            caseId: definition.id,
            kind: definition.kind,
            legs: [
                forward.observation,
                missingEntryObservation(via, "markdown", `${labelFor(definition)}.back`, "forward leg produced no binary output"),
            ],
            durationMs: Math.round(performance.now() - started),
            capabilities: capabilities(),
        };
    }
    const back = convertLeg(via, "markdown", `${labelFor(definition)}.back`, Buffer.from(forward.binary));
    return {
        caseId: definition.id,
        kind: definition.kind,
        legs: [forward.observation, back.observation],
        durationMs: Math.round(performance.now() - started),
        capabilities: capabilities(),
    };
}

/** Runs a conversion, roundtrip or rejection definition through the public binding. */
export async function runCase(definition: CaseDefinition): Promise<CaseObservation> {
    return definition.kind === "roundtrip" ? runRoundtrip(definition) : runConvertOrRejection(definition);
}

/** Runs one format pair in memory so the coverage matrix can measure a feature. */
export function probeConversion(from: string, to: string, text: string): LegObservation {
    return convertLeg(from, to, `coverage.${from}`, Buffer.from(text, "utf8")).observation;
}

/** Renders a diagram source through the public `renderDiagram` entry. */
export function renderDiagramSource(language: string, source: string): DiagramObservation {
    const render = bindings().renderDiagram;
    if (typeof render !== "function") {
        return {
            success: false,
            svg: null,
            diagnostics: [],
            error: "missing-entry: `PanduckBindings.renderDiagram` is not exposed by this build",
        };
    }
    try {
        const response = render(language, source);
        return {
            success: response.success,
            svg: response.svg ?? null,
            diagnostics: response.diagnostics,
        };
    } catch (error) {
        return {
            success: false,
            svg: null,
            diagnostics: [],
            error: error instanceof Error ? error.message : String(error),
        };
    }
}

const PASS_ORDER = [
    "RecompileData",
    "RecomputeViewport",
    "Remeasure",
    "Relayout",
    "Restyle",
    "RetargetMotion",
    "RepaintOverlay",
] as const;

/** Derives the union of incremental passes for a set of runtime actions. */
export function runtimePassesForActions(actions: readonly string[]): RuntimeObservation {
    const derive = bindings().runtimePassesForAction;
    if (typeof derive !== "function") {
        return {
            passes: [],
            scopes: [],
            error: "missing-entry: `PanduckBindings.runtimePassesForAction` is not exposed by this build",
        };
    }
    const passes = new Set<string>();
    const scopes = new Set<string>();
    try {
        for (const action of actions) {
            const response = derive(action);
            if (response.error !== null && response.error !== undefined) {
                return { passes: [], scopes: [], error: response.error };
            }
            for (const pass of response.passes) {
                passes.add(pass);
            }
            for (const scope of response.scopes) {
                scopes.add(scope);
            }
        }
    } catch (error) {
        return {
            passes: [],
            scopes: [],
            error: error instanceof Error ? error.message : String(error),
        };
    }
    const ordered = PASS_ORDER.filter((pass) => passes.has(pass));
    return { passes: ordered, scopes: [...scopes] };
}

/** Derives incremental passes for one or more invalidation scopes. */
export function runtimePassesForScopes(scopes: readonly string[]): RuntimeObservation {
    const derive = bindings().runtimePassesForScopes;
    if (typeof derive !== "function") {
        return {
            passes: [],
            scopes: [],
            error: "missing-entry: `PanduckBindings.runtimePassesForScopes` is not exposed by this build",
        };
    }
    try {
        const response = derive([...scopes]);
        if (response.error !== null && response.error !== undefined) {
            return { passes: [], scopes: [], error: response.error };
        }
        return { passes: [...response.passes], scopes: [...response.scopes] };
    } catch (error) {
        return {
            passes: [],
            scopes: [],
            error: error instanceof Error ? error.message : String(error),
        };
    }
}

/** Renders a Vega-Lite spec through the public `renderChart` entry. */
export function renderChartSource(vegaLiteJson: string): ChartObservation {
    const render = bindings().renderChart;
    if (typeof render !== "function") {
        return {
            success: false,
            svg: null,
            diagnostics: [],
            error: "missing-entry: `PanduckBindings.renderChart` is not exposed by this build",
        };
    }
    try {
        const response = render(vegaLiteJson);
        return {
            success: response.success,
            svg: response.svg ?? null,
            diagnostics: response.diagnostics.map((diagnostic) => ({
                code: diagnostic.code,
                severity: diagnostic.severity,
                message: diagnostic.message,
                spanText: diagnostic.spanText,
                path: diagnostic.path,
            })),
        };
    } catch (error) {
        return {
            success: false,
            svg: null,
            diagnostics: [],
            error: error instanceof Error ? error.message : String(error),
        };
    }
}

/** Validates a serialized graph through the public `validateDocumentGraph` entry. */
export function validateGraphSource(graphJson: string): IrObservation {
    const validate = bindings().validateDocumentGraph;
    if (typeof validate !== "function") {
        return {
            valid: false,
            issues: [],
            error: "missing-entry: `PanduckBindings.validateDocumentGraph` is not exposed by this build",
        };
    }
    try {
        const response = validate(graphJson);
        return {
            valid: response.valid,
            issues: response.issues.map((issue) => ({
                code: issue.code,
                path: issue.path,
                detail: issue.detail,
            })),
            error: response.error ?? undefined,
        };
    } catch (error) {
        return {
            valid: false,
            issues: [],
            error: error instanceof Error ? error.message : String(error),
        };
    }
}

/** Lexes source text through the public `lexDocument` entry. */
export function lexSource(language: string, source: string): LexObservation {
    const lex = bindings().lexDocument;
    if (typeof lex !== "function") {
        return {
            success: false,
            tokens: [],
            diagnostics: [],
            error: "missing-entry: `PanduckBindings.lexDocument` is not exposed by this build",
        };
    }
    try {
        const response = lex(language, source);
        return {
            success: response.success,
            tokens: response.tokens,
            diagnostics: response.diagnostics,
        };
    } catch (error) {
        return {
            success: false,
            tokens: [],
            diagnostics: [],
            error: error instanceof Error ? error.message : String(error),
        };
    }
}
