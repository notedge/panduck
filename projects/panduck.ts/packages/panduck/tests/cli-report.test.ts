import assert from "node:assert/strict";
import { access, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { constants } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { runPanduck } from "./helpers/cli.js";

type CliReport = {
    schema_version: string;
    operation: string;
    status: string;
    pipeline?: { blocked_reason?: string };
    outputs?: Array<{ path: string; published: boolean }>;
};

async function exists(path: string): Promise<boolean> {
    try {
        await access(path, constants.F_OK);
        return true;
    } catch {
        return false;
    }
}

const SOURCE = "# Report Title\n\nBody paragraph.\n";

test("cli writes the report to the requested file on a successful convert", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-report-file-"));
    const input = join(workdir, "source.md");
    const output = join(workdir, "source.html");
    const reportPath = join(workdir, "nested", "report.json");
    await writeFile(input, SOURCE, "utf8");

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
            "--report",
            reportPath,
            "--diagnostics",
            "silent",
        ]);
        assert.equal(run.code, 0, run.stderr);
        assert.equal(run.stdout, "", "the report file must replace the stdout channel");
        const report = JSON.parse(await readFile(reportPath, "utf8")) as CliReport;
        assert.equal(report.schema_version, "panduck.report/v0");
        assert.equal(report.operation, "convert");
        assert.equal(report.status, "success");
        assert.deepEqual(report.outputs, [{ path: output, published: true }]);
        assert.equal(await exists(output), true, "the document itself must still be published");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("cli writes the blocked report to the file when the publish is refused", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-report-blocked-"));
    const input = join(workdir, "source.md");
    const output = join(workdir, "source.html");
    const reportPath = join(workdir, "report.json");
    await writeFile(input, SOURCE, "utf8");
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
            "--report",
            reportPath,
            "--diagnostics",
            "silent",
        ]);
        assert.equal(run.code, 6, run.stderr);
        const report = JSON.parse(await readFile(reportPath, "utf8")) as CliReport;
        assert.equal(report.status, "blocked");
        assert.equal(report.pipeline?.blocked_reason, "output already exists");
        assert.deepEqual(report.outputs, []);
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("cli sends the report to stdout for every dash form of the report path", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-report-stdout-"));
    const input = join(workdir, "source.md");
    await writeFile(input, SOURCE, "utf8");

    try {
        const forms: string[][] = [
            ["--report", "-"],
            ["--report=-"],
        ];
        for (const [index, args] of forms.entries()) {
            const output = join(workdir, `report-stdout-${index}.html`);
            const run = runPanduck([
                "convert",
                input,
                "--from",
                "markdown",
                "--to",
                "html",
                "--output",
                output,
                ...args,
                "--diagnostics",
                "silent",
            ]);
            assert.equal(run.code, 0, `${args.join(" ")}: ${run.stderr}`);
            const report = JSON.parse(run.stdout) as CliReport;
            assert.equal(report.schema_version, "panduck.report/v0");
            assert.equal(report.status, "success");
            assert.ok(
                !run.stdout.includes("<h1"),
                `${args.join(" ")}: the document payload must not share stdout with the report`,
            );
        }
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

const FIXED_TOP_LEVEL_KEYS = [
    "schema_version",
    "tool",
    "operation",
    "status",
    "inputs",
    "detection",
    "pipeline",
    "coverage",
    "losses",
    "diagnostics",
    "outputs",
    "budgets",
    "determinism",
    "status_policy",
];

test("cli emits the same report bytes for identical runs", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-report-deterministic-"));
    const input = join(workdir, "source.md");
    const output = join(workdir, "source.html");
    const firstReport = join(workdir, "first.json");
    const secondReport = join(workdir, "second.json");
    await writeFile(input, SOURCE, "utf8");

    try {
        for (const reportPath of [firstReport, secondReport]) {
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
                "--report",
                reportPath,
                "--diagnostics",
                "silent",
            ]);
            assert.equal(run.code, 0, run.stderr);
        }

        const first = await readFile(firstReport, "utf8");
        const second = await readFile(secondReport, "utf8");
        assert.equal(first, second, "identical inputs must produce byte-identical reports");

        const report = JSON.parse(first) as Record<string, unknown>;
        assert.deepEqual(
            Object.keys(report),
            FIXED_TOP_LEVEL_KEYS,
            "the fixed top-level keys must stay present and ordered",
        );
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("cli rejects a request that puts both the document and the report on stdout", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-report-conflict-"));
    const input = join(workdir, "source.md");
    await writeFile(input, SOURCE, "utf8");

    try {
        for (const args of [
            ["-o", "-", "--report=-"],
            ["--output", "-", "--report", "-"],
        ]) {
            const run = runPanduck([
                "convert",
                input,
                "--from",
                "markdown",
                "--to",
                "markdown",
                ...args,
                "--diagnostics",
                "silent",
            ]);
            assert.equal(run.code, 2, `${args.join(" ")} must be an argument error: ${run.stderr}`);
            assert.match(run.stderr, /stdout cannot carry both the document and the report/);
            assert.equal(run.stdout, "", `${args.join(" ")}: no partial payload may reach stdout`);
        }
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});
