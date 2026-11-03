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
    diagnostics?: Array<{ code: string }>;
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

const STDOUT_FORMS: Array<{ label: string; args: string[] }> = [
    { label: "-o -", args: ["-o", "-"] },
    { label: "--output -", args: ["--output", "-"] },
    { label: "--output=-", args: ["--output=-"] },
];

test("cli publishes the converted text to stdout for every dash form without polluting the payload", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-stdout-"));
    const input = join(workdir, "source.md");
    const markdown = "# Stdout Title\n\nBody paragraph.\n";
    await writeFile(input, markdown, "utf8");

    try {
        for (const form of STDOUT_FORMS) {
            const run = runPanduck([
                "convert",
                input,
                "--from",
                "markdown",
                "--to",
                "markdown",
                ...form.args,
                "--diagnostics",
                "silent",
            ]);
            assert.equal(run.code, 0, `${form.label}: ${run.stderr}`);
            assert.equal(run.stdout.trimEnd(), markdown.trimEnd(), `${form.label} must carry the document only`);
            assert.ok(
                !run.stdout.includes("panduck.report/v0"),
                `${form.label}: no report may leak into the document payload`,
            );
        }
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("cli rejects an option token where the output value belongs", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-output-missing-value-"));
    const input = join(workdir, "source.md");
    await writeFile(input, "# Stdout Title\n\nBody paragraph.\n", "utf8");

    try {
        for (const args of [["--output", "--verbose"], ["--output", "-v"]]) {
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
            assert.match(run.stderr, /missing value for/, `${args.join(" ")} must report the missing value`);
            assert.match(run.stderr, /--output/);
            assert.equal(run.stdout, "", `${args.join(" ")} must not reach the document payload`);
        }
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

test("cli refuses to carry the document and the JSON report on stdout", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-stdout-json-"));
    const input = join(workdir, "source.md");
    await writeFile(input, "# Title\n\nBody.\n", "utf8");

    try {
        const run = runPanduck([
            "convert",
            input,
            "--from",
            "markdown",
            "--to",
            "html",
            "-o",
            "-",
            "--diagnostics",
            "json",
        ]);
        assert.equal(run.code, 2, `stream sharing must be an argument error: ${run.stderr}`);
        assert.equal(run.stdout, "", "neither the document nor the report may reach the stream");
        assert.match(run.stderr, /stdout cannot carry both/, "the refusal must explain the stream conflict");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("cli rejects a report path that would overwrite the converted document", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-report-collision-"));
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
            "-o",
            output,
            "--report",
            output,
            "--diagnostics",
            "silent",
        ]);
        assert.equal(run.code, 2, `a report/output collision must fail before conversion: ${run.stderr}`);
        assert.equal(await exists(output), false, "the collision must not publish either payload");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("cli rejects a report path that would overwrite the source document", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-report-input-collision-"));
    const input = join(workdir, "source.md");
    const markdown = "# Title\n\nBody.\n";
    await writeFile(input, markdown, "utf8");

    try {
        const run = runPanduck([
            "convert",
            input,
            "--from",
            "markdown",
            "--to",
            "html",
            "--report",
            input,
            "--diagnostics",
            "silent",
        ]);
        assert.equal(run.code, 2, `a report/input collision must fail before conversion: ${run.stderr}`);
        assert.equal(await readFile(input, "utf8"), markdown, "the source must keep its bytes");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("cli stops before converting when the input_bytes budget is exceeded", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-budget-input-"));
    const input = join(workdir, "source.md");
    const output = join(workdir, "source.html");
    const reportPath = join(workdir, "report.json");
    await writeFile(input, `# Title\n\n${"Body. ".repeat(40)}\n`, "utf8");

    try {
        const run = runPanduck([
            "convert",
            input,
            "--from",
            "markdown",
            "--to",
            "html",
            "-o",
            output,
            "--budget",
            "input_bytes=10",
            "--report",
            reportPath,
            "--diagnostics",
            "silent",
        ]);
        assert.equal(run.code, 7, `an exceeded budget must exit with 7: ${run.stderr}`);
        assert.equal(await exists(output), false, "a budget stop must not publish the output");
        const report = parseReport(await readFile(reportPath, "utf8"));
        assert.equal(report.status, "blocked");
        assert.match(report.pipeline?.blocked_reason ?? "", /input_bytes budget exceeded/);
        assert.equal(report.status_policy?.exit_code, 7);
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("cli maps a corrupt document to the parse failure exit code", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-corrupt-docx-"));
    const input = join(workdir, "broken.docx");
    const output = join(workdir, "broken.md");
    await writeFile(input, "this is not a zip container", "utf8");

    try {
        const run = runPanduck([
            "convert",
            input,
            "--from",
            "docx",
            "--to",
            "markdown",
            "-o",
            output,
            "--diagnostics",
            "json",
        ]);
        assert.equal(run.code, 4, `a corrupt container must exit with the parse code: ${run.stderr}`);
        const report = parseReport(run.stdout);
        assert.equal(report.status, "blocked");
        assert.equal(report.status_policy?.exit_code, 4);
        assert.ok(
            report.diagnostics?.some((item) => item.code.startsWith("notedown.format.")),
            `the native diagnostic must survive: ${run.stdout}`,
        );
        assert.equal(await exists(output), false, "a failed parse must not publish the output");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("cli converts standard input when the input path is a dash", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-stdin-"));
    const output = join(workdir, "from-stdin.html");

    try {
        const run = runPanduck(
            [
                "convert",
                "-",
                "--from",
                "markdown",
                "--to",
                "html",
                "-o",
                output,
                "--diagnostics",
                "silent",
            ],
            { stdin: "# Piped\n\nFrom standard input.\n" },
        );
        assert.equal(run.code, 0, `stdin conversion must succeed: ${run.stderr}`);
        const html = await readFile(output, "utf8");
        assert.match(html, /<h1[^>]*>Piped<\/h1>/);
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("cli fails closed instead of silently ignoring extra batch inputs", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-batch-"));
    const first = join(workdir, "one.md");
    const second = join(workdir, "two.md");
    await writeFile(first, "# One\n", "utf8");
    await writeFile(second, "# Two\n", "utf8");

    try {
        const run = runPanduck([
            "convert",
            first,
            second,
            "--from",
            "markdown",
            "--to",
            "html",
            "--batch",
            "--output-dir",
            workdir,
            "--diagnostics",
            "silent",
        ]);
        assert.equal(run.code, 8, `batch must fail closed until implemented: ${run.stderr}`);
        assert.match(run.stderr, /--batch is not implemented yet/);
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});
