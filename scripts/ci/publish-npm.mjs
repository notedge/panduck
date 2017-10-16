/**
 * Publish Panduck npm packages via OIDC Trusted Publisher.
 *
 * Contract: file=publish-npm.yml env=NPM_PUBLISH repo=notedge/panduck
 */

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const REPO_URL = "git+https://github.com/notedge/panduck.git";

const NATIVE_PLATFORMS = [
    {
        short: "win32-x64",
        triple: "win32-x64-msvc",
        os: ["win32"],
        cpu: ["x64"],
        fileName: "panduck-win32-x64-msvc.node",
        artifact: "index.win32-x64-msvc.node",
    },
    {
        short: "linux-x64",
        triple: "linux-x64-gnu",
        os: ["linux"],
        cpu: ["x64"],
        fileName: "panduck-linux-x64-gnu.node",
        artifact: "index.linux-x64-gnu.node",
    },
    {
        short: "linux-arm64",
        triple: "linux-arm64-gnu",
        os: ["linux"],
        cpu: ["arm64"],
        fileName: "panduck-linux-arm64-gnu.node",
        artifact: "index.linux-arm64-gnu.node",
    },
    {
        short: "darwin-x64",
        triple: "darwin-x64",
        os: ["darwin"],
        cpu: ["x64"],
        fileName: "panduck-darwin-x64.node",
        artifact: "index.darwin-x64.node",
    },
    {
        short: "darwin-arm64",
        triple: "darwin-arm64",
        os: ["darwin"],
        cpu: ["arm64"],
        fileName: "panduck-darwin-arm64.node",
        artifact: "index.darwin-arm64.node",
    },
];

/** @type {{ dir: string, publishName?: string }[]} */
const JS_PACKAGES = [
    { dir: "projects/packages/panduck-unknown-wasm32" },
    { dir: "projects/packages/panduck-skills" },
    { dir: "projects/packages/panduck", publishName: "@notedge/panduck" },
];

function fail(msg) {
    console.error(`ci-publish-npm: ${msg}`);
    process.exit(1);
}

function run(cmd, args, opts = {}) {
    const r = spawnSync(cmd, args, {
        cwd: opts.cwd ?? ROOT,
        encoding: "utf8",
        shell: process.platform === "win32",
        env: opts.env ?? process.env,
        stdio: opts.stdio ?? "pipe",
    });
    return {
        status: r.status ?? 1,
        stdout: String(r.stdout ?? "").trim(),
        stderr: String(r.stderr ?? "").trim(),
    };
}

function resolveVersion() {
    const fromArg = process.argv.find((a) => a.startsWith("--version="))?.slice("--version=".length);
    if (fromArg) return fromArg.replace(/^v/, "");
    const ref = process.env.GITHUB_REF ?? "";
    const m = ref.match(/^refs\/tags\/v?(\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?)$/);
    if (m) return m[1];
    fail("need --version=X.Y.Z or GITHUB_REF=refs/tags/vX.Y.Z");
}

function readJson(p) {
    return JSON.parse(fs.readFileSync(p, "utf8"));
}

function writeJson(p, obj) {
    fs.writeFileSync(p, `${JSON.stringify(obj, null, 2)}\n`);
}

function copyTree(src, dest) {
    fs.mkdirSync(dest, { recursive: true });
    for (const name of fs.readdirSync(src)) {
        if (name === "node_modules" || name === ".git") continue;
        const from = path.join(src, name);
        const to = path.join(dest, name);
        const st = fs.statSync(from);
        if (st.isDirectory()) copyTree(from, to);
        else {
            fs.mkdirSync(path.dirname(to), { recursive: true });
            fs.copyFileSync(from, to);
        }
    }
}

function rewriteWorkspaceDeps(deps, version) {
    if (!deps) return deps;
    const out = {};
    for (const [k, v] of Object.entries(deps)) {
        if (typeof v === "string" && (v.startsWith("workspace:") || v === "*")) {
            out[k] = version;
        } else {
            out[k] = v;
        }
    }
    return out;
}

function rewriteDepsField(pkg, version) {
    for (const field of ["dependencies", "optionalDependencies", "peerDependencies"]) {
        if (pkg[field]) pkg[field] = rewriteWorkspaceDeps(pkg[field], version);
    }
    return pkg;
}

function isAlreadyPublished(blob) {
    return /cannot publish over existing|EPUBLISHCONFLICT|previously published versions|version already exists|cannot publish.*same version|you cannot publish over/i.test(
        blob,
    );
}

function isAuthFailure(blob) {
    return /ENEEDAUTH|Unable to authenticate|not authorized|OIDC|trusted publisher|two-factor|need to be logged|login|identity token|do not have permission to access it|Access token expired or revoked/i.test(
        blob,
    );
}

function versionExists(name, version) {
    const r = run("npm", ["view", `${name}@${version}`, "version"]);
    return r.status === 0 && r.stdout === version;
}

function npmPublish(stagingDir, name, version) {
    const args = ["publish", "--access", "public"];
    console.log(`\n=== ${name}@${version} npm ${args.join(" ")} ===`);
    const r = run("npm", args, { cwd: stagingDir });
    if (r.stdout) process.stdout.write(`${r.stdout}\n`);
    if (r.stderr) process.stderr.write(`${r.stderr}\n`);
    const blob = `${r.stdout}\n${r.stderr}`;
    if (r.status === 0) return "published";
    if (isAlreadyPublished(blob) || versionExists(name, version)) return "exists";
    if (isAuthFailure(blob)) return "auth";
    console.error(blob.slice(0, 1200));
    return "other";
}

function stagePackageFiles(abs, stage) {
    const raw = readJson(path.join(abs, "package.json"));
    const files = Array.isArray(raw.files) && raw.files.length ? raw.files : null;
    fs.mkdirSync(stage, { recursive: true });
    if (files) {
        for (const f of files) {
            const from = path.join(abs, f);
            if (!fs.existsSync(from)) continue;
            const to = path.join(stage, f);
            if (fs.statSync(from).isDirectory()) copyTree(from, to);
            else {
                fs.mkdirSync(path.dirname(to), { recursive: true });
                fs.copyFileSync(from, to);
            }
        }
        for (const extra of ["package.json", "readme.md", "README.md", "LICENSE", "bin"]) {
            const from = path.join(abs, extra);
            if (!fs.existsSync(from)) continue;
            const to = path.join(stage, extra);
            if (fs.statSync(from).isDirectory()) copyTree(from, to);
            else fs.copyFileSync(from, to);
        }
    } else {
        copyTree(abs, stage);
    }
}

function publishNative(version, artifactsRoot) {
    let published = 0;
    let skipped = 0;
    for (const plat of NATIVE_PLATFORMS) {
        const name = `@notedge/panduck-${plat.short}`;
        const packageDir = path.join(ROOT, "projects/packages", `panduck-${plat.short}`);
        const artDir = path.join(artifactsRoot, plat.short);
        if (!fs.existsSync(artDir)) {
            console.log(` · ${name} no artifact (${plat.short}) — skip`);
            skipped += 1;
            continue;
        }
        if (versionExists(name, version)) {
            console.log(` ✓ ${name}@${version} already on registry — skip`);
            skipped += 1;
            continue;
        }
        const nodeFile = fs.readdirSync(artDir).find((f) => f.endsWith(".node"));
        if (!nodeFile) {
            fail(`${name}: artifact dir ${artDir} has no .node file`);
        }
        const stage = path.join(os.tmpdir(), `panduck-pub-native-${plat.short}-${version}`);
        fs.rmSync(stage, { recursive: true, force: true });
        fs.mkdirSync(path.join(stage, "lib"), { recursive: true });
        fs.copyFileSync(path.join(packageDir, "index.js"), path.join(stage, "index.js"));
        fs.copyFileSync(path.join(artDir, nodeFile), path.join(stage, "lib", plat.fileName));
        writeJson(path.join(stage, "package.json"), {
            name,
            version,
            description: `Panduck Node-API binary for ${plat.short}`,
            type: "module",
            license: "MPL-2.0",
            main: "./index.js",
            files: ["index.js", "lib"],
            os: plat.os,
            cpu: plat.cpu,
            publishConfig: { access: "public" },
            repository: { type: "git", url: REPO_URL },
        });
        const outcome = npmPublish(stage, name, version);
        if (outcome === "published") published += 1;
        else if (outcome === "exists") skipped += 1;
        else if (outcome === "auth") {
            fail(`OIDC/auth failed for ${name}. Configure Trusted Publisher: file=publish-npm.yml env=NPM_PUBLISH repo=notedge/panduck`);
        } else fail(`publish failed for ${name}`);
    }
    return { published, skipped };
}

function publishJs(version, artifactsRoot) {
    let published = 0;
    let skipped = 0;
    const optionalNatives = {};
    for (const plat of NATIVE_PLATFORMS) {
        const n = `@notedge/panduck-${plat.short}`;
        const artDir = path.join(artifactsRoot, plat.short);
        if (fs.existsSync(artDir) || versionExists(n, version)) {
            optionalNatives[n] = version;
        }
    }

    for (const spec of JS_PACKAGES) {
        const abs = path.join(ROOT, spec.dir);
        const raw = readJson(path.join(abs, "package.json"));
        const name = spec.publishName ?? raw.name;
        if (!name) fail(`no name for ${spec.dir}`);

        if (versionExists(name, version)) {
            console.log(` ✓ ${name}@${version} already on registry — skip`);
            skipped += 1;
            continue;
        }

        const stage = path.join(os.tmpdir(), `panduck-pub-js-${name.replace(/[/@]/g, "-")}-${version}`);
        fs.rmSync(stage, { recursive: true, force: true });
        stagePackageFiles(abs, stage);

        const pkg = rewriteDepsField({ ...raw }, version);
        pkg.name = name;
        pkg.version = version;
        delete pkg.private;
        pkg.publishConfig = { ...(pkg.publishConfig ?? {}), access: "public" };
        if (pkg.scripts) {
            delete pkg.scripts.prepack;
            delete pkg.scripts.prepare;
            if (Object.keys(pkg.scripts).length === 0) delete pkg.scripts;
        }
        if (!pkg.repository) {
            pkg.repository = { type: "git", url: REPO_URL };
        }
        if (name === "@notedge/panduck") {
            pkg.optionalDependencies = { ...(pkg.optionalDependencies ?? {}), ...optionalNatives };
        }
        delete pkg.devDependencies;
        writeJson(path.join(stage, "package.json"), pkg);

        const outcome = npmPublish(stage, name, version);
        if (outcome === "published") published += 1;
        else if (outcome === "exists") skipped += 1;
        else if (outcome === "auth") {
            fail(`OIDC/auth failed for ${name}. Configure Trusted Publisher: file=publish-npm.yml env=NPM_PUBLISH repo=notedge/panduck`);
        } else fail(`publish failed for ${name}`);
    }
    return { published, skipped };
}

const version = resolveVersion();
console.log(`ci-publish-npm: version=${version}`);
console.log(` GITHUB_REF=${process.env.GITHUB_REF ?? "(none)"}`);
console.log(" Trusted Publisher contract: publish-npm.yml + env NPM_PUBLISH\n");

delete process.env.NODE_AUTH_TOKEN;
delete process.env.NPM_TOKEN;

const artifactsRoot = process.env.PANDUCK_NATIVE_ARTIFACTS || path.join(ROOT, "dist", "native-flat");
const native = publishNative(version, artifactsRoot);
const js = publishJs(version, artifactsRoot);

console.log(
    `\nci-publish-npm: done (native published=${native.published} skipped=${native.skipped}; js published=${js.published} skipped=${js.skipped})`,
);
