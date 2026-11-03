import { execFileSync, execSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const cliDir = join(root, "projects/panduck.ts/packages/panduck");
const platformsDir = join(root, "projects/panduck.ts/platforms");

const PLATFORM_PACKAGES = {
    "win32-x64": "panduck-win32-x64",
    "linux-x64": "panduck-linux-x64",
    "linux-arm64": "panduck-linux-arm64",
    "darwin-x64": "panduck-darwin-x64",
    "darwin-arm64": "panduck-darwin-arm64",
};

/** pnpm and npm are `.cmd` shims on Windows, so every external tool runs through one shell line. */
function run(command, args, options = {}) {
    const line = [command, ...args].map((part) => (/\s/.test(part) ? `"${part}"` : part)).join(" ");
    try {
        return execSync(line, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"], ...options });
    } catch (error) {
        const out = `${error.stdout ?? ""}${error.stderr ?? ""}`;
        throw new Error(`${line} failed\n${out}`);
    }
}

function pack(packageDir, destination) {
    const before = new Set(readdirSync(destination));
    run("pnpm", ["pack", "--pack-destination", destination], { cwd: packageDir });
    const created = readdirSync(destination).filter((entry) => !before.has(entry) && entry.endsWith(".tgz"));
    if (created.length !== 1) {
        throw new Error(`expected one tarball from ${packageDir}, got ${created.join(", ") || "none"}`);
    }
    return join(destination, created[0]);
}

function main() {
    const platformKey = `${process.platform}-${process.arch}`;
    const platformPackage = PLATFORM_PACKAGES[platformKey];
    if (!platformPackage) {
        throw new Error(`no install check for platform ${platformKey}`);
    }

    const workdir = mkdtempSync(join(tmpdir(), "panduck-install-"));
    const packDir = join(workdir, "pack");
    const installDir = join(workdir, "app");
    mkdirSync(packDir, { recursive: true });
    mkdirSync(installDir, { recursive: true });

    const cliTarball = pack(cliDir, packDir);
    const platformTarball = pack(join(platformsDir, platformPackage), packDir);
    const wasmTarball = pack(join(platformsDir, "panduck-unknown-wasm32"), packDir);

    run("npm", ["install", "--no-audit", "--no-fund", "--ignore-scripts", cliTarball, platformTarball, wasmTarball], {
        cwd: installDir,
    });

    const cliBin = join(installDir, "node_modules/@notedge/panduck/bin/panduck");
    const version = JSON.parse(readFileSync(join(cliDir, "package.json"), "utf8")).version;

    const checks = [];
    const check = (name, fn) => checks.push({ name, fn });

    check("version reports the installed package version", () => {
        const out = run(process.execPath, [cliBin, "--version"]).trim();
        if (out !== version) {
            throw new Error(`expected version ${version}, got ${out}`);
        }
    });

    const source = join(workdir, "source.md");
    const output = join(workdir, "source.html");
    const reportPath = join(workdir, "report.json");
    writeFileSync(source, "# Installed\n\nReal conversion through the published tarball.\n", "utf8");

    check("a real conversion publishes the target and a versioned report", () => {
        run(process.execPath, [
            cliBin,
            "convert",
            source,
            "--from",
            "markdown",
            "--to",
            "html",
            "-o",
            output,
            "--report",
            reportPath,
            "--diagnostics",
            "silent",
        ]);
        const html = readFileSync(output, "utf8");
        if (!html.includes("Installed")) {
            throw new Error(`the published output is missing its heading: ${html.slice(0, 80)}`);
        }
        const report = JSON.parse(readFileSync(reportPath, "utf8"));
        if (report.schema_version !== "panduck.report/v0") {
            throw new Error(`unexpected report schema: ${String(report.schema_version)}`);
        }
        if (report.status !== "success") {
            throw new Error(`unexpected report status: ${String(report.status)}`);
        }
    });

    const broken = join(workdir, "broken.docx");
    const brokenOutput = join(workdir, "broken.md");
    writeFileSync(broken, "this is not a zip container", "utf8");

    check("a broken container exits with the parse failure code and publishes nothing", () => {
        const code = exitCodeOf(process.execPath, [cliBin, "convert", broken, "--from", "docx", "--to", "markdown", "-o", brokenOutput, "--diagnostics", "json"]);
        if (code !== 4) {
            throw new Error(`expected exit code 4 for a broken docx, got ${code}`);
        }
        assertMissing(brokenOutput, "a failed parse must not publish the output");
    });

    check("an exceeded budget exits with the resource code and publishes nothing", () => {
        const budgetOutput = join(workdir, "budget.html");
        const code = exitCodeOf(process.execPath, [
            cliBin,
            "convert",
            source,
            "--from",
            "markdown",
            "--to",
            "html",
            "-o",
            budgetOutput,
            "--budget",
            "input_bytes=1",
        ]);
        if (code !== 7) {
            throw new Error(`expected exit code 7 for an exceeded budget, got ${code}`);
        }
        assertMissing(budgetOutput, "a budget stop must not publish the output");
    });

    check("an undeclared flag exits with the argument code", () => {
        const code = exitCodeOf(process.execPath, [cliBin, "convert", source, "--profile", "office"]);
        if (code !== 2) {
            throw new Error(`expected exit code 2 for an unsupported flag, got ${code}`);
        }
    });

    const failures = [];
    for (const item of checks) {
        try {
            item.fn();
            console.log(`ok ${item.name}`);
        } catch (error) {
            failures.push(`${item.name}: ${error instanceof Error ? error.message : String(error)}`);
            console.error(`not ok ${item.name}`);
        }
    }

    rmSync(workdir, { recursive: true, force: true });

    if (failures.length > 0) {
        throw new Error(`${failures.length} install checks failed\n${failures.join("\n")}`);
    }
    console.log(`install check passed for ${platformKey}`);
}

function exitCodeOf(command, args) {
    try {
        execFileSync(command, args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
        return 0;
    } catch (error) {
        if (typeof error.status === "number") {
            return error.status;
        }
        throw error;
    }
}

function assertMissing(path, message) {
    try {
        readFileSync(path);
    } catch {
        return;
    }
    throw new Error(message);
}

main();
