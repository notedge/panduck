import { access } from "node:fs/promises";
import { constants } from "node:fs";

import type { PanduckBindings } from "../types.js";
import { loadPanduckNode } from "../node/load.js";
import { getFormatRegistry, findFormat, type FormatRecord } from "./format-registry.js";
import { extensionHints, inferTargetFromPath, isStdinPath, normalizeFormatId, pathsOverlap } from "./paths.js";
import { ExitCode } from "./exit-codes.js";
import type { SharedCliOptions } from "./options.js";
import { createReport, type PanduckReport } from "./report.js";

export type CliContext = {
    bindings: PanduckBindings | null;
    toolVersion: string;
    registry: FormatRecord[];
};

export function loadBindings(): PanduckBindings | null {
    try {
        return loadPanduckNode();
    } catch {
        return null;
    }
}

export function createContext(): CliContext {
    const bindings = loadBindings();
    return {
        bindings,
        toolVersion: bindings?.panduckVersion() ?? "0.0.0",
        registry: getFormatRegistry(bindings),
    };
}

export type ResolvedFormats = {
    from?: string;
    to?: string;
    ambiguous: boolean;
    hints: string[];
};

export function resolveFormats(
    inputPath: string,
    outputPath: string | undefined,
    shared: SharedCliOptions,
): ResolvedFormats {
    const hints = extensionHints(inputPath);
    const from = shared.from ? normalizeFormatId(shared.from) : hints.length === 1 ? hints[0] : undefined;
    const to =
        shared.to
            ? normalizeFormatId(shared.to)
            : outputPath && outputPath !== "-"
              ? inferTargetFromPath(outputPath)
              : undefined;
    const ambiguous = !shared.from && hints.length > 1;
    return { from, to, ambiguous, hints };
}

export async function assertReadableInput(path: string): Promise<void> {
    if (isStdinPath(path)) {
        return;
    }
    await access(path, constants.R_OK);
}

export function buildBlockedReport(
    ctx: CliContext,
    operation: string,
    inputPath: string,
    resolved: ResolvedFormats,
    reason: string,
    exitCode: number,
    shared: SharedCliOptions,
): PanduckReport {
    return createReport({
        operation,
        toolVersion: ctx.toolVersion,
        status: "blocked",
        inputs: [{ path: inputPath, format: resolved.from }],
        detection: {
            format: resolved.from,
            confidence: resolved.ambiguous ? "ambiguous" : resolved.from ? "hint" : undefined,
            hints: resolved.hints,
        },
        pipeline: { blocked_reason: reason },
        diagnostics: [{ owner: "panduck", severity: "error", message: reason }],
        determinism: { profile: shared.profile ?? "default", config: shared.config ?? null },
        statusPolicy: { loss: shared.loss, strict: shared.strict, exit_code: exitCode },
    });
}

export function validateIoPaths(inputPath: string, outputPath: string | undefined): number | null {
    if (outputPath && pathsOverlap(inputPath, outputPath)) {
        return ExitCode.InvalidArgs;
    }
    return null;
}
