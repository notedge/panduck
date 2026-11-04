import { realpathSync } from "node:fs";
import { createRequire } from "node:module";

import { loadPanduckNode, type PanduckBindings } from "@notedge/panduck";
import { readFixtureBytes } from "@notedge/panduck-fixtures";

import type {
    CaseObservation,
    CaseDefinition,
    CapabilitySnapshot,
    LegObservation,
    LexObservation,
} from "./types.js";
import { diagnosticCodes, lossCodes, parseReportJson, reportStatus, sha256 } from "./util.js";

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

function caseInputBytes(definition: CaseDefinition): Uint8Array {
    return readFixtureBytes(definition.input);
}

function labelFor(definition: CaseDefinition): string {
    return definition.input.split("/").pop() ?? definition.input;
}

function runConvertOrRejection(definition: CaseDefinition): CaseObservation {
    const started = performance.now();
    const to = definition.to ?? "";
    const leg = convertLeg(definition.from, to, labelFor(definition), caseInputBytes(definition));
    return {
        caseId: definition.id,
        kind: definition.kind,
        legs: [leg.observation],
        durationMs: Math.round(performance.now() - started),
        capabilities: capabilities(),
    };
}

function runRoundtrip(definition: CaseDefinition): CaseObservation {
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
export function runCase(definition: CaseDefinition): CaseObservation {
    return definition.kind === "roundtrip" ? runRoundtrip(definition) : runConvertOrRejection(definition);
}

/** Runs one format pair in memory so the coverage matrix can measure a feature. */
export function probeConversion(from: string, to: string, text: string): LegObservation {
    return convertLeg(from, to, `coverage.${from}`, Buffer.from(text, "utf8")).observation;
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
