import { fileURLToPath } from "node:url";

import {
    loadConvertManifest,
    loadNotedownManifest,
    readFixtureBytes,
    readFixtureJson,
    resolveExpectation,
} from "@notedge/panduck-fixtures";

import type { CaseDefinition, Expectation, LexedSnapshot, SnapshotCaseDefinition } from "./types.js";
import { readJsonFile } from "./util.js";

export const resultsRoot = fileURLToPath(new URL("../results/", import.meta.url));
export const baselinesRoot = fileURLToPath(new URL("../baselines/", import.meta.url));
export const packageRoot = fileURLToPath(new URL("../", import.meta.url));

/** Loads `convert/manifest.json` from the corpus package and verifies expectations exist. */
export function loadConversionCases(): readonly CaseDefinition[] {
    const loaded = loadConvertManifest();
    for (const entry of loaded.cases) {
        resolveExpectation(entry.expectation);
    }
    return loaded.cases;
}

/** Corpus grammars deliberately outside Panduck's format declarations. */
const EXCLUDED_GRAMMARS: Readonly<Record<string, string>> = {
    "oak-org-mode":
        "`oak-org-mode` is excluded by decision: Panduck declares `org` unavailable and exposes no org reader or lexer entry",
};

/** Loads the manifest driven lexer snapshot cases from `panduck-fixtures`. */
export function loadSnapshotCases(): readonly SnapshotCaseDefinition[] {
    const loaded = loadNotedownManifest();
    return loaded.cases.map((entry) => ({
        id: entry.id,
        kind: "lexer-snapshot" as const,
        title: `${entry.language} token snapshot`,
        input: `${loaded.directory}/${entry.source}`,
        snapshot: `${loaded.directory}/${entry.snapshot}`,
        language: grammarFor(entry.language),
        oracle: entry.oracle,
        comparison: entry.comparison,
        ...(EXCLUDED_GRAMMARS[entry.language] === undefined
            ? {}
            : { excluded: EXCLUDED_GRAMMARS[entry.language] }),
        ...(entry.snapshotLag === undefined ? {} : { snapshotLag: entry.snapshotLag }),
    }));
}

/**
 * Maps a corpus grammar label onto the product `lexDocument` language name.
 * Grammars Panduck does not read have no product entry, so they stay gaps
 * instead of getting an invented API.
 */
function grammarFor(language: string): string | null {
    switch (language) {
        case "oak-notedown":
            return "notedown";
        case "oak-markdown":
            return "markdown";
        case "oak-html":
            return "html";
        case "oak-xml":
            return "xml";
        default:
            return null;
    }
}

/**
 * Reads snapshot source text with LF endings.
 *
 * The checked out corpus may carry CRLF because of host git settings, while
 * every `.lexed.json` snapshot records byte offsets over LF text. Normalising
 * here keeps the oracle and the input byte-for-byte comparable; the harness
 * reports this normalisation in the case observation.
 */
export function snapshotSourceText(caseDefinition: SnapshotCaseDefinition): string {
    return Buffer.from(snapshotSource(caseDefinition)).toString("utf8").replace(/\r\n/g, "\n");
}

/** Reads source bytes for a snapshot case through the fixtures package. */
export function snapshotSource(caseDefinition: SnapshotCaseDefinition): Uint8Array {
    return readFixtureBytes(caseDefinition.input);
}

/** Reads the expected lexer token stream for a snapshot case. */
export function snapshotValue(caseDefinition: SnapshotCaseDefinition): LexedSnapshot {
    return readFixtureJson<LexedSnapshot>(caseDefinition.snapshot);
}

/** Reads one expectation file by case id. */
export function loadExpectation(caseId: string, fileName: string): Expectation {
    const path = resolveExpectation(fileName);
    const expectation = readJsonFile<Expectation>(path);
    if (expectation.schema !== "panduck.harness.expectation") {
        throw new Error(`unexpected expectation schema for \`${caseId}\`: ${expectation.schema}`);
    }
    if (expectation.caseId !== caseId) {
        throw new Error(`expectation file ${fileName} is for \`${expectation.caseId}\`, not \`${caseId}\``);
    }
    if (!Array.isArray(expectation.assertions) || expectation.assertions.length === 0) {
        throw new Error(`expectation for \`${caseId}\` must carry at least one assertion`);
    }
    if (!Array.isArray(expectation.provenance) || expectation.provenance.length === 0) {
        throw new Error(`expectation for \`${caseId}\` must carry provenance`);
    }
    return expectation;
}

/** Resolves the expectation file name for a conversion case. */
export function expectationFile(caseDefinition: CaseDefinition): string {
    return caseDefinition.expectation;
}
