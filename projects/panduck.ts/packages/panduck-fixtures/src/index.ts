import { existsSync, readFileSync } from "node:fs";
import { resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

/** One lexer conformance row from `notedown/manifest.json`. */
export type NotedownManifestCase = {
    readonly id: string;
    readonly source: string;
    readonly snapshot: string;
    readonly extension: string;
    readonly language: string;
    readonly oracle: string | null;
    readonly comparison: string;
    /** Set when the recorded snapshot predates a known lexer fix in its source repo. */
    readonly snapshotLag?: {
        readonly reason: string;
        readonly evidence: string;
    };
};

/** The `manifest.json` schema moved downstream from `notedown-fixtures`. */
export type NotedownManifest = {
    readonly schema: string;
    readonly provenance: Readonly<Record<string, string>>;
    readonly cases: readonly NotedownManifestCase[];
    readonly benchReferences: readonly string[];
};

/** A manifest row with absolute paths resolved inside this package. */
export type LoadedNotedownCase = NotedownManifestCase & {
    readonly sourcePath: string;
    readonly snapshotPath: string;
};

/** One conversion, roundtrip or rejection row from `convert/manifest.json`. */
export type ConvertManifestCase = {
    readonly id: string;
    readonly kind: "convert" | "roundtrip" | "rejection";
    readonly title: string;
    readonly input: string;
    readonly from: string;
    readonly to?: string;
    readonly via?: string;
    readonly expectation: string;
};

/** The conversion case catalog that every consumer harness reads. */
export type ConvertManifest = {
    readonly schema: string;
    readonly provenance: Readonly<Record<string, string>>;
    readonly cases: readonly ConvertManifestCase[];
};

/** Absolute root of the checked-in corpus. */
export const fixturesRoot = fileURLToPath(new URL("../fixtures/", import.meta.url));

/** Absolute root of the checked-in expectations. */
export const expectationsRoot = fileURLToPath(new URL("../expectations/", import.meta.url));

function assertInside(root: string, relativePath: string, resolved: string): string {
    const rootPrefix = resolve(root) + sep;
    if (!resolved.startsWith(rootPrefix)) {
        throw new Error(`fixture path escapes ${root}: ${relativePath}`);
    }
    return resolved;
}

function guardedResolve(root: string, relativePath: string, label: string): string {
    if (relativePath.length === 0 || relativePath.startsWith("/") || relativePath.includes("..")) {
        throw new Error(`${label}: invalid relative path: ${relativePath}`);
    }
    return assertInside(root, relativePath, resolve(root, relativePath));
}

/** Resolves a package-relative corpus path and rejects absolute or parent paths. */
export function resolveFixture(relativePath: string): string {
    return guardedResolve(fixturesRoot, relativePath, "fixture");
}

/** Resolves a package-relative expectation path and rejects absolute or parent paths. */
export function resolveExpectation(relativePath: string): string {
    return guardedResolve(expectationsRoot, relativePath, "expectation");
}

/** Reads an expectation JSON file with a narrow generic cast. */
export function readExpectationJson<T>(relativePath: string): T {
    return JSON.parse(readFileSync(resolveExpectation(relativePath), "utf8")) as T;
}

/** Reads a corpus file as bytes. */
export function readFixtureBytes(relativePath: string): Uint8Array {
    return new Uint8Array(readFileSync(resolveFixture(relativePath)));
}

/** Reads a corpus file as UTF-8 text. */
export function readFixtureText(relativePath: string): string {
    return readFileSync(resolveFixture(relativePath), "utf8");
}

/** Reads a corpus JSON file with a narrow generic cast. */
export function readFixtureJson<T>(relativePath: string): T {
    return JSON.parse(readFixtureText(relativePath)) as T;
}

function requiredString(row: Record<string, unknown>, key: string, label: string): string {
    const value = row[key];
    if (typeof value !== "string" || value.length === 0) {
        throw new Error(`${label}: \`${key}\` must be a non-empty string`);
    }
    return value;
}

function parseManifestCase(row: unknown, index: number): NotedownManifestCase {
    if (typeof row !== "object" || row === null) {
        throw new Error(`notedown manifest case ${index} must be an object`);
    }
    const record = row as Record<string, unknown>;
    const label = `notedown manifest case ${index}`;
    let snapshotLag: NotedownManifestCase["snapshotLag"];
    if (record.snapshotLag !== undefined) {
        if (typeof record.snapshotLag !== "object" || record.snapshotLag === null) {
            throw new Error(`${label}: \`snapshotLag\` must be an object`);
        }
        const lag = record.snapshotLag as Record<string, unknown>;
        snapshotLag = {
            reason: requiredString(lag, "reason", `${label} snapshotLag`),
            evidence: requiredString(lag, "evidence", `${label} snapshotLag`),
        };
    }
    return {
        id: requiredString(record, "id", label),
        source: requiredString(record, "source", label),
        snapshot: requiredString(record, "snapshot", label),
        extension: requiredString(record, "extension", label),
        language: requiredString(record, "language", label),
        oracle: record.oracle === null ? null : requiredString(record, "oracle", label),
        comparison: requiredString(record, "comparison", label),
        ...(snapshotLag === undefined ? {} : { snapshotLag }),
    };
}

/** Loads `notedown/manifest.json` and resolves each row against the corpus root. */
export function loadNotedownManifest(): {
    readonly directory: string;
    readonly manifest: NotedownManifest;
    readonly cases: readonly LoadedNotedownCase[];
} {
    const directory = "notedown";
    const manifest = readFixtureJson<NotedownManifest>(`${directory}/manifest.json`);
    if (manifest.schema !== "panduck.fixtures.notedown") {
        throw new Error(`unexpected notedown manifest schema: ${manifest.schema}`);
    }
    if (!Array.isArray(manifest.cases) || manifest.cases.length === 0) {
        throw new Error("notedown manifest must carry at least one case");
    }
    const cases = manifest.cases.map((row, index) => {
        const parsed = parseManifestCase(row, index);
        const sourcePath = resolveFixture(`${directory}/${parsed.source}`);
        const snapshotPath = resolveFixture(`${directory}/${parsed.snapshot}`);
        if (!existsSync(sourcePath)) {
            throw new Error(`missing corpus source for \`${parsed.id}\`: ${sourcePath}`);
        }
        if (!existsSync(snapshotPath)) {
            throw new Error(`missing corpus snapshot for \`${parsed.id}\`: ${snapshotPath}`);
        }
        return { ...parsed, sourcePath, snapshotPath };
    });
    return { directory, manifest, cases };
}

function parseConvertCase(row: unknown, index: number): ConvertManifestCase {
    if (typeof row !== "object" || row === null) {
        throw new Error(`convert manifest case ${index} must be an object`);
    }
    const record = row as Record<string, unknown>;
    const label = `convert manifest case ${index}`;
    const kind = record.kind;
    if (kind !== "convert" && kind !== "roundtrip" && kind !== "rejection") {
        throw new Error(`${label} has unsupported kind: ${String(kind)}`);
    }
    const id = requiredString(record, "id", label);
    const title = requiredString(record, "title", label);
    const input = requiredString(record, "input", label);
    const from = requiredString(record, "from", label);
    const expectation = requiredString(record, "expectation", label);
    if (kind === "roundtrip") {
        return { id, kind, title, input, from, expectation, via: requiredString(record, "via", label) };
    }
    return { id, kind, title, input, from, expectation, to: requiredString(record, "to", label) };
}

/** Loads `convert/manifest.json`, validates every row and confirms the input exists. */
export function loadConvertManifest(): {
    readonly directory: string;
    readonly manifest: ConvertManifest;
    readonly cases: readonly ConvertManifestCase[];
} {
    const directory = "convert";
    const manifest = readFixtureJson<ConvertManifest>(`${directory}/manifest.json`);
    if (manifest.schema !== "panduck.fixtures.convert") {
        throw new Error(`unexpected convert manifest schema: ${manifest.schema}`);
    }
    if (!Array.isArray(manifest.cases) || manifest.cases.length === 0) {
        throw new Error("convert manifest must carry at least one case");
    }
    const seen = new Set<string>();
    const cases = manifest.cases.map((row, index) => {
        const parsed = parseConvertCase(row, index);
        if (seen.has(parsed.id)) {
            throw new Error(`duplicate convert case id: ${parsed.id}`);
        }
        seen.add(parsed.id);
        if (parsed.input.includes("..") || parsed.input.startsWith("/")) {
            throw new Error(`convert case \`${parsed.id}\` has an unsafe input path`);
        }
        if (parsed.expectation.includes("..") || parsed.expectation.startsWith("/")) {
            throw new Error(`convert case \`${parsed.id}\` has an unsafe expectation path`);
        }
        if (!existsSync(resolveFixture(parsed.input))) {
            throw new Error(`missing corpus input for \`${parsed.id}\`: ${parsed.input}`);
        }
        return parsed;
    });
    return { directory, manifest, cases };
}

export * from "./corpus.js";
export * from "./coverage.js";
