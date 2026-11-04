import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { Converter } from "../src/converter.js";
import { defineConversion } from "../src/conversion-plan.js";
import { footnoteDocumentXml, footnotesXml } from "./fixtures/docx-xml.js";
import { runPanduck } from "./helpers/cli.js";
import { storedZip } from "./helpers/stored-zip.js";

function sampleHtml(body: string): string {
    return `<!doctype html><html><head></head><body>${body}</body></html>`;
}

test("markdown to docx to markdown roundtrip preserves authored semantics", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-roundtrip-"));
    const input = join(workdir, "source.md");
    const docxPath = join(workdir, "middle.docx");
    const backPath = join(workdir, "roundtrip.md");
    const markdown = [
        "# Roundtrip Title",
        "",
        "A paragraph with **bold** and a [link](https://example.com/target).",
        "",
        "- first item",
        "- second item",
        "",
        "## Subheading",
        "",
        "Closing text.",
        "",
    ].join("\n");
    await writeFile(input, markdown, "utf8");

    const toDocx = runPanduck(["convert", input, "--from", "markdown", "--to", "docx", "-o", docxPath, "--diagnostics", "silent"]);
    assert.equal(toDocx.code, 0, toDocx.stderr || toDocx.stdout);

    const toMarkdown = runPanduck(["convert", docxPath, "--from", "docx", "--to", "markdown", "-o", backPath, "--diagnostics", "silent"]);
    assert.equal(toMarkdown.code, 0, toMarkdown.stderr || toMarkdown.stdout);

    const roundtripped = await readFile(backPath, "utf8");
    assert.match(roundtripped, /# Roundtrip Title/, "the h1 must survive both legs");
    assert.match(roundtripped, /## Subheading/, "the h2 must survive both legs");
    assert.match(roundtripped, /\[link\]\(https:\/\/example\.com\/target\)/, "the link text and target must survive");
    assert.match(roundtripped, /\*\*bold\*\*/, "emphasis must survive");
    assert.match(roundtripped, /first item/, "list items must survive");
    assert.match(roundtripped, /second item/, "list items must survive");
    assert.match(roundtripped, /Closing text\./, "plain paragraphs must survive");
});

test("mermaid diagrams survive the markdown to html to markdown roundtrip as diagrams", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-mermaid-roundtrip-"));
    const input = join(workdir, "diagram.md");
    const htmlPath = join(workdir, "diagram.html");
    const backPath = join(workdir, "roundtrip.md");
    const markdown = [
        "# Diagram Roundtrip",
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
    await writeFile(input, markdown, "utf8");

    const toHtml = runPanduck(["convert", input, "--from", "markdown", "--to", "html", "-o", htmlPath, "--diagnostics", "silent"]);
    assert.equal(toHtml.code, 0, toHtml.stderr || toHtml.stdout);

    const html = await readFile(htmlPath, "utf8");
    assert.match(html, /<figure class="diagram" data-diagram-language="mermaid"/, "the diagram must render as a real figure");
    assert.match(html, /<svg[^>]*viewBox="/, "the figure must embed a vector scene, not plain text");
    assert.ok(!html.includes('<pre class="diagram" data-diagram-language'), "a rendered diagram must not be downgraded to a source block");

    const toMarkdown = runPanduck(["convert", htmlPath, "--from", "html", "--to", "markdown", "-o", backPath, "--diagnostics", "silent"]);
    assert.equal(toMarkdown.code, 0, toMarkdown.stderr || toMarkdown.stdout);

    const roundtripped = await readFile(backPath, "utf8");
    assert.match(roundtripped, /```mermaid/, "the rendered figure must read back as a mermaid fence, not plain text");
    assert.match(roundtripped, /A\[Start\] --> B\{Done\?\}/, "the authored edges must survive both legs");
    assert.match(roundtripped, /B -->\|yes\| C\[Stop\]/, "the edge label must survive both legs");
    assert.match(roundtripped, /Closing paragraph\./);
});

test("footnote definitions survive the markdown to html to markdown roundtrip byte for byte", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-footnote-roundtrip-"));
    const input = join(workdir, "footnote.md");
    const htmlPath = join(workdir, "footnote.html");
    const backPath = join(workdir, "roundtrip.md");
    const markdown = ["# Footnotes", "", "See[^1] and also[^long].", "", "[^1]: First body.", "", "[^long]: Second body with `code`.", ""].join("\n");
    await writeFile(input, markdown, "utf8");

    try {
        const toHtml = runPanduck(["convert", input, "--from", "markdown", "--to", "html", "-o", htmlPath, "--diagnostics", "silent"]);
        assert.equal(toHtml.code, 0, toHtml.stderr || toHtml.stdout);

        const html = await readFile(htmlPath, "utf8");
        assert.match(html, /<section class="footnotes" role="doc-endnotes">/, "definitions must render as one endnotes section");
        assert.match(html, /data-footnote-label="1"/, "each reference must carry its label");
        assert.ok(!html.includes("[^1]:"), "the raw definition must not leak into the page prose");

        const toMarkdown = runPanduck(["convert", htmlPath, "--from", "html", "--to", "markdown", "-o", backPath, "--diagnostics", "silent"]);
        assert.equal(toMarkdown.code, 0, toMarkdown.stderr || toMarkdown.stdout);

        const roundtripped = await readFile(backPath, "utf8");
        assert.equal(roundtripped.trimEnd(), markdown.trimEnd(), "footnote references and definitions must be byte-identical");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("tables keep their cell semantics across html and report alignment as the only loss", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-table-roundtrip-"));
    const plainInput = join(workdir, "plain.md");
    const plainHtml = join(workdir, "plain.html");
    const plainBack = join(workdir, "plain-back.md");
    const plain = ["# Table", "", "| head | value |", "| --- | --- |", "| alpha | **bold** and `code` |", ""].join("\n");
    await writeFile(plainInput, plain, "utf8");

    const alignedInput = join(workdir, "aligned.md");
    const alignedReportPath = join(workdir, "aligned-report.json");
    const aligned = ["# Aligned", "", "| left | center | right |", "| :--- | :---: | ---: |", "| a | b | c |", ""].join("\n");
    await writeFile(alignedInput, aligned, "utf8");

    try {
        const toHtml = runPanduck(["convert", plainInput, "--from", "markdown", "--to", "html", "-o", plainHtml, "--diagnostics", "silent"]);
        assert.equal(toHtml.code, 0, toHtml.stderr || toHtml.stdout);
        const html = await readFile(plainHtml, "utf8");
        assert.match(html, /<table>/, "the table must stay a real table element");

        const toMarkdown = runPanduck(["convert", plainHtml, "--from", "html", "--to", "markdown", "-o", plainBack, "--diagnostics", "silent"]);
        assert.equal(toMarkdown.code, 0, toMarkdown.stderr || toMarkdown.stdout);
        const back = await readFile(plainBack, "utf8");
        assert.equal(back.trimEnd(), plain.trimEnd(), "an unaligned table must roundtrip byte for byte");

        const alignedRun = runPanduck([
            "convert",
            alignedInput,
            "--from",
            "markdown",
            "--to",
            "markdown",
            "-o",
            join(workdir, "aligned-out.md"),
            "--diagnostics",
            "json",
            "--report",
            alignedReportPath,
        ]);
        assert.equal(alignedRun.code, 0, alignedRun.stderr);
        const report = JSON.parse(await readFile(alignedReportPath, "utf8")) as {
            status: string;
            losses: Array<{ code: string; status: string }>;
            diagnostics: Array<{ code: string; severity: string }>;
        };
        assert.equal(report.status, "success_with_loss");
        assert.deepEqual(report.losses.map((loss) => loss.code), ["import.markdown.table_alignment"]);
        assert.equal(report.losses[0].status, "Partial");
        assert.equal(report.diagnostics[0].severity, "warning", "a content loss must stay a warning, not an error");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("html table cells authored as block layout report a flattened-cell loss", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-table-cells-"));
    const input = join(workdir, "cells.html");
    const output = join(workdir, "cells.md");
    const reportPath = join(workdir, "cells-report.json");
    await writeFile(
        input,
        sampleHtml("<table><tr><th>Head</th></tr><tr><td><p>one</p><p>two</p></td></tr></table>"),
        "utf8",
    );

    try {
        const run = runPanduck(["convert", input, "--from", "html", "--to", "markdown", "-o", output, "--diagnostics", "json", "--report", reportPath]);
        assert.equal(run.code, 0, run.stderr);
        const report = JSON.parse(await readFile(reportPath, "utf8")) as {
            losses: Array<{ code: string; status: string }>;
        };
        assert.ok(
            report.losses.some((loss) => loss.code === "import.html.table_cell_blocks" && loss.status === "Partial"),
            `flattened cell blocks must be reported: ${JSON.stringify(report.losses)}`,
        );
        assert.match(await readFile(output, "utf8"), /onetwo/, "the authored text must survive as one inline run");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("Converter walks the whole ConverterStatus enum on real inputs", async () => {
    const converter = new Converter();
    // Footnote bodies resolve through footnotes.xml; the docx import still
    // records its structural partial-coverage loss, which drives the enum.
    const lossyDocx = storedZip([
        { path: "word/document.xml", payload: footnoteDocumentXml },
        { path: "word/footnotes.xml", payload: footnotesXml },
    ]);

    // success: a clean markdown document converts without loss markers.
    // (HTML input always records `import.html.partial_coverage`, so the
    // loss-free case uses the markdown route.)
    const clean = await converter.toMarkdown(
        "# Clean\n\nBody.\n",
        defineConversion({ id: "status-success", input: { format: "markdown" }, output: { format: "markdown" } }),
    );
    assert.equal(clean.status, "success", `clean input must be success: ${JSON.stringify(clean.diagnostics)}`);

    // success_with_loss: the footnote docx carries structures the markdown
    // writer reports as real IR loss; the default warn policy keeps them.
    const lossy = await converter.toMarkdown(
        lossyDocx,
        defineConversion({ id: "status-loss", input: { format: "docx", label: "footnote.docx" }, output: { format: "markdown" } }),
    );
    assert.equal(lossy.status, "success_with_loss", "lossy input under warn policy must be success_with_loss");
    assert.ok(
        lossy.diagnostics.some((d) => d.code === "panduck.convert.rust-loss" && d.severity === "warning"),
        "the loss must surface as a structured warning diagnostic",
    );

    // failed: the same loss under a deny policy flips the status.
    const denied = await converter.toMarkdown(
        lossyDocx,
        defineConversion({
            id: "status-deny",
            input: { format: "docx", label: "footnote.docx" },
            output: { format: "markdown", lossPolicy: "deny" },
        }),
    );
    assert.equal(denied.status, "failed", "deny policy must fail on any loss");
});

test("Converter consumes Pandark-shaped artifacts and pages through the async entry", async () => {
    const converter = new Converter();
    const html = sampleHtml("<h1>Seam</h1><p>Delivered by Pandark.</p>");
    const bytes = new TextEncoder().encode(html);
    const plan = defineConversion({
        id: "seam-check",
        input: { format: "html" },
        output: { format: "markdown" },
    });

    // FetchArtifact projection — what Crawler.fetch() delivers.
    const fromArtifact = await converter.toMarkdown(
        { finalUrl: "https://example.com/article", bodyText: html, bodyBytes: bytes },
        plan,
    );
    assert.notEqual(fromArtifact.status, "failed", JSON.stringify(fromArtifact.diagnostics));
    assert.match(fromArtifact.text, /Delivered by Pandark\./);
    assert.ok(
        fromArtifact.diagnostics.some((d) => d.message?.args?.some(([key, value]) => key === "url" && (value as { text?: string })?.text === "https://example.com/article")),
        "the artifact URL must reach the diagnostics as the url hint",
    );

    // Page projection — what handlePage receives.
    const fromPage = await converter.toMarkdown(
        {
            url: "https://example.com/page",
            bytes: () => bytes,
            html: async () => html,
        },
        plan,
    );
    assert.notEqual(fromPage.status, "failed", JSON.stringify(fromPage.diagnostics));
    assert.match(fromPage.text, /Delivered by Pandark\./);
});

test("garbage bytes declared as docx fail with an error diagnostic instead of empty output", async () => {
    const converter = new Converter();
    const plan = defineConversion({
        id: "garbage-check",
        input: { format: "docx", label: "garbage.docx" },
        output: { format: "markdown" },
    });

    const result = await converter.toMarkdown(new TextEncoder().encode("this is not a zip container"), plan);

    assert.equal(result.status, "failed", "undecodable input must fail");
    assert.equal(result.text, "", "no markdown may be emitted for undecodable input");
    assert.ok(
        result.diagnostics.some((d) => d.severity === "error"),
        `an error diagnostic must explain the failure: ${JSON.stringify(result.diagnostics)}`,
    );
});

test("list numbering, task markers, and quoted nesting survive the markdown roundtrip", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-list-semantics-"));
    const input = join(workdir, "lists.md");
    const output = join(workdir, "lists-back.md");
    const source = ["3. third", "4. fourth", "", "- [x] done", "- [ ] todo", "", "> - alpha", "> - beta", ""].join("\n");
    await writeFile(input, source, "utf8");

    try {
        const run = runPanduck(["convert", input, "--from", "markdown", "--to", "markdown", "-o", output, "--diagnostics", "json"]);
        assert.equal(run.code, 0, run.stderr);
        const markdown = await readFile(output, "utf8");
        assert.match(markdown, /3\. third/, "the authored list number must be written back, not restarted at one");
        assert.match(markdown, /4\. fourth/, "the following item keeps its authored number");
        assert.match(markdown, /- \[x\] done/, "the checked task marker must be written back");
        assert.match(markdown, /- \[ \] todo/, "the unchecked task marker must be written back");
        assert.match(markdown, /^> - alpha$/m, "a quoted list must stay nested instead of being escaped");
        assert.match(markdown, /^> - beta$/m, "every quoted item keeps its nested list form");
        assert.doesNotMatch(markdown, /\\-/, "a nested list item must not be backslash escaped");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("merged table cells survive the html to markdown to html roundtrip", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-merged-cells-"));
    const input = join(workdir, "spans.html");
    const markdownPath = join(workdir, "spans.md");
    const backPath = join(workdir, "spans-back.html");
    await writeFile(
        input,
        sampleHtml('<table><tr><th colspan="2">Head</th></tr><tr><td rowspan="2">a</td><td>b</td></tr><tr><td>c</td></tr></table>'),
        "utf8",
    );

    try {
        const toMarkdown = runPanduck(["convert", input, "--from", "html", "--to", "markdown", "-o", markdownPath, "--diagnostics", "json"]);
        assert.equal(toMarkdown.code, 0, toMarkdown.stderr);
        const markdown = await readFile(markdownPath, "utf8");
        assert.match(markdown, /<table>/, "a spanning table must be written as an HTML table rather than flattened");
        assert.match(markdown, /colspan="2"/, "the column span must be written back");
        assert.match(markdown, /rowspan="2"/, "the row span must be written back");

        const back = runPanduck(["convert", markdownPath, "--from", "markdown", "--to", "html", "-o", backPath, "--diagnostics", "json"]);
        assert.equal(back.code, 0, back.stderr);
        const html = await readFile(backPath, "utf8");
        assert.match(html, /<th colspan="2">Head<\/th>/, "the reopened table keeps the column span: " + html);
        assert.match(html, /<td rowspan="2">a<\/td>/, "the reopened table keeps the row span: " + html);
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("a definition list downgrade is reported and its text survives", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-definition-list-"));
    const input = join(workdir, "terms.md");
    const output = join(workdir, "terms-back.md");
    const reportPath = join(workdir, "terms-report.json");
    await writeFile(input, "Term\n: Definition\n", "utf8");

    try {
        const run = runPanduck([
            "convert",
            input,
            "--from",
            "markdown",
            "--to",
            "markdown",
            "-o",
            output,
            "--diagnostics",
            "json",
            "--report",
            reportPath,
        ]);
        assert.equal(run.code, 0, run.stderr);
        const report = JSON.parse(await readFile(reportPath, "utf8")) as {
            losses: Array<{ code: string; status: string }>;
        };
        assert.ok(
            report.losses.some((loss) => loss.code === "import.markdown.definition_list" && loss.status === "Partial"),
            `the paragraph downgrade must be reported: ${JSON.stringify(report.losses)}`,
        );
        const markdown = await readFile(output, "utf8");
        assert.match(markdown, /Term/, "the term text must survive the downgrade");
        assert.match(markdown, /Definition/, "the definition text must survive the downgrade");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("raw html blocks and inline html stay verbatim on the markdown roundtrip", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-raw-html-"));
    const input = join(workdir, "raw.md");
    const output = join(workdir, "raw-back.md");
    const source = ["<div>", "raw <span>kept</span>", "</div>", "", "Inline <b>markup</b> stays.", ""].join("\n");
    await writeFile(input, source, "utf8");

    try {
        const run = runPanduck(["convert", input, "--from", "markdown", "--to", "markdown", "-o", output, "--diagnostics", "json"]);
        assert.equal(run.code, 0, run.stderr);
        const markdown = await readFile(output, "utf8");
        assert.match(markdown, /<div>/, "the authored html block must stay a raw block");
        assert.match(markdown, /raw <span>kept<\/span>/, "every line of the html block must survive");
        assert.match(markdown, /<\/div>/, "the block must not be truncated after its first line");
        assert.match(markdown, /Inline <b>markup<\/b> stays\./, "inline html must stay verbatim instead of being escaped");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("mixed diagram languages stay typed fences in one document", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-mixed-diagrams-"));
    const input = join(workdir, "mixed.md");
    const htmlPath = join(workdir, "mixed.html");
    const backPath = join(workdir, "mixed-back.md");
    const source = [
        "# Mixed",
        "",
        "```mermaid",
        "flowchart LR",
        "  A --> B",
        "```",
        "",
        "```plantuml",
        "@startuml",
        "Alice -> Bob",
        "@enduml",
        "```",
        "",
        "```dot",
        "digraph { a -> b }",
        "```",
        "",
    ].join("\n");
    await writeFile(input, source, "utf8");

    try {
        const forward = runPanduck(["convert", input, "--from", "markdown", "--to", "html", "-o", htmlPath, "--diagnostics", "json"]);
        assert.equal(forward.code, 0, forward.stderr);
        const html = await readFile(htmlPath, "utf8");
        for (const language of ["mermaid", "plantuml", "dot"]) {
            assert.match(
                html,
                new RegExp(`<figure class="diagram" data-diagram-language="${language}"`),
                `${language} must render as its own typed diagram`,
            );
        }

        const backward = runPanduck(["convert", htmlPath, "--from", "html", "--to", "markdown", "-o", backPath, "--diagnostics", "json"]);
        assert.equal(backward.code, 0, backward.stderr);
        const back = await readFile(backPath, "utf8");
        for (const fence of ["```mermaid", "```plantuml", "```dot"]) {
            assert.match(back, new RegExp(fence), `${fence} must survive the roundtrip`);
        }
        assert.match(back, /A --> B/, "the mermaid body must stay verbatim");
        assert.match(back, /Alice -> Bob/, "the plantuml body must stay verbatim");
        assert.match(back, /digraph \{ a -> b \}/, "the dot body must stay verbatim");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});
