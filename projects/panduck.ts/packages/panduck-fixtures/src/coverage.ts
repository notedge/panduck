import { readFixtureJson } from "./index.js";

/** One measurable expectation attached to a feature row. */
export type CoverageProbe = {
    readonly contains?: readonly string[];
    readonly excludes?: readonly string[];
    readonly order?: readonly string[];
    readonly count?: Readonly<Record<string, number>>;
    /** The product declares this construct unsupported, so a reported loss is a pass. */
    readonly unsupported?: boolean;
};

/** One authoritative-spec feature of an input format. */
export type CoverageFeature = {
    readonly id: string;
    /** Spec section or clause the feature comes from. */
    readonly spec: string;
    /** Semantic contract the sample is expected to satisfy. */
    readonly semantics: string;
    /** Minimal source carrying exactly this feature. */
    readonly source: string;
    /** Measurement rule; `null` when no product entry can drive the sample. */
    readonly probe: CoverageProbe | null;
};

/** Conversion route used to drive a format matrix. */
export type CoverageRoute = {
    readonly from: string;
    readonly to: string;
    readonly field: "html" | "text";
};

/** One matrix row of `fixtures/coverage/manifest.json`. */
export type CoverageFormatEntry = {
    readonly format: string;
    readonly spec: string;
    readonly route: CoverageRoute | null;
    readonly matrix: string;
    /** Why the format has no driving route; only set when `route` is null. */
    readonly gap?: string | null;
};

/** The coverage catalog that the harness `coverage` command reads. */
export type CoverageManifest = {
    readonly schema: string;
    readonly formats: readonly CoverageFormatEntry[];
};

/** Loads and validates the coverage catalog. */
export function loadCoverageManifest(): CoverageManifest {
    const manifest = readFixtureJson<CoverageManifest>("coverage/manifest.json");
    if (manifest.schema !== "panduck.coverage") {
        throw new Error(`unexpected coverage manifest schema: ${manifest.schema}`);
    }
    if (!Array.isArray(manifest.formats) || manifest.formats.length === 0) {
        throw new Error("coverage manifest must carry at least one format");
    }
    const seen = new Set<string>();
    for (const entry of manifest.formats) {
        if (seen.has(entry.format)) {
            throw new Error(`duplicate coverage format: ${entry.format}`);
        }
        seen.add(entry.format);
        if (!entry.matrix.startsWith(`${entry.format}/`)) {
            throw new Error(`coverage matrix for \`${entry.format}\` must live under \`${entry.format}/\``);
        }
    }
    return manifest;
}

/** Loads one format matrix and validates every feature row. */
export function loadCoverageFeatures(format: string): readonly CoverageFeature[] {
    const entry = loadCoverageManifest().formats.find((item) => item.format === format);
    if (entry === undefined) {
        throw new Error(`unknown coverage format: ${format}`);
    }
    const matrix = readFixtureJson<{ schema?: string; format?: string; features?: readonly CoverageFeature[] }>(
        `coverage/${entry.matrix}`,
    );
    if (matrix.schema !== "panduck.coverage.matrix" || matrix.format !== format) {
        throw new Error(`coverage matrix \`${entry.matrix}\` does not describe \`${format}\``);
    }
    const features = matrix.features;
    if (!Array.isArray(features) || features.length === 0) {
        throw new Error(`coverage matrix \`${entry.matrix}\` must carry at least one feature`);
    }
    const seen = new Set<string>();
    for (const feature of features) {
        if (seen.has(feature.id)) {
            throw new Error(`duplicate coverage feature: ${feature.id}`);
        }
        seen.add(feature.id);
        if (typeof feature.source !== "string" || feature.source.length === 0) {
            throw new Error(`coverage feature \`${feature.id}\` needs a minimal source`);
        }
        if ((feature.probe === null) === (entry.route !== null)) {
            throw new Error(
                `coverage feature \`${feature.id}\` must have a probe exactly when the format has a route`,
            );
        }
    }
    return features;
}
