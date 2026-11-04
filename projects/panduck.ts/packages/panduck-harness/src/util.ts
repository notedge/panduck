import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

/** Reads and parses a UTF-8 JSON file. */
export function readJsonFile<T>(path: string): T {
    return JSON.parse(readFileSync(path, "utf8")) as T;
}

/** Writes deterministic, reviewable JSON with a trailing newline. */
export function writeJsonFile(path: string, value: unknown): void {
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, `${JSON.stringify(value, null, 4)}\n`, "utf8");
}

/** SHA-256 hex digest for byte payloads and text. */
export function sha256(value: Uint8Array | string): string {
    return createHash("sha256").update(value).digest("hex");
}

/** UTC timestamp used in archive names. */
export function runIdFromDate(date: Date): string {
    return date.toISOString().replace(/[-:]/g, "").replace(/\.\d{3}Z$/, "Z");
}

/** Normalizes a case id into a safe archive file segment. */
export function fileSegment(value: string): string {
    return value.replace(/[^a-zA-Z0-9._-]/g, "_");
}

/** Parses a report JSON payload without throwing at the call site. */
export function parseReportJson(raw: string): unknown {
    try {
        return JSON.parse(raw) as unknown;
    } catch (error) {
        return { parseError: error instanceof Error ? error.message : String(error) };
    }
}

/** Extracts stable diagnostic codes from the two report shapes in use today. */
export function diagnosticCodes(report: unknown): string[] {
    if (typeof report !== "object" || report === null) {
        return [];
    }
    const record = report as Record<string, unknown>;
    const diagnostics = record.diagnostics;
    const rows = Array.isArray(diagnostics)
        ? diagnostics
        : typeof diagnostics === "object" && diagnostics !== null
          ? (diagnostics as Record<string, unknown>).diagnostics
          : undefined;
    if (!Array.isArray(rows)) {
        return [];
    }
    return rows
        .map((row) => (typeof row === "object" && row !== null ? (row as Record<string, unknown>).code : undefined))
        .filter((code): code is string => typeof code === "string");
}

/** Extracts loss codes from a Panduck report. */
export function lossCodes(report: unknown): string[] {
    if (typeof report !== "object" || report === null) {
        return [];
    }
    const losses = (report as Record<string, unknown>).losses;
    if (!Array.isArray(losses)) {
        return [];
    }
    return losses
        .map((row) => (typeof row === "object" && row !== null ? (row as Record<string, unknown>).code : undefined))
        .filter((code): code is string => typeof code === "string");
}

/** Reads a string field from a report. */
export function reportStatus(report: unknown, fallback = "failed"): string {
    if (typeof report === "object" && report !== null) {
        const status = (report as Record<string, unknown>).status;
        if (typeof status === "string") {
            return status;
        }
    }
    return fallback;
}
