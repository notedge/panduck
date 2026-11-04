import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { homedir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

/** One `oak-*` / `document-*` git dependency resolved from `Cargo.lock`. */
type GitDependency = {
    readonly name: string;
    readonly rev: string;
    readonly committedAt: number;
};

/** One local file that feeds the built binding. */
type LocalInput = {
    readonly path: string;
    readonly mtimeMs: number;
};

/** One reason the built binding cannot be trusted for the current revisions. */
export type FreshnessIssue = {
    readonly kind: "missing" | "stale" | "stale-local" | "unresolved";
    readonly detail: string;
};

export type FreshnessReport = {
    readonly repoRoot: string;
    readonly artifacts: readonly { readonly path: string; readonly mtimeMs: number }[];
    readonly dependencies: readonly GitDependency[];
    readonly localInputs: readonly LocalInput[];
    readonly issues: readonly FreshnessIssue[];
};

const ARTIFACT_ROOTS = "projects/panduck.ts/platforms";
const REBUILD_HINT = "rebuild with `node scripts/build-napi.mjs` before trusting a green run";

/** Finds the repository root from this module location. */
function findRepoRoot(): string {
    let current = fileURLToPath(new URL(".", import.meta.url));
    for (let depth = 0; depth < 8; depth += 1) {
        if (
            existsSync(join(current, "Cargo.lock")) &&
            existsSync(join(current, "projects/panduck.rs/panduck-napi/Cargo.toml"))
        ) {
            return current;
        }
        const parent = resolve(current, "..");
        if (parent === current) {
            break;
        }
        current = parent;
    }
    throw new Error("cannot locate the repository root from the harness module");
}

function parseGitSource(source: string): string | undefined {
    const pinned = /[?&]rev=([0-9a-f]{40})/.exec(source);
    if (pinned !== null) {
        return pinned[1];
    }
    const fragment = /#([0-9a-f]{40})/.exec(source);
    return fragment === null ? undefined : fragment[1];
}

/** Reads the pinned `oak-*` / `document-*` revisions out of `Cargo.lock`. */
function readPinnedRevisions(lockPath: string): readonly { readonly name: string; readonly rev: string }[] {
    const text = readFileSync(lockPath, "utf8");
    const blocks = text.split(/\[\[package\]\]/).slice(1);
    const pins: { name: string; rev: string }[] = [];
    for (const block of blocks) {
        const name = /^name = "([^"]+)"/m.exec(block);
        const source = /^source = "git\+([^"]+)"/m.exec(block);
        if (name === null || source === null) {
            continue;
        }
        if (!name[1].startsWith("oak-") && !name[1].startsWith("document-")) {
            continue;
        }
        const rev = parseGitSource(source[1]);
        if (rev !== undefined) {
            pins.push({ name: name[1], rev });
        }
    }
    return pins;
}

/** Resolves a revision's committer time through the local Cargo git database. */
function resolveCommittedAt(rev: string, dbRoot: string): number | undefined {
    if (!existsSync(dbRoot)) {
        return undefined;
    }
    for (const entry of readdirSync(dbRoot)) {
        const gitDir = join(dbRoot, entry);
        try {
            execFileSync("git", ["--git-dir", gitDir, "cat-file", "-e", `${rev}^{commit}`], { stdio: "ignore" });
        } catch {
            continue;
        }
        const seconds = Number(
            execFileSync("git", ["--git-dir", gitDir, "show", "-s", "--format=%ct", rev], { encoding: "utf8" }).trim(),
        );
        if (Number.isFinite(seconds) && seconds > 0) {
            return seconds * 1000;
        }
    }
    return undefined;
}

function platformArtifacts(repoRoot: string): readonly { readonly path: string; readonly mtimeMs: number }[] {
    const libDir = join(repoRoot, ARTIFACT_ROOTS, `panduck-${process.platform}-${process.arch}`, "lib");
    if (!existsSync(libDir)) {
        return [];
    }
    return readdirSync(libDir)
        .filter((entry) => entry.endsWith(".node"))
        .map((entry) => {
            const path = join(libDir, entry);
            return { path, mtimeMs: statSync(path).mtimeMs };
        });
}

/** Collects every `.rs` file under a directory, including subdirectories. */
function collectRustSources(dir: string, out: string[]): void {
    if (!existsSync(dir)) {
        return;
    }
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
        const path = join(dir, entry.name);
        if (entry.isDirectory()) {
            collectRustSources(path, out);
            continue;
        }
        if (entry.isFile() && entry.name.endsWith(".rs")) {
            out.push(path);
        }
    }
}

/** Local inputs that end up inside the `.node` artifact; TS sources load at runtime. */
function localInputs(repoRoot: string): readonly LocalInput[] {
    const rustSources: string[] = [];
    collectRustSources(join(repoRoot, "projects/panduck.rs/panduck-napi/src"), rustSources);
    const manifests = [
        "projects/panduck.rs/panduck-napi/Cargo.toml",
        "Cargo.toml",
        "Cargo.lock",
    ].map((relative) => join(repoRoot, relative));
    return [...rustSources, ...manifests]
        .filter((path) => existsSync(path))
        .map((path) => ({ path, mtimeMs: statSync(path).mtimeMs }));
}

function formatTime(ms: number): string {
    return new Date(ms).toISOString();
}

/**
 * Checks that every built binding is newer than the newest `oak-*` / `document-*`
 * revision pinned by `Cargo.lock` and than every local Rust input. A stale `.node`
 * silently turns a red corpus green.
 */
export function checkArtifactFreshness(): FreshnessReport {
    const repoRoot = findRepoRoot();
    const lockPath = join(repoRoot, "Cargo.lock");
    const dbRoot = join(process.env.CARGO_HOME ?? join(homedir(), ".cargo"), "git", "db");
    const issues: FreshnessIssue[] = [];
    const inputs = localInputs(repoRoot);
    if (!existsSync(lockPath)) {
        issues.push({ kind: "unresolved", detail: `missing \`Cargo.lock\` at ${lockPath}, run \`cargo fetch\` first` });
        return { repoRoot, artifacts: [], dependencies: [], localInputs: inputs, issues };
    }
    const pins = readPinnedRevisions(lockPath);
    const dependencies: GitDependency[] = [];
    for (const pin of pins) {
        if (dependencies.some((entry) => entry.rev === pin.rev)) {
            continue;
        }
        const committedAt = resolveCommittedAt(pin.rev, dbRoot);
        if (committedAt === undefined) {
            issues.push({
                kind: "unresolved",
                detail: `\`${pin.name}\` rev ${pin.rev} is not in the Cargo git database at ${dbRoot}, run \`cargo fetch\``,
            });
            continue;
        }
        dependencies.push({ name: pin.name, rev: pin.rev, committedAt });
    }
    const artifacts = platformArtifacts(repoRoot);
    if (artifacts.length === 0) {
        issues.push({
            kind: "missing",
            detail: `no \`.node\` artifact under \`${ARTIFACT_ROOTS}/panduck-${process.platform}-${process.arch}/lib\``,
        });
    }
    const newest = dependencies.reduce((latest, entry) => Math.max(latest, entry.committedAt), 0);
    for (const artifact of artifacts) {
        if (newest > 0 && artifact.mtimeMs <= newest) {
            const dependency = dependencies.find((entry) => entry.committedAt === newest)!;
            issues.push({
                kind: "stale",
                detail: `${artifact.path} was written ${formatTime(artifact.mtimeMs)} and is older than \`${dependency.name}\` rev ${dependency.rev} committed ${formatTime(dependency.committedAt)}`,
            });
        }
    }
    const newestInput = inputs.reduce<LocalInput | undefined>(
        (latest, entry) => (latest === undefined || entry.mtimeMs > latest.mtimeMs ? entry : latest),
        undefined,
    );
    if (newestInput !== undefined) {
        for (const artifact of artifacts) {
            if (artifact.mtimeMs <= newestInput.mtimeMs) {
                issues.push({
                    kind: "stale-local",
                    detail: `${artifact.path} was written ${formatTime(artifact.mtimeMs)} and is not newer than ${newestInput.path} modified ${formatTime(newestInput.mtimeMs)}`,
                });
            }
        }
    }
    return { repoRoot, artifacts, dependencies, localInputs: inputs, issues };
}

/** Renders the freshness verdict with the artifact, revision and rebuild hint. */
export function formatFreshnessReport(report: FreshnessReport): string {
    const lines = [`freshness ${report.issues.length === 0 ? "ok" : "failed"} · root ${report.repoRoot}`];
    for (const artifact of report.artifacts) {
        lines.push(`  artifact ${artifact.path} · ${formatTime(artifact.mtimeMs)}`);
    }
    for (const dependency of report.dependencies) {
        lines.push(`  dependency ${dependency.name} · ${dependency.rev.slice(0, 12)} · ${formatTime(dependency.committedAt)}`);
    }
    const newestInput = report.localInputs.reduce<LocalInput | undefined>(
        (latest, entry) => (latest === undefined || entry.mtimeMs > latest.mtimeMs ? entry : latest),
        undefined,
    );
    if (newestInput !== undefined) {
        lines.push(`  source ${newestInput.path} · ${formatTime(newestInput.mtimeMs)}`);
    }
    for (const issue of report.issues) {
        lines.push(`  ${issue.kind}: ${issue.detail}`);
    }
    if (report.issues.length > 0) {
        lines.push(`  ${REBUILD_HINT}`);
    }
    return lines.join("\n");
}
