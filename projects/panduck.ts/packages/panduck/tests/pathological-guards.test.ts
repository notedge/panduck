import assert from "node:assert/strict";
import { access, mkdtemp, rm, writeFile } from "node:fs/promises";
import { constants } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { runPanduck } from "./helpers/cli.js";

type GuardReport = {
    status: string;
    pipeline?: { blocked_reason?: string };
    outputs?: Array<{ path: string; published: boolean }>;
    diagnostics?: Array<{ code: string }>;
    status_policy?: { exit_code: number };
};

async function exists(path: string): Promise<boolean> {
    try {
        await access(path, constants.F_OK);
        return true;
    } catch {
        return false;
    }
}

function parseReport(stdout: string): GuardReport {
    return JSON.parse(stdout) as GuardReport;
}

test("cli refuses a remote URI input without fetching it or writing an output", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-remote-"));
    const output = join(workdir, "remote.md");
    const remote = "https://example.com/article.html";

    try {
        const run = runPanduck([
            "convert",
            remote,
            "--from",
            "html",
            "--to",
            "markdown",
            "--output",
            output,
            "--diagnostics",
            "json",
        ]);
        assert.equal(run.code, 3, `a remote URI must exit with the unsupported input code: ${run.stderr}`);
        const report = parseReport(run.stdout);
        assert.equal(report.status, "blocked");
        assert.equal(
            report.pipeline?.blocked_reason,
            "remote URI inputs are not fetched: panduck v0 performs no network requests, download the document first",
        );
        assert.ok(
            report.diagnostics?.some((entry) => entry.code === "panduck.input.remote-fetch-disabled"),
            `the rejection must carry a stable code: ${run.stdout}`,
        );
        assert.deepEqual(report.outputs, [], "a rejected remote input must claim no outputs");
        assert.equal(await exists(output), false, "panduck must not create any artifact for a remote input");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("cli rejects an HTML document nested beyond the built-in limit and writes nothing", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-deep-html-"));
    const input = join(workdir, "deep.html");
    const output = join(workdir, "deep.md");
    const depth = 5000;
    const html = `${"<div>".repeat(depth)}body${"</div>".repeat(depth)}`;
    await writeFile(input, `<!doctype html><html><body>${html}</body></html>`, "utf8");

    try {
        const run = runPanduck([
            "convert",
            input,
            "--from",
            "html",
            "--to",
            "markdown",
            "--output",
            output,
            "--diagnostics",
            "json",
        ]);
        assert.equal(run.code, 7, `deep nesting must exit with the resource code: ${run.stderr}`);
        const report = parseReport(run.stdout);
        assert.equal(report.status, "blocked");
        assert.ok(
            report.diagnostics?.some((entry) => entry.code.endsWith("budget-exhausted")),
            `the structural guard must report a budget diagnostic: ${run.stdout}`,
        );
        assert.deepEqual(report.outputs, [], "a blocked conversion must claim no outputs");
        assert.equal(await exists(output), false, "no output may be written after the guard rejects the input");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("cli rejects Markdown nested beyond the built-in limit and writes nothing", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-deep-markdown-"));
    const input = join(workdir, "deep.md");
    const output = join(workdir, "deep.html");
    const depth = 5000;
    await writeFile(input, `# Deep\n\n${"[".repeat(depth)}${"]".repeat(depth)}\n`, "utf8");

    try {
        const run = runPanduck([
            "convert",
            input,
            "--from",
            "markdown",
            "--to",
            "html",
            "--output",
            output,
            "--diagnostics",
            "json",
        ]);
        assert.equal(run.code, 7, `deep Markdown nesting must exit with the resource code: ${run.stderr}`);
        const report = parseReport(run.stdout);
        assert.ok(
            report.diagnostics?.some((entry) => entry.code.endsWith("budget-exhausted")),
            `the structural guard must report a budget diagnostic: ${run.stdout}`,
        );
        assert.equal(await exists(output), false, "no output may be written after the guard rejects the input");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("cli completes a large pathological input inside the harness timeout", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-large-"));
    const input = join(workdir, "wide.html");
    const output = join(workdir, "wide.md");
    const paragraphs = 40000;
    await writeFile(
        input,
        `<!doctype html><html><body>${"<p>paragraph text</p>".repeat(paragraphs)}</body></html>`,
        "utf8",
    );

    try {
        const run = runPanduck(
            [
                "convert",
                input,
                "--from",
                "html",
                "--to",
                "markdown",
                "--output",
                output,
                "--diagnostics",
                "silent",
            ],
            { timeoutMs: 90_000 },
        );
        assert.equal(
            run.code,
            0,
            `a wide but shallow input must not hang or trip the structural guard: ${run.stderr}`,
        );
        assert.equal(await exists(output), true, "a successful conversion must publish the target");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});
