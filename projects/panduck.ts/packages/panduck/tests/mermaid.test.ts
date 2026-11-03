import assert from "node:assert/strict";
import { mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { Converter } from "../src/converter.js";
import { defineConversion } from "../src/conversion-plan.js";
import { loadPanduckNode } from "../src/node/load.js";
import { runPanduck } from "./helpers/cli.js";

const FLOWCHART_DOC = [
    "# Diagram Doc",
    "",
    "```mermaid",
    "flowchart TD",
    "  A[Start] --> B{Done?}",
    "  B -->|yes| C[Stop]",
    "```",
    "",
    "Closing paragraph.",
    "",
].join("\n");

const BROKEN_DOC = ["# Broken Diagram", "", "```mermaid", "this is not a mermaid diagram at all", "```", ""].join("\n");

type CliReport = {
    status: string;
    losses: Array<{ code: string }>;
};

function parseCliReport(stdout: string): CliReport {
    return JSON.parse(stdout) as CliReport;
}

test("cli converts a mermaid fence into a real rendered svg figure", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-mermaid-"));
    const input = join(workdir, "diagram.md");
    const output = join(workdir, "diagram.html");
    await writeFile(input, FLOWCHART_DOC, "utf8");

    const run = runPanduck(["convert", input, "--from", "markdown", "--to", "html", "-o", output, "--diagnostics", "json"]);
    assert.equal(run.code, 0, run.stderr || run.stdout);

    const html = await readFile(output, "utf8");
    assert.match(html, /<figure class="diagram" data-diagram-language="mermaid"/, "the diagram must render as a figure");
    assert.match(html, /<svg[^>]*viewBox="/, "the figure must carry a real inline SVG scene");
    assert.match(html, /data-diagram-source-hex="[0-9a-f]+"/, "the authored source must ride along for round-trip");
    assert.match(html, /Closing paragraph\./);
    assert.ok(!html.includes('<pre class="diagram" data-diagram-language'), "a rendered diagram must not keep the source fallback");

    const report = parseCliReport(run.stdout);
    assert.equal(report.status, "success", `clean render must stay loss-free: ${run.stdout}`);
    assert.equal(report.losses.length, 0);
});

test("cli keeps broken mermaid as visible source and reports structured loss", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-mermaid-broken-"));
    const input = join(workdir, "broken.md");
    const output = join(workdir, "broken.html");
    await writeFile(input, BROKEN_DOC, "utf8");

    const run = runPanduck(["convert", input, "--from", "markdown", "--to", "html", "-o", output, "--diagnostics", "json"]);
    assert.equal(run.code, 0, `warn policy publishes the fallback: ${run.stderr}`);

    const html = await readFile(output, "utf8");
    assert.match(html, /<pre class="diagram" data-diagram-language="mermaid"/, "failed renders keep the visible source block");
    assert.match(html, /this is not a mermaid diagram at all/);
    assert.ok(!html.includes("<figure"), "no figure may be emitted for a failed render");

    const report = parseCliReport(run.stdout);
    assert.equal(report.status, "success_with_loss");
    assert.ok(
        report.losses.some((loss) => loss.code.startsWith("davinci.diagram.")),
        `expected a davinci.diagram.* loss code, got ${JSON.stringify(report.losses)}`,
    );

    const denied = runPanduck(["convert", input, "--from", "markdown", "--to", "html", "-o", join(workdir, "denied.html"), "--loss", "deny", "--diagnostics", "json"]);
    assert.equal(denied.code, 1, "deny policy must fail the conversion on render loss");
});

test("cli namespaces ids per diagram and keeps classes scoped", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-mermaid-namespaced-"));
    const input = join(workdir, "two.md");
    const output = join(workdir, "two.html");
    const twoDiagrams = ["# Two", "", "```mermaid", "flowchart LR", "  A --> B", "```", "", "```mermaid", "flowchart LR", "  A --> B", "```", ""].join("\n");
    await writeFile(input, twoDiagrams, "utf8");

    const run = runPanduck(["convert", input, "--from", "markdown", "--to", "html", "-o", output, "--diagnostics", "silent"]);
    assert.equal(run.code, 0, run.stderr || run.stdout);

    const html = await readFile(output, "utf8");
    const ids = [...html.matchAll(/ id="([^"]+)"/g)].map((match) => match[1]);
    assert.equal(new Set(ids).size, ids.length, `duplicate ids across diagrams: ${ids.join(", ")}`);
    assert.ok(
        ids.filter((id) => !/^node-\d+$/.test(id)).every((id) => id.startsWith("diagram-")),
        `diagram ids carry the page namespace: ${ids.join(", ")}`,
    );
    assert.match(html, /<svg xmlns="http:\/\/www\.w3\.org\/2000\/svg" class="diagram"/);
    assert.match(html, /class="diagram-node"/);
    for (const bare of ['class="node"', 'class="edge"', 'class="label"', 'class="cluster"']) {
        assert.ok(!html.includes(bare), `bare class ${bare} must not leak into the host page`);
    }
});

test("cli mermaid html output is byte stable across runs", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-mermaid-stable-"));
    const input = join(workdir, "diagram.md");
    const first = join(workdir, "first.html");
    const second = join(workdir, "second.html");
    await writeFile(input, FLOWCHART_DOC, "utf8");

    for (const target of [first, second]) {
        const run = runPanduck(["convert", input, "--from", "markdown", "--to", "html", "-o", target, "--diagnostics", "silent"]);
        assert.equal(run.code, 0, run.stderr || run.stdout);
    }
    assert.equal(await readFile(first, "utf8"), await readFile(second, "utf8"));
});

test("node binding renders mermaid inline through the in-memory entry", () => {
    const bindings = loadPanduckNode();
    const convert = bindings.convertBytesMemory;
    assert.ok(convert, "convertBytesMemory must be available on the node binding");

    const response = convert("markdown", "html", "diagram.md", new TextEncoder().encode(FLOWCHART_DOC));
    assert.equal(response.exitCode, 0, response.reportJson);
    assert.ok(response.html, "the html target must return the html payload on the binding");
    assert.match(response.html!, /<figure class="diagram"/);
    assert.match(response.html!, /<svg/);

    const report = JSON.parse(response.reportJson) as CliReport;
    assert.equal(report.status, "success");
});

test("converter async entry preserves the authored mermaid fence on the markdown route", async () => {
    const converter = new Converter();
    const result = await converter.toMarkdown(
        FLOWCHART_DOC,
        defineConversion({ id: "mermaid-fence", input: { format: "markdown" }, output: { format: "markdown" } }),
    );
    assert.equal(result.status, "success", JSON.stringify(result.diagnostics));
    assert.match(result.text, /```mermaid\nflowchart TD/);
    assert.match(result.text, /B -->\|yes\| C\[Stop\]/);
});

test("cli reports an unwired target format as blocked instead of writing output", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-mermaid-blocked-"));
    const input = join(workdir, "diagram.md");
    const output = join(workdir, "diagram.doc");
    await writeFile(input, FLOWCHART_DOC, "utf8");

    const run = runPanduck(["convert", input, "--from", "markdown", "--to", "doc", "-o", output, "--diagnostics", "json"]);
    assert.notEqual(run.code, 0, "an unwired target must not exit successfully");
    const report = parseCliReport(run.stdout);
    assert.equal(report.status, "blocked");
});
