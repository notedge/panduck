import { extname, resolve } from "node:path";

const EXTENSION_HINTS: Record<string, string[]> = {
    ".md": ["markdown"],
    ".markdown": ["markdown"],
    ".nd": ["notedown"],
    ".org": ["org"],
    ".rst": ["rst"],
    ".tex": ["tex"],
    ".html": ["html"],
    ".htm": ["html"],
    ".docx": ["docx"],
    ".epub": ["epub"],
    ".pdf": ["pdf"],
    ".doc": ["doc"],
};

export function isStdinPath(path: string | undefined): boolean {
    return path === "-";
}

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
