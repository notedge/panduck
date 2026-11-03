import assert from "node:assert/strict";
import { mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { runPanduck } from "./helpers/cli.js";

const DIGRAPH_DOC = [
    "# DOT Doc",
    "",
    "```dot",
    "digraph G {",
    "  rankdir=LR",
    '  a [label="Start" shape=box]',
    '  b [label="Check" shape=box]',
    '  c [label="Stop" shape=box]',
    '  a -> b [label="go"]',
    '  b -> c [label="done"]',
    "}",
    "```",
    "",
    "Closing paragraph.",
    "",
].join("\n");

const REJECTED_DOC = [
    "# Rejected DOT",
    "",
    "```dot",
    "digraph G {",
    "  splines=ortho",
    "  a -> b",
    "}",
    "```",
    "",
].join("\n");

type CliReport = {
    status: string;
    losses: Array<{ code: string; message: string }>;
};

function parseCliReport(stdout: string): CliReport {
    return JSON.parse(stdout) as CliReport;
}

test("cli converts a dot fence into a real rendered svg figure", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-dot-"));
    const input = join(workdir, "dot.md");
    const output = join(workdir, "dot.html");
    await writeFile(input, DIGRAPH_DOC, "utf8");

    const run = runPanduck(["convert", input, "--from", "markdown", "--to", "html", "-o", output, "--diagnostics", "json"]);
    assert.equal(run.code, 0, run.stderr || run.stdout);

    const html = await readFile(output, "utf8");
    assert.match(html, /<figure class="diagram" data-diagram-language="dot"/, "the DOT fence must render as a figure");
    assert.match(html, /<svg[^>]*viewBox="/, "the figure must carry a real inline SVG scene");
    assert.match(html, /data-diagram-source-hex="[0-9a-f]+"/, "the authored source must ride along for round-trip");
    assert.match(html, /Closing paragraph\./);
    assert.ok(
        !html.includes('<pre class="diagram" data-diagram-language="dot"'),
        "a rendered DOT diagram must not keep the source fallback",
    );

    const report = parseCliReport(run.stdout);
    assert.equal(report.status, "success", `clean DOT render must stay loss-free: ${run.stdout}`);
    assert.deepEqual(report.losses, []);
});

test("cli keeps rejected dot attributes as visible source and reports the stable loss", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-dot-rejected-"));
    const input = join(workdir, "rejected.md");
    const output = join(workdir, "rejected.html");
    await writeFile(input, REJECTED_DOC, "utf8");

    const run = runPanduck(["convert", input, "--from", "markdown", "--to", "html", "-o", output, "--diagnostics", "json"]);
    assert.equal(run.code, 0, `warn policy publishes the fallback: ${run.stderr}`);

    const html = await readFile(output, "utf8");
    assert.match(html, /<pre class="diagram" data-diagram-language="dot"/, "a rejected DOT fence keeps the visible source block");
    assert.match(html, /splines=ortho/);
    assert.ok(!html.includes("<figure"), "no figure may be emitted for a rejected render");

    const report = parseCliReport(run.stdout);
    assert.equal(report.status, "success_with_loss");
    assert.ok(
        report.losses.some((loss) => loss.code === "davinci.diagram.unsupported"),
        `expected the DOT rejection code, got ${JSON.stringify(report.losses)}`,
    );

    const denied = runPanduck([
        "convert",
        input,
        "--from",
        "markdown",
        "--to",
        "html",
        "-o",
        join(workdir, "denied.html"),
        "--loss",
        "deny",
        "--diagnostics",
        "json",
    ]);
    assert.equal(denied.code, 1, "deny policy must fail the conversion on a rejected DOT fence");
});

test("dot diagrams survive the markdown to html to markdown roundtrip as diagrams", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-dot-roundtrip-"));
    const input = join(workdir, "dot.md");
    const html = join(workdir, "dot.html");
    const back = join(workdir, "back.md");
    await writeFile(input, DIGRAPH_DOC, "utf8");

    const toHtml = runPanduck(["convert", input, "--from", "markdown", "--to", "html", "-o", html, "--diagnostics", "silent"]);
    assert.equal(toHtml.code, 0, toHtml.stderr || toHtml.stdout);

    const toMarkdown = runPanduck(["convert", html, "--from", "html", "--to", "markdown", "-o", back, "--diagnostics", "silent"]);
    assert.equal(toMarkdown.code, 0, toMarkdown.stderr || toMarkdown.stdout);

    const markdown = await readFile(back, "utf8");
    assert.match(markdown, /```dot\n/, "the DOT fence language must survive the HTML leg");
    assert.match(markdown, /digraph G \{/);
    assert.match(markdown, /rankdir=LR/);
    assert.match(markdown, /a -> b \[label="go"\]/);

    const rerun = runPanduck([
        "convert",
        back,
        "--from",
        "markdown",
        "--to",
        "html",
        "-o",
        join(workdir, "back.html"),
        "--diagnostics",
        "silent",
    ]);
    assert.equal(rerun.code, 0, rerun.stderr || rerun.stdout);
    assert.match(
        await readFile(join(workdir, "back.html"), "utf8"),
        /<figure class="diagram" data-diagram-language="dot"/,
        "the reopened fence must render again as a diagram",
    );
});
