/**
 * Markdown project materialization on the TS side.
 *
 * Mirrors the staging-directory semantics of `document-convert`'s publish layer
 * without exposing a library function that reads or writes paths. The whole
 * layout is written to a sibling staging directory first, the existing output
 * (if any) is moved aside, and only then is the staging directory renamed into
 * place. A failure before that rename restores the previous output, so a failed
 * publish never leaves a half-written project or destroys the readable one.
 *
 * Publishes to the same directory are serialized inside the process and guarded
 * by a sibling lock file across processes. The lock is short-lived and only
 * covers the write-and-swap window.
 */

import { mkdir, readFile, rename, rm, stat, writeFile } from "node:fs/promises";
import { basename, dirname, isAbsolute, join, resolve, sep } from "node:path";

export type PublishableProject = {
    indexMarkdown: string;
    chapters: Array<{ path: string; markdown: string }>;
    assets: Array<{ path: string; bytes: Uint8Array }>;
};

export type PublishedProject = {
    outputDir: string;
    indexPath: string;
    chapterPaths: string[];
    assetPaths: string[];
    reportPath: string;
};

/** Raised when another publisher holds the lock for the same output directory. */
export class PublishConflictError extends Error {
    readonly lockPath: string;

    constructor(outputDir: string, lockPath: string) {
        super(`output directory is locked by another publisher: ${outputDir}`);
        this.name = "PublishConflictError";
        this.lockPath = lockPath;
    }
}

/** Publishes that target the same resolved directory queue behind each other. */
const inFlightPublishes = new Map<string, Promise<unknown>>();

const RESERVED_PROJECT_PATHS = ["index.md", "panduck.report.json"];

let uniqueSequence = 0;

/** Materializes `index.md`, chapters, assets and the report into `outputDir`. */
export async function publishMarkdownProject(
    outputDir: string,
    project: PublishableProject,
    reportJson: string,
): Promise<PublishedProject> {
    const target = resolve(outputDir);
    validateProject(project);
    return serializePublish(target, () => publishExclusive(target, project, reportJson));
}

async function publishExclusive(
    outputDir: string,
    project: PublishableProject,
    reportJson: string,
): Promise<PublishedProject> {
    await mkdir(dirname(outputDir), { recursive: true });
    const lockPath = siblingPath(outputDir, "lock");
    await acquireLock(outputDir, lockPath);

    const stagingDir = siblingPath(outputDir, `${uniqueSuffix()}.staging`);
    let backupDir: string | undefined;
    let placed = false;
    try {
        await mkdir(stagingDir, { recursive: false });
        await writeProject(stagingDir, project, reportJson);
        backupDir = await moveAside(outputDir);
        await rename(stagingDir, outputDir);
        placed = true;
    } catch (error) {
        if (backupDir) {
            await rm(outputDir, { recursive: true, force: true }).catch(() => undefined);
            await rename(backupDir, outputDir);
            backupDir = undefined;
        }
        throw error;
    } finally {
        await rm(stagingDir, { recursive: true, force: true }).catch(() => undefined);
        if (placed && backupDir) {
            await rm(backupDir, { recursive: true, force: true }).catch(() => undefined);
        }
        await rm(lockPath, { force: true }).catch(() => undefined);
    }

    return {
        outputDir,
        indexPath: join(outputDir, "index.md"),
        chapterPaths: project.chapters.map((chapter) => chapter.path),
        assetPaths: project.assets.map((asset) => asset.path),
        reportPath: join(outputDir, "panduck.report.json"),
    };
}

async function writeProject(
    stagingDir: string,
    project: PublishableProject,
    reportJson: string,
): Promise<void> {
    await writeFile(join(stagingDir, "index.md"), project.indexMarkdown, "utf8");
    for (const chapter of project.chapters) {
        const chapterPath = resolveProjectPath(stagingDir, chapter.path);
        await mkdir(dirname(chapterPath), { recursive: true });
        await writeFile(chapterPath, chapter.markdown, "utf8");
    }
    for (const asset of project.assets) {
        const assetPath = resolveProjectPath(stagingDir, asset.path);
        await mkdir(dirname(assetPath), { recursive: true });
        await writeFile(assetPath, asset.bytes);
    }
    await writeFile(join(stagingDir, "panduck.report.json"), reportJson, "utf8");
}

/**
 * Rejects project layouts that would overwrite each other or the fixed outputs
 * before anything is written: reserved names, duplicate paths (case-insensitive
 * on Windows), and file/directory collisions such as `x` next to `x/y.md`.
 */
function validateProject(project: PublishableProject): void {
    const files = new Set<string>();
    const directories = new Set<string>();
    for (const reserved of RESERVED_PROJECT_PATHS) {
        files.add(projectPathKey(reserved));
    }
    for (const chapter of project.chapters) {
        reservePath(files, directories, chapter.path, "another project path");
    }
    for (const asset of project.assets) {
        assertSafeProjectPath(asset.path);
        if (!/^assets[\\/]/.test(asset.path)) {
            throw new Error(`asset path must start with assets/: ${asset.path}`);
        }
        reservePath(files, directories, asset.path, "another project path");
    }
}

function reservePath(files: Set<string>, directories: Set<string>, projectPath: string, kind: string): void {
    assertSafeProjectPath(projectPath);
    const segments = projectPath.split(/[\\/]+/);
    const key = projectPathKey(projectPath);
    if (RESERVED_PROJECT_PATHS.some((reserved) => projectPathKey(reserved) === key)) {
        throw new Error(`project path collides with ${kind}: ${projectPath}`);
    }
    if (files.has(key) || directories.has(key)) {
        throw new Error(`project path is declared twice: ${projectPath}`);
    }
    for (let index = 1; index < segments.length; index += 1) {
        const prefix = projectPathKey(segments.slice(0, index).join("/"));
        if (files.has(prefix)) {
            throw new Error(`project path collides with another project path: ${projectPath}`);
        }
    }
    files.add(key);
    for (let index = 1; index < segments.length; index += 1) {
        directories.add(projectPathKey(segments.slice(0, index).join("/")));
    }
}

/**
 * Rejects absolute paths, drive-letter and UNC paths, Windows alternate data
 * streams (`:` inside a segment), control characters and `.`/`..` segments. A
 * trailing dot or space segment is rejected because Windows silently rewrites
 * those names.
 */
function assertSafeProjectPath(projectPath: string): void {
    const escapes = () => {
        throw new Error(`project path escapes the output directory: ${projectPath}`);
    };
    if (projectPath.length === 0 || isAbsolute(projectPath) || /^[A-Za-z]:/.test(projectPath)) {
        escapes();
    }
    if (/[\u0000-\u001f\u007f]/.test(projectPath) || projectPath.includes(":")) {
        escapes();
    }
    const segments = projectPath.split(/[\\/]+/);
    if (segments.some((segment) => segment === "." || segment === ".." || segment.length === 0)) {
        escapes();
    }
    if (segments.some((segment) => /[. ]$/.test(segment))) {
        escapes();
    }
}

function projectPathKey(projectPath: string): string {
    const normalized = projectPath.split(/[\\/]+/).join("/");
    return process.platform === "win32" ? normalized.toLowerCase() : normalized;
}

/** Resolves a validated project path and asserts it stays inside the staging directory. */
function resolveProjectPath(stagingDir: string, projectPath: string): string {
    const resolved = resolve(stagingDir, projectPath);
    const root = resolve(stagingDir);
    if (resolved === root || !resolved.startsWith(root + sep)) {
        throw new Error(`project path escapes the output directory: ${projectPath}`);
    }
    return resolved;
}

function serializePublish<T>(key: string, run: () => Promise<T>): Promise<T> {
    const previous = inFlightPublishes.get(key) ?? Promise.resolve();
    const current = previous.then(run, run);
    inFlightPublishes.set(key, current);
    const settle = () => {
        if (inFlightPublishes.get(key) === current) {
            inFlightPublishes.delete(key);
        }
    };
    current.then(settle, settle);
    return current;
}

/**
 * Takes the sibling lock with an exclusive create. A lock left by a dead process
 * is treated as stale and replaced once, so a killed publisher does not block
 * later publishes. A live holder raises `PublishConflictError` instead of
 * waiting, because waiting on another process has no bounded completion.
 */
async function acquireLock(outputDir: string, lockPath: string): Promise<void> {
    if (await writeLock(lockPath)) {
        return;
    }
    if (await isStaleLock(lockPath)) {
        await rm(lockPath, { force: true }).catch(() => undefined);
        if (await writeLock(lockPath)) {
            return;
        }
    }
    throw new PublishConflictError(outputDir, lockPath);
}

async function writeLock(lockPath: string): Promise<boolean> {
    try {
        await writeFile(lockPath, JSON.stringify({ pid: process.pid, started_at: Date.now() }), {
            encoding: "utf8",
            flag: "wx",
        });
        return true;
    } catch (error) {
        if ((error as NodeJS.ErrnoException).code === "EEXIST") {
            return false;
        }
        throw error;
    }
}

async function isStaleLock(lockPath: string): Promise<boolean> {
    let pid: number | undefined;
    try {
        const parsed = JSON.parse(await readFile(lockPath, "utf8")) as { pid?: unknown };
        pid = typeof parsed.pid === "number" ? parsed.pid : undefined;
    } catch {
        pid = undefined;
    }
    if (pid === undefined || !Number.isInteger(pid) || pid <= 0) {
        return true;
    }
    try {
        process.kill(pid, 0);
        return false;
    } catch (error) {
        return (error as NodeJS.ErrnoException).code === "ESRCH";
    }
}

function siblingPath(outputDir: string, suffix: string): string {
    const name = basename(outputDir) || "output";
    return join(dirname(outputDir), `.${name}.${suffix}`);
}

function uniqueSuffix(): string {
    uniqueSequence += 1;
    return `${process.pid}.${process.hrtime.bigint()}.${uniqueSequence}`;
}

/** Renames the existing output aside, returning the backup path. */
async function moveAside(outputDir: string): Promise<string | undefined> {
    const backupDir = siblingPath(outputDir, `${uniqueSuffix()}.backup`);
    try {
        await rename(outputDir, backupDir);
        return backupDir;
    } catch (error) {
        if ((error as NodeJS.ErrnoException).code === "ENOENT" || !(await pathExists(outputDir))) {
            return undefined;
        }
        throw error;
    }
}

async function pathExists(target: string): Promise<boolean> {
    try {
        await stat(target);
        return true;
    } catch {
        return false;
    }
}
