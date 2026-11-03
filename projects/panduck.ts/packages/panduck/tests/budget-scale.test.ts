import assert from "node:assert/strict";
import { access, mkdtemp, rm, stat, writeFile } from "node:fs/promises";
import { constants } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { runPanduck } from "./helpers/cli.js";

const TARGET_BYTES = 4 * 1024 * 1024;

/** Builds a real document near the budget scale instead of a toy fixture. */
function buildLargeMarkdown(): string {
    const section = (index: number) => {
        const body = `Paragraph ${index} keeps enough prose to make the guard meaningful. `.repeat(12);
        return `# Section ${index}\n\n${body.trim()}\n\n`;
    };
    const parts: string[] = [];
    let size = 0;
    let index = 0;
    while (size < TARGET_BYTES) {
        const chunk = section(index);
        parts.push(chunk);
        size += Buffer.byteLength(chunk, "utf8");
        index += 1;
    }
    return parts.join("");
}

async function exists(path: string): Promise<boolean> {
    try {
        await access(path, constants.F_OK);
        return true;
    } catch {
        return false;
    }
}

test("input_bytes budget triggers on a real multi-megabyte document", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-budget-scale-"));
    const input = join(workdir, "large.md");
    const allowed = join(workdir, "allowed.html");
    const blocked = join(workdir, "blocked.html");
    const markdown = buildLargeMarkdown();
    await writeFile(input, markdown, "utf8");
    const inputSize = (await stat(input)).size;

    try {
        assert.ok(inputSize >= TARGET_BYTES, `the fixture must reach the real scale, got ${inputSize} bytes`);

        const within = runPanduck([
            "convert",
            input,
            "--from",
            "markdown",
            "--to",
            "html",
            "-o",
            allowed,
            "--budget",
            `input_bytes=${inputSize + 1}`,
            "--diagnostics",
            "silent",
        ]);
        assert.equal(within.code, 0, withDiagnostics(within));
        assert.ok((await stat(allowed)).size > 0, "a document under the limit must publish a real output");

        const over = runPanduck([
            "convert",
            input,
            "--from",
            "markdown",
            "--to",
            "html",
            "-o",
            blocked,
            "--budget",
            `input_bytes=${inputSize - 1}`,
            "--diagnostics",
            "json",
        ]);
        assert.equal(over.code, 7, withDiagnostics(over));
        assert.equal(await exists(blocked), false, "a budget stop must not publish on real inputs either");
        const report = JSON.parse(over.stdout) as { status: string; pipeline?: { blocked_reason?: string } };
        assert.equal(report.status, "blocked");
        assert.match(report.pipeline?.blocked_reason ?? "", /input_bytes budget exceeded/);
        assert.ok(
            (report.pipeline?.blocked_reason ?? "").includes(`${inputSize}`),
            "the diagnostic must report the observed size",
        );
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("output_bytes budget triggers on the real converted size", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-budget-output-scale-"));
    const input = join(workdir, "large.md");
    const baseline = join(workdir, "baseline.html");
    const blocked = join(workdir, "blocked.html");
    await writeFile(input, buildLargeMarkdown(), "utf8");

    try {
        const first = runPanduck([
            "convert",
            input,
            "--from",
            "markdown",
            "--to",
            "html",
            "-o",
            baseline,
            "--diagnostics",
            "silent",
        ]);
        assert.equal(first.code, 0, withDiagnostics(first));
        const outputSize = (await stat(baseline)).size;
        assert.ok(outputSize > 0);

        const over = runPanduck([
            "convert",
            input,
            "--from",
            "markdown",
            "--to",
            "html",
            "-o",
            blocked,
            "--budget",
            `output_bytes=${outputSize - 1}`,
            "--diagnostics",
            "json",
        ]);
        assert.equal(over.code, 7, withDiagnostics(over));
        assert.equal(await exists(blocked), false, "an exceeded output budget must not publish the result");
        const report = JSON.parse(over.stdout) as { status: string; pipeline?: { blocked_reason?: string } };
        assert.equal(report.status, "blocked");
        assert.match(report.pipeline?.blocked_reason ?? "", /output_bytes budget exceeded/);
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

function withDiagnostics(result: { code: number; stdout: string; stderr: string }): string {
    return `exit ${result.code}: ${result.stderr || result.stdout}`;
}
