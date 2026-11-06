import { extname, resolve } from "node:path";

import { PANDUCK_DIAGNOSTIC_CODES } from "../diagnostic-codes.js";

const EXTENSION_HINTS: Record<string, string[]> = {
    ".md": ["markdown"],
    ".markdown": ["markdown"],
    ".note": ["notedown"],
    ".notedown": ["notedown"],
    ".org": ["org"],
    ".rst": ["rst"],
    ".tex": ["tex"],
    ".typ": ["typst"],
    ".typst": ["typst"],
    ".html": ["html"],
    ".htm": ["html"],
    ".docx": ["docx"],
    ".epub": ["epub"],
    ".pdf": ["pdf"],
    ".doc": ["doc"],
};

/** Matches an absolute URI with a scheme separator, including `file://`. */
const URI_PREFIX = /^[a-z][a-z0-9+.-]*:\/\//i;

export function isStdinPath(path: string | undefined): boolean {
    return path === "-";
}

/**
 * Whether the input names a remote (or otherwise non-filesystem) URI.
 *
 * Panduck v0 never performs network requests, so a URI reaching the CLI must
 * be rejected explicitly instead of surfacing a generic unreadable-path error.
 */
export function isRemoteUri(path: string): boolean {
    return URI_PREFIX.test(path.trim());
}

/** Human-readable reason for an input that cannot be read. */
export function inputReadFailureReason(path: string): string {
    return isRemoteUri(path)
        ? "remote URI inputs are not fetched: panduck v0 performs no network requests, download the document first"
        : "input is not readable";
}

/** Stable diagnostic code for a rejected remote URI input. */
export const REMOTE_INPUT_DIAGNOSTIC = PANDUCK_DIAGNOSTIC_CODES.remoteFetchDisabled;

export function isStdoutPath(path: string | undefined): boolean {
    return path === "-";
}

export function normalizeFormatId(value: string): string {
    const lowered = value.trim().toLowerCase();
    if (lowered === "ebook") {
        return "epub";
    }
    return lowered;
}

export function extensionHints(path: string): string[] {
    const ext = extname(path).toLowerCase();
    return EXTENSION_HINTS[ext] ? [...EXTENSION_HINTS[ext]] : [];
}

export function inferTargetFromPath(path: string): string | undefined {
    const hints = extensionHints(path);
    return hints.length === 1 ? hints[0] : undefined;
}

export function resolveFilesystemPath(path: string, cwd = process.cwd()): string {
    if (isStdinPath(path) || isStdoutPath(path)) {
        return path;
    }
    return resolve(cwd, path);
}

export function pathsOverlap(inputPath: string, outputPath: string, cwd = process.cwd()): boolean {
    if (isStdinPath(inputPath) || isStdoutPath(outputPath)) {
        return false;
    }
    return resolveFilesystemPath(inputPath, cwd) === resolveFilesystemPath(outputPath, cwd);
}
