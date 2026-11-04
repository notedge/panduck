import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

import {
    loadCoverageFeatures,
    loadCoverageManifest,
    type CoverageFeature,
    type CoverageFormatEntry,
} from "@notedge/panduck-fixtures";

import { probeConversion } from "./adapter.js";
import { resultsRoot } from "./corpus.js";
import type { LegObservation } from "./types.js";
import { runIdFromDate } from "./util.js";

/** Measured state of one authoritative-spec feature. */
export type CoverageStatus = "untested" | "passed" | "failed" | "rejected" | "silent-loss";

/** One measured feature row. */
export type CoverageFeatureResult = {
    readonly id: string;
    readonly spec: string;
    readonly semantics: string;
    readonly status: CoverageStatus;
    readonly detail: string;
    readonly lossCodes: readonly string[];
    readonly diagnosticCodes: readonly string[];
};

/** The status distribution of one format matrix. */
export type CoverageFormatResult = {
    readonly format: string;
    readonly spec: string;
    readonly total: number;
    readonly counts: Readonly<Record<CoverageStatus, number>>;
    readonly gap: string | null;
    readonly features: readonly CoverageFeatureResult[];
};

/** The full coverage report written by the `coverage` command. */
export type CoverageReport = {
    readonly schema: "panduck.harness.coverage";
    readonly runId: string;
    readonly startedAt: string;
    readonly formats: readonly CoverageFormatResult[];
    readonly totals: Readonly<Record<CoverageStatus, number>>;
    readonly silentLoss: readonly CoverageFeatureResult[];
};

function countOccurrences(haystack: string, needle: string): number {
    if (needle.length === 0) {
        return 0;
    }
    let count = 0;
    let index = haystack.indexOf(needle);
    while (index !== -1) {
        count += 1;
        index = haystack.indexOf(needle, index + needle.length);
    }
    return count;
}

function unmetExpectations(feature: CoverageFeature, output: string): readonly string[] {
    const probe = feature.probe;
    if (probe === null) {
        return [];
    }
    if (probe.unsupported === true) {
        return [];
    }
    const unmet: string[] = [];
    for (const marker of probe.contains ?? []) {
        if (!output.includes(marker)) {
            unmet.push(`missing ${JSON.stringify(marker)}`);
        }
    }
    for (const marker of probe.excludes ?? []) {
        if (output.includes(marker)) {
            unmet.push(`unexpected ${JSON.stringify(marker)}`);
        }
    }
    for (const [marker, expected] of Object.entries(probe.count ?? {})) {
        const actual = countOccurrences(output, marker);
        if (actual !== expected) {
            unmet.push(`count ${JSON.stringify(marker)} is ${actual}, expected ${expected}`);
        }
    }
    const order = probe.order ?? [];
    let previous = -1;
    for (const marker of order) {
        const index = output.indexOf(marker, previous + 1);
        if (index === -1 || index <= previous) {
            unmet.push(`order ${JSON.stringify(marker)} is out of place`);
            break;
        }
        previous = index;
    }
    return unmet;
}

/**
 * Loss codes that say "something was lost" without naming the construct.
 * They do not count as an accounting for a specific feature.
 */
const GENERIC_LOSS_CODES: ReadonlySet<string> = new Set([
    "import.html.empty_document",
    "import.html.partial_coverage",
]);

function specificAccounting(observation: LegObservation): boolean {
    const codes = [...observation.lossCodes, ...observation.diagnosticCodes];
    return codes.some((code) => !GENERIC_LOSS_CODES.has(code));
}

function measureFeature(
    feature: CoverageFeature,
    entry: CoverageFormatEntry,
    observation: LegObservation,
): CoverageFeatureResult {
    const base = {
        id: feature.id,
        spec: feature.spec,
        semantics: feature.semantics,
        lossCodes: observation.lossCodes,
        diagnosticCodes: observation.diagnosticCodes,
    };
    const reportedUnsupported = specificAccounting(observation);
    if (feature.probe?.unsupported === true) {
        return {
            ...base,
            status: reportedUnsupported ? "rejected" : "silent-loss",
            detail: reportedUnsupported
                ? `product declares the construct unsupported: ${observation.lossCodes.join(", ")}`
                : `product dropped the construct without reporting a loss`,
        };
    }
    if (observation.error !== undefined) {
        return { ...base, status: "untested", detail: `missing entry on ${entry.route?.from}:${entry.route?.to}: ${observation.error}` };
    }
    if (observation.exitCode !== 0 || observation.status === "failed") {
        return { ...base, status: "rejected", detail: `conversion failed with exit code ${observation.exitCode}` };
    }
    const output = observation.html ?? observation.text ?? "";
    const unmet = unmetExpectations(feature, output);
    if (unmet.length === 0) {
        return { ...base, status: "passed", detail: `satisfied ${entry.route?.from}:${entry.route?.to}` };
    }
    const silent = !reportedUnsupported;
    return {
        ...base,
        status: silent ? "silent-loss" : "failed",
        detail: unmet.join(" · "),
    };
}

function emptyCounts(): Record<CoverageStatus, number> {
    return { untested: 0, passed: 0, failed: 0, rejected: 0, "silent-loss": 0 };
}

/** Measures every feature in every matrix that a public route can drive. */
export function measureCoverage(only: string | null): CoverageReport {
    const manifest = loadCoverageManifest();
    const formats: CoverageFormatResult[] = [];
    const totals = emptyCounts();
    for (const entry of manifest.formats) {
        if (only !== null && only !== entry.format) {
            continue;
        }
        const counts = emptyCounts();
        const features = loadCoverageFeatures(entry.format).map((feature) => {
            if (entry.route === null) {
                const measured: CoverageFeatureResult = {
                    id: feature.id,
                    spec: feature.spec,
                    semantics: feature.semantics,
                    status: "untested",
                    detail: entry.gap ?? "no public route drives this format",
                    lossCodes: [],
                    diagnosticCodes: [],
                };
                counts.untested += 1;
                totals.untested += 1;
                return measured;
            }
            const observation = probeConversion(entry.route.from, entry.route.to, feature.source);
            const measured = measureFeature(feature, entry, observation);
            counts[measured.status] += 1;
            totals[measured.status] += 1;
            return measured;
        });
        formats.push({
            format: entry.format,
            spec: entry.spec,
            total: features.length,
            counts,
            gap: entry.gap ?? null,
            features,
        });
    }
    const silentLoss = formats.flatMap((format) => format.features.filter((feature) => feature.status === "silent-loss"));
    return {
        schema: "panduck.harness.coverage",
        runId: runIdFromDate(new Date()),
        startedAt: new Date().toISOString(),
        formats,
        totals,
        silentLoss,
    };
}

/** Writes the coverage report under `results/coverage/`. */
export function writeCoverage(report: CoverageReport): string {
    const directory = resolve(resultsRoot, "coverage");
    mkdirSync(directory, { recursive: true });
    const path = resolve(directory, `${report.runId}.json`);
    writeFileSync(path, `${JSON.stringify(report, null, 2)}\n`, "utf8");
    return path;
}

/** Prints the human readable status distribution and the P0 silent loss list. */
export function printCoverage(report: CoverageReport, archive: string): void {
    for (const format of report.formats) {
        const counts = format.counts;
        console.log(
            `${format.format} · ${format.total} features · 通过 ${counts.passed} · 失败 ${counts.failed} · 拒绝 ${counts.rejected} · 静默丢内容 ${counts["silent-loss"]} · 未测 ${counts.untested}`,
        );
        if (format.gap !== null) {
            console.log(`  gap ${format.gap}`);
        }
    }
    console.log(
        `totals · 通过 ${report.totals.passed} · 失败 ${report.totals.failed} · 拒绝 ${report.totals.rejected} · 静默丢内容 ${report.totals["silent-loss"]} · 未测 ${report.totals.untested}`,
    );
    if (report.silentLoss.length > 0) {
        console.log("P0 silent content loss:");
        for (const feature of report.silentLoss) {
            console.log(`  ${feature.id}: ${feature.detail}`);
        }
    }
    console.log(`archive ${archive}`);
}
