import { appendFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";

import { resolve } from "node:path";

import { baselinesRoot, resultsRoot } from "./corpus.js";
import type { CaseObservation, CaseResult, RunReport } from "./types.js";
import { fileSegment, writeJsonFile } from "./util.js";

function summaryMarkdown(report: RunReport): string {
    const lines: string[] = [];
    lines.push(`# Panduck harness run ${report.runId}`);
    lines.push("");
    lines.push(`- suite: \`${report.suite}\``);
    lines.push(`- cases: ${report.totals.cases} pass ${report.totals.passed} fail ${report.totals.failed}`);
    lines.push(
        `- classifications: implementation ${report.classifications["implementation-defect"]} stale ${report.classifications["stale-expectation"]} undetermined ${report.classifications.undetermined}`,
    );
    lines.push(`- panduck: ${report.environment.panduckVersion}`);
    lines.push("");
    lines.push("| case | kind | status | category | first failure |");
    lines.push("|---|---|---|---|---|");
    for (const entry of report.cases) {
        const failure = entry.assertions.find((assertion) => assertion.status === "fail");
        lines.push(
            `| \`${entry.id}\` | ${entry.kind} | ${entry.status} | ${entry.category ?? "-"} | ${failure ? failure.detail.replace(/\|/g, "\\|") : "-"} |`,
        );
    }
    lines.push("");
    lines.push("## Gaps");
    for (const gap of report.gaps) {
        lines.push(`- \`${gap.id}\` (${gap.repo}): ${gap.neededEntry}`);
    }
    lines.push("");
    return lines.join("\n");
}

/** Writes a run report plus per-case observations into the local archive. */
export function writeRun(report: RunReport, observations: readonly CaseObservation[]): string {
    const runDirectory = resolve(resultsRoot, report.runId);
    mkdirSync(resolve(runDirectory, "observations"), { recursive: true });
    writeJsonFile(resolve(runDirectory, "report.json"), report);
    for (const observation of observations) {
        writeJsonFile(resolve(runDirectory, "observations", `${fileSegment(observation.caseId)}.json`), observation);
    }
    writeFileSync(resolve(runDirectory, "summary.md"), summaryMarkdown(report), "utf8");
    writeJsonFile(resolve(resultsRoot, "latest.json"), report);
    appendFileSync(
        resolve(resultsRoot, "history.jsonl"),
        `${JSON.stringify({
            runId: report.runId,
            suite: report.suite,
            cases: report.totals.cases,
            passed: report.totals.passed,
            failed: report.totals.failed,
            classifications: report.classifications,
        })}\n`,
        "utf8",
    );
    return runDirectory;
}

/** Copies the latest run report into a tracked baseline file. */
export function writeBaseline(name: string): string {
    const latest = resolve(resultsRoot, "latest.json");
    if (!existsSync(latest)) {
        throw new Error("no run archive exists yet, run the harness first");
    }
    mkdirSync(baselinesRoot, { recursive: true });
    const target = resolve(baselinesRoot, `${fileSegment(name)}.json`);
    writeFileSync(target, readFileSync(latest, "utf8"), "utf8");
    return target;
}

/** Finds the first failing assertion for a case result. */
export function firstFailure(entry: CaseResult): string {
    return entry.assertions.find((assertion) => assertion.status === "fail")?.detail ?? "-";
}
