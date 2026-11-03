import assert from "node:assert/strict";
import { access, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { constants } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { runPanduck } from "./helpers/cli.js";

type BatchReport = {
    operation: string;
    status: string;
    inputs?: Array<{ path: string; format?: string }>;
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

test("batch converts every input into its own file and reports one aggregate", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-batch-"));
    const outputDir = join(workdir, "out");
    const inputs = [join(workdir, "one.md"), join(workdir, "two.md"), join(workdir, "three.md")];
    for (const [index, path] of inputs.entries()) {
        await writeFile(path, `# Doc ${index + 1}\n\nBody ${index + 1}.\n`, "utf8");
    }

    try {
        const run = runPanduck([
            "convert",
            ...inputs,
            "--to",
            "html",
            "--output-dir",
            outputDir,
            "--diagnostics",
            "json",
        ]);
        assert.equal(run.code, 0, run.stderr || run.stdout);
        const report = JSON.parse(run.stdout) as BatchReport;
        assert.equal(report.operation, "convert-batch");
        assert.equal(report.status, "success");
        assert.equal(report.inputs?.length, 3, "the aggregate must list every input");
        assert.deepEqual(
            report.outputs?.map((output) => output.path).sort(),
            [join(outputDir, "one.html"), join(outputDir, "three.html"), join(outputDir, "two.html")].sort(),
        );
        assert.ok(
            report.outputs?.every((output) => output.published),
            "a successful batch must publish every output",
        );
        for (const [index, name] of ["one", "two", "three"].entries()) {
            const html = await readFile(join(outputDir, `${name}.html`), "utf8");
            assert.match(html, new RegExp(`<h1[^>]*>Doc ${index + 1}</h1>`), `${name} must carry its own document`);
        }
        assert.equal(report.status_policy?.exit_code, 0);
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("batch keeps going after a failed input and returns the worst exit code", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-batch-partial-"));
    const outputDir = join(workdir, "out");
    const good = join(workdir, "good.md");
    const broken = join(workdir, "broken.docx");
    const alsoGood = join(workdir, "also-good.md");
    await writeFile(good, "# Good\n\nBody.\n", "utf8");
    await writeFile(alsoGood, "# Also good\n\nBody.\n", "utf8");
    await writeFile(broken, "this is not a zip container", "utf8");

    try {
        const run = runPanduck([
            "convert",
            good,
            broken,
            alsoGood,
            "--to",
            "html",
            "--output-dir",
            outputDir,
            "--diagnostics",
            "json",
        ]);
        assert.equal(run.code, 4, `the worst failure must win: ${run.stderr}`);
        const report = JSON.parse(run.stdout) as BatchReport;
        assert.equal(report.operation, "convert-batch");
        assert.equal(report.status, "blocked");
        assert.equal(report.inputs?.length, 3);
        assert.deepEqual(
            report.outputs?.map((output) => output.path).sort(),
            [join(outputDir, "also-good.html"), join(outputDir, "good.html")].sort(),
        );
        assert.ok(report.outputs?.every((output) => output.published), "the readable inputs must still publish");
        assert.equal(await exists(join(outputDir, "broken.html")), false, "the failed input must publish nothing");
        assert.ok(
            report.diagnostics?.some((diagnostic) => diagnostic.code.length > 0),
            "the aggregate must carry the failure diagnostics",
        );
        assert.equal(report.status_policy?.exit_code, 4);
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("batch refuses two inputs that map to the same output", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-batch-dupe-"));
    const outputDir = join(workdir, "out");
    const left = join(workdir, "left");
    const right = join(workdir, "right");
    await mkdir(left, { recursive: true });
    await mkdir(right, { recursive: true });
    const first = join(left, "same.md");
    const second = join(right, "same.md");
    await writeFile(first, "# Left\n", "utf8");
    await writeFile(second, "# Right\n", "utf8");

    try {
        const run = runPanduck([
            "convert",
            first,
            second,
            "--to",
            "html",
            "--output-dir",
            outputDir,
            "--diagnostics",
            "silent",
        ]);
        assert.equal(run.code, 2, `colliding names must be an argument error: ${run.stderr}`);
        assert.match(run.stderr, /two batch inputs map to the same output/);
        assert.equal(await exists(join(outputDir, "same.html")), false, "a rejected batch must publish nothing");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("batch dry-run reports every output as unpublished and writes nothing", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-batch-dry-"));
    const outputDir = join(workdir, "out");
    const inputs = [join(workdir, "a.md"), join(workdir, "b.md")];
    for (const path of inputs) {
        await writeFile(path, "# Doc\n", "utf8");
    }

    try {
        const run = runPanduck([
            "convert",
            ...inputs,
            "--from",
            "markdown",
            "--to",
            "html",
            "--output-dir",
            outputDir,
            "--dry-run",
            "--diagnostics",
            "json",
        ]);
        assert.equal(run.code, 0, run.stderr || run.stdout);
        const report = JSON.parse(run.stdout) as BatchReport;
        assert.ok(report.outputs?.every((output) => output.published === false));
        assert.equal(await exists(join(outputDir, "a.html")), false, "a dry run must not write a batch output");
        assert.equal(await exists(join(outputDir, "b.html")), false, "a dry run must not write a batch output");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});
