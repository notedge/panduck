import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { runPanduck } from "./helpers/cli.js";
import { storedZip } from "./helpers/stored-zip.js";
import { minimalDocumentXml } from "./fixtures/docx-xml.js";

const containerXml = `<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>`;

const bookOpf = `<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Route Book</dc:title>
    <dc:language>en</dc:language>
  </metadata>
  <manifest>
    <item id="ch1" href="ch1.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="ch1"/>
  </spine>
</package>`;

const bookChapter = `<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <body>
    <h1>Chapter Body</h1>
    <p>Text with <strong>weight</strong> and a <a href="https://example.com/doc">link</a>.</p>
  </body>
</html>`;

/** A one-chapter EPUB that exercises heading, styled inline text and a link. */
function bookEpub(): Buffer {
    return storedZip([
        { path: "mimetype", payload: Buffer.from("application/epub+zip") },
        { path: "META-INF/container.xml", payload: Buffer.from(containerXml) },
        { path: "OEBPS/content.opf", payload: Buffer.from(bookOpf) },
        { path: "OEBPS/ch1.xhtml", payload: Buffer.from(bookChapter) },
    ]);
}

/** A DOCX package whose body is one plain paragraph and one heading paragraph. */
function headingDocx(): Buffer {
    return storedZip([{ path: "word/document.xml", payload: minimalDocumentXml }]);
}

type RouteReport = {
    status: string;
    losses: Array<{ code: string; status: string }>;
};

/** Converts one input to HTML and reopens the written page through HTML import. */
async function convertToHtmlAndReopen(input: {
    workdir: string;
    bytes: Buffer;
    label: string;
    from: string;
}): Promise<{ html: string; markdown: string; report: RouteReport }> {
    const { workdir, bytes, label, from } = input;
    const sourcePath = join(workdir, label);
    const htmlPath = join(workdir, `${from}.html`);
    const reportPath = join(workdir, `${from}.report.json`);
    const backPath = join(workdir, `${from}.md`);
    await writeFile(sourcePath, bytes);

    const toHtml = runPanduck([
        "convert",
        sourcePath,
        "--from",
        from,
        "--to",
        "html",
        "-o",
        htmlPath,
        "--report",
        reportPath,
        "--diagnostics",
        "silent",
    ]);
    assert.equal(toHtml.code, 0, toHtml.stderr || toHtml.stdout);

    const toMarkdown = runPanduck([
        "convert",
        htmlPath,
        "--from",
        "html",
        "--to",
        "markdown",
        "-o",
        backPath,
        "--overwrite",
        "--diagnostics",
        "silent",
    ]);
    assert.equal(toMarkdown.code, 0, toMarkdown.stderr || toMarkdown.stdout);

    return {
        html: await readFile(htmlPath, "utf8"),
        markdown: await readFile(backPath, "utf8"),
        report: JSON.parse(await readFile(reportPath, "utf8")) as RouteReport,
    };
}

test("epub to html writes page blocks and reopens as the same heading and link", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-epub-html-"));
    try {
        const { html, markdown, report } = await convertToHtmlAndReopen({
            workdir,
            bytes: bookEpub(),
            label: "route-book.epub",
            from: "epub",
        });

        assert.match(html, /<h1[^>]*>Chapter Body<\/h1>/, "the chapter heading must reach the page");
        assert.match(html, /<strong>weight<\/strong>/, "the styled run must stay strong");
        assert.match(html, /<a [^>]*href="https:\/\/example\.com\/doc"[^>]*>link<\/a>/, "the link must keep its target");

        assert.match(markdown, /^# Chapter Body$/m, "reopening must recover the heading level");
        assert.match(markdown, /Text with \*\*weight\*\* and a \[link\]\(https:\/\/example\.com\/doc\)\./);

        assert.equal(report.status, "success_with_loss", "partial coverage must be declared, not hidden");
        assert.deepEqual(
            report.losses.map((loss) => loss.code),
            ["import.epub.partial_coverage"],
            "the only declared loss must be the EPUB import partial coverage marker",
        );
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("docx to html writes headings and paragraphs and reopens with the same structure", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-docx-html-"));
    try {
        const { html, markdown, report } = await convertToHtmlAndReopen({
            workdir,
            bytes: headingDocx(),
            label: "route.docx",
            from: "docx",
        });

        assert.match(html, /<p[^>]*>Hello DOCX<\/p>/, "the plain paragraph must reach the page");
        assert.match(html, /<h1[^>]*>Title<\/h1>/, "the heading paragraph must keep its level");

        assert.match(markdown, /^Hello DOCX$/m, "reopening must recover the plain paragraph");
        assert.match(markdown, /^# Title$/m, "reopening must recover the heading level");

        assert.equal(report.status, "success_with_loss", "partial coverage must be declared, not hidden");
        assert.deepEqual(
            report.losses.map((loss) => loss.code),
            ["import.docx.partial_coverage"],
            "the only declared loss must be the DOCX import partial coverage marker",
        );
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});
