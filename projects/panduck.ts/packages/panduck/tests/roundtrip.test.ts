import assert from "node:assert/strict";
import { mkdtemp, readFile, writeFile } from "node:fs/promises";
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
