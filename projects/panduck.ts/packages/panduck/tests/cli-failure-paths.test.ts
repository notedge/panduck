import assert from "node:assert/strict";
import { access, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { constants } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { runPanduck } from "./helpers/cli.js";

type CliReport = {
    operation: string;
    status: string;
    pipeline?: { blocked_reason?: string };
    outputs?: Array<{ path: string; published: boolean }>;
    losses?: Array<{ code: string }>;
    status_policy?: { loss: string; strict: boolean; exit_code: number };
};

function parseReport(stdout: string): CliReport {
    return JSON.parse(stdout) as CliReport;
}

async function exists(path: string): Promise<boolean> {
    try {
        await access(path, constants.F_OK);
        return true;
    } catch {
        return false;
    }
}

test("cli publishes the converted text to stdout without polluting the payload", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-stdout-"));
    const input = join(workdir, "source.md");
    const markdown = "# Stdout Title\n\nBody paragraph.\n";
    await writeFile(input, markdown, "utf8");

    try {
        const run = runPanduck([
            "convert",
            input,
            "--from",
            "markdown",
            "--to",
            "markdown",
            "--output=-",
            "--diagnostics",
            "silent",
        ]);
        assert.equal(run.code, 0, run.stderr);
        assert.equal(run.stdout.trimEnd(), markdown.trimEnd(), "stdout must carry the document only");
        assert.ok(!run.stdout.includes("panduck.report/v0"), "no report may leak into the document payload");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("cli refuses to overwrite an existing output and leaves it untouched", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-overwrite-deny-"));
    const input = join(workdir, "source.md");
    const output = join(workdir, "source.html");
    await writeFile(input, "# Title\n\nBody.\n", "utf8");
    await writeFile(output, "sentinel", "utf8");

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
        assert.equal(run.code, 6, `an existing target must exit with the output failure code: ${run.stderr}`);
        const report = parseReport(run.stdout);
        assert.equal(report.operation, "convert");
        assert.equal(report.status, "blocked");
        assert.equal(report.pipeline?.blocked_reason, "output already exists");
        assert.deepEqual(report.outputs, [], "a blocked publish must claim no outputs");
        assert.equal(report.status_policy?.exit_code, 6);
        assert.equal(await readFile(output, "utf8"), "sentinel", "the existing file must not be rewritten");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("cli overwrites the existing output when the overwrite flag is set", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-overwrite-allow-"));
    const input = join(workdir, "source.md");
    const output = join(workdir, "source.html");
    await writeFile(input, "# Title\n\nBody.\n", "utf8");
    await writeFile(output, "sentinel", "utf8");

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
            "--overwrite",
            "--diagnostics",
            "json",
        ]);
        assert.equal(run.code, 0, run.stderr || run.stdout);
        const report = parseReport(run.stdout);
        assert.equal(report.status, "success");
        assert.deepEqual(report.outputs, [{ path: output, published: true }]);
        assert.match(await readFile(output, "utf8"), /<h1[^>]*>Title<\/h1>/, "the overwrite must replace the file");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("cli blocks an unreadable input before touching the output path", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-missing-input-"));
    const missing = join(workdir, "absent.md");
    const output = join(workdir, "absent.html");

    try {
        const run = runPanduck([
            "convert",
            missing,
            "--from",
            "markdown",
            "--to",
            "html",
            "--output",
            output,
            "--diagnostics",
            "json",
        ]);
        assert.equal(run.code, 3, `a missing input must exit with the unsupported input code: ${run.stderr}`);
        const report = parseReport(run.stdout);
        assert.equal(report.status, "blocked");
        assert.equal(report.pipeline?.blocked_reason, "input is not readable");
        assert.equal(report.status_policy?.exit_code, 3);
        assert.equal(await exists(output), false, "a blocked read must not create the output");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("cli dry-run reports an unpublished output and writes nothing", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-dry-run-"));
    const input = join(workdir, "source.md");
    const output = join(workdir, "source.html");
    await writeFile(input, "# Title\n\nBody.\n", "utf8");

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
            "--dry-run",
            "--diagnostics",
            "json",
        ]);
        assert.equal(run.code, 0, run.stderr || run.stdout);
        const report = parseReport(run.stdout);
        assert.equal(report.status, "success");
        assert.deepEqual(report.outputs, [{ path: output, published: false }], "a dry run must not claim a publish");
        assert.equal(await exists(output), false, "a dry run must not write the output");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("cli deny policy fails a real lossy route while the report stays honest", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-deny-"));
    const input = join(workdir, "source.html");
    const output = join(workdir, "source.md");
    await writeFile(input, "<!doctype html><html><body><h1>D</h1><p>p</p></body></html>", "utf8");

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
            "--loss",
            "deny",
            "--diagnostics",
            "json",
        ]);
        assert.equal(run.code, 1, `the deny policy must exit with the loss code: ${run.stderr}`);
        const report = parseReport(run.stdout);
        assert.equal(report.status, "success_with_loss", "the conversion itself succeeded");
        assert.equal(report.status_policy?.loss, "deny");
        assert.equal(report.status_policy?.exit_code, 1);
        assert.ok(
            report.losses?.some((loss) => loss.code === "import.html.partial_coverage"),
            `the real loss must stay in the report: ${run.stdout}`,
        );
        assert.equal(await exists(output), true, "warn and deny both publish, only the exit code differs");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});
