import assert from "node:assert/strict";
import { access, mkdir, mkdtemp, readFile, readdir, rm, writeFile } from "node:fs/promises";
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
    losses?: Array<{ code: string }>;
    diagnostics?: Array<{ code: string }>;
    coverage?: { state: string; known?: string[]; unknown?: string[] } | null;
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
        assert.equal(await exists(outputDir), false, "a dry run must not create the batch output directory");
        assert.equal(await exists(join(outputDir, "a.html")), false, "a dry run must not write a batch output");
        assert.equal(await exists(join(outputDir, "b.html")), false, "a dry run must not write a batch output");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("batch publishes one markdown-project directory per input", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-batch-project-"));
    const outputDir = join(workdir, "site");
    const inputs = [join(workdir, "guide.md"), join(workdir, "notes.md")];
    await writeFile(inputs[0], "# Guide\n\nStep one.\n", "utf8");
    await writeFile(inputs[1], "# Notes\n\nRemember this.\n", "utf8");

    try {
        const run = runPanduck([
            "convert",
            ...inputs,
            "--from",
            "markdown",
            "--to",
            "markdown-project",
            "--output-dir",
            outputDir,
            "--diagnostics",
            "json",
        ]);
        assert.equal(run.code, 0, run.stderr || run.stdout);
        const report = JSON.parse(run.stdout) as BatchReport;
        assert.equal(report.operation, "convert-batch");
        assert.deepEqual(
            report.outputs?.map((output) => output.path).filter((path) => path.endsWith(".md")).sort(),
            [join(outputDir, "guide", "index.md"), join(outputDir, "notes", "index.md")].sort(),
            "each input owns its own project directory",
        );
        assert.equal(await readFile(join(outputDir, "guide", "index.md"), "utf8").then((text) => /Step one\./.test(text)), true);
        assert.equal(await readFile(join(outputDir, "notes", "index.md"), "utf8").then((text) => /Remember this\./.test(text)), true);
        assert.equal(await exists(join(outputDir, "guide", "panduck.report.json")), true, "each project keeps its own report");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("batch rejects two markdown-project inputs that share one directory name", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-batch-project-dupe-"));
    const outputDir = join(workdir, "site");
    const left = join(workdir, "left");
    const right = join(workdir, "right");
    await mkdir(left, { recursive: true });
    await mkdir(right, { recursive: true });
    await writeFile(join(left, "same.md"), "# Left\n", "utf8");
    await writeFile(join(right, "same.md"), "# Right\n", "utf8");

    try {
        const run = runPanduck([
            "convert",
            join(left, "same.md"),
            join(right, "same.md"),
            "--from",
            "markdown",
            "--to",
            "markdown-project",
            "--output-dir",
            outputDir,
            "--diagnostics",
            "silent",
        ]);
        assert.equal(run.code, 2, run.stderr);
        assert.match(run.stderr, /two batch inputs share one markdown-project output directory: same/);
        assert.equal(await exists(join(outputDir, "same")), false, "a rejected batch must publish nothing");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("profile flags reject with the v0 behavior instead of a bare flag list", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-profile-copy-"));
    const input = join(workdir, "doc.md");
    await writeFile(input, "# Doc\n", "utf8");

    try {
        const convert = runPanduck(["convert", input, "--from", "markdown", "--to", "html", "--profile", "print", "-o", join(workdir, "doc.html")]);
        assert.equal(convert.code, 2, convert.stderr);
        assert.match(convert.stderr, /these flags are not supported in v0: --profile/);
        assert.match(convert.stderr, /profile selection is not available in v0: conversions always use the built-in default profile/);

        const plan = runPanduck(["plan", input, "--from", "markdown", "--to", "html", "--config", "panduck.config.json"]);
        assert.equal(plan.code, 2, plan.stderr);
        assert.match(plan.stderr, /these flags are not supported in v0: --config/);
        assert.match(plan.stderr, /profile selection is not available in v0/);
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("batch aggregates mixed failure classes under the worst exit code and writes nothing for failed entries", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-batch-mixed-"));
    const outputDir = join(workdir, "out");
    await mkdir(outputDir, { recursive: true });
    const good = join(workdir, "good.md");
    const broken = join(workdir, "broken.docx");
    const unknown = join(workdir, "book.xlsx");
    await writeFile(good, "# Good\n\nBody.\n", "utf8");
    await writeFile(broken, "this is not a zip container", "utf8");
    await writeFile(unknown, "not a workbook", "utf8");

    try {
        const run = runPanduck([
            "convert",
            good,
            broken,
            unknown,
            "--to",
            "html",
            "--output-dir",
            outputDir,
            "--diagnostics",
            "json",
        ]);
        assert.equal(run.code, 4, `a parse failure outranks an unlisted route: ${run.stderr}`);
        const report = JSON.parse(run.stdout) as BatchReport;
        assert.equal(report.operation, "convert-batch");
        assert.equal(report.status, "blocked", "any failed entry blocks the aggregate status");
        assert.deepEqual(
            report.inputs?.map((input) => input.format ?? null),
            ["markdown", "docx", null],
            "the aggregate must keep every input once, in order",
        );
        assert.deepEqual(
            report.outputs?.map((output) => output.path),
            [join(outputDir, "good.html")],
            "only the readable entry publishes",
        );
        assert.deepEqual(
            report.diagnostics?.map((diagnostic) => diagnostic.code).sort(),
            ["notedown.format.invalid-input", "panduck.route.unlisted"],
            "failures aggregate into one flat diagnostic list",
        );
        assert.deepEqual(
            report.coverage,
            { state: "unavailable" },
            "one unavailable entry makes the batch route unavailable",
        );
        assert.equal(await exists(join(outputDir, "broken.html")), false, "the failed entry publishes nothing");
        assert.deepEqual(
            (await readdir(outputDir)).filter((name) => name.endsWith(".tmp")),
            [],
            "a failed entry must not leave a temp file behind",
        );
        assert.equal(await exists(join(outputDir, "book.html")), false, "the unlisted entry publishes nothing");
        assert.equal(report.status_policy?.exit_code, 4);
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("batch aggregates declared loss and still publishes every entry under the deny policy", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-batch-loss-"));
    const outputDir = join(workdir, "out");
    const inputs = [join(workdir, "first.html"), join(workdir, "second.html")];
    for (const path of inputs) {
        await writeFile(path, "<!doctype html><html><body><h1>Doc</h1><p>Body.</p></body></html>", "utf8");
    }

    try {
        const run = runPanduck([
            "convert",
            ...inputs,
            "--from",
            "html",
            "--to",
            "markdown",
            "--output-dir",
            outputDir,
            "--loss",
            "deny",
            "--diagnostics",
            "json",
        ]);
        assert.equal(run.code, 1, `the deny policy must win over success: ${run.stderr}`);
        const report = JSON.parse(run.stdout) as BatchReport;
        assert.equal(report.status, "success_with_loss");
        assert.equal(report.outputs?.length, 2, "every entry converts");
        assert.ok(report.outputs?.every((output) => output.published), "the deny policy still publishes");
        assert.ok(
            (report.losses?.length ?? 0) >= 2,
            "each lossy entry contributes its own declared loss",
        );
        assert.ok(
            report.losses?.some((loss) => loss.code === "import.html.partial_coverage"),
            "the aggregate must keep the upstream loss code",
        );
        assert.equal(report.status_policy?.exit_code, 1);
        assert.equal(await exists(join(outputDir, "first.md")), true);
        assert.equal(await exists(join(outputDir, "second.md")), true);
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("a batch that fails every entry leaves no output directory", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-batch-all-failed-"));
    const outputDir = join(workdir, "never-created");
    const broken = join(workdir, "broken.docx");
    await writeFile(broken, "this is not a zip container", "utf8");

    try {
        const run = runPanduck([
            "convert",
            broken,
            "--from",
            "docx",
            "--to",
            "markdown-project",
            "--output-dir",
            outputDir,
            "--diagnostics",
            "json",
        ]);
        assert.equal(run.code, 4, run.stderr);
        const report = JSON.parse(run.stdout) as BatchReport;
        assert.equal(report.status, "blocked");
        assert.deepEqual(report.outputs, [], "a failed entry publishes nothing");
        assert.equal(
            await exists(outputDir),
            false,
            "the site root is created by the first successful publish, not before the batch",
        );
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});
