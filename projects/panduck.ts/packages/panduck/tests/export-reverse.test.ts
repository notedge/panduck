import assert from "node:assert/strict";
import { constants, readFileSync } from "node:fs";
import { access, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

import { chromium } from "playwright";

import { runPanduck } from "./helpers/cli.js";
import { readStoredZip } from "./helpers/stored-zip.js";

const fixtures = join(dirname(fileURLToPath(import.meta.url)), "fixtures");
const pdfjsEntry = import.meta.resolve("pdfjs-dist/legacy/build/pdf.mjs");
const pdfjsBuild = dirname(fileURLToPath(pdfjsEntry));

type PdfPage = {
    getViewport(options: { scale: number }): { width: number; height: number };
    getTextContent(): Promise<{ items: Array<{ str?: string }> }>;
};

type PdfDocument = {
    numPages: number;
    getPage(pageNumber: number): Promise<PdfPage>;
};

type PdfJs = {
    getDocument(options: { data: Uint8Array }): { promise: Promise<PdfDocument> };
};

const pdfjs = (await import(pdfjsEntry)) as PdfJs;

/** 1x1 PNG used by the fixture image construct. */
const PIXEL_PNG = Buffer.from(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==",
    "base64",
);

async function exists(path: string): Promise<boolean> {
    try {
        await access(path, constants.F_OK);
        return true;
    } catch {
        return false;
    }
}

async function convertToPdf(workdir: string): Promise<{ pdf: Buffer; pdfPath: string; reportText: string }> {
    const source = join(fixtures, "complex-report.md");
    const input = join(workdir, "complex-report.md");
    const pdfPath = join(workdir, "complex-report.pdf");
    const reportPath = join(workdir, "complex-report.report.json");
    await writeFile(input, await readFile(source, "utf8"), "utf8");
    await writeFile(join(workdir, "pixel.png"), PIXEL_PNG);

    const result = runPanduck([
        "convert",
        input,
        "--from",
        "markdown",
        "--to",
        "pdf",
        "-o",
        pdfPath,
        "--overwrite",
        "--report",
        reportPath,
        "--diagnostics",
        "silent",
    ]);
    assert.equal(result.code, 0, result.stderr || result.stdout);
    return { pdf: await readFile(pdfPath), pdfPath, reportText: await readFile(reportPath, "utf8") };
}

/** Renders one PDF page to PNG through a local Chromium canvas. */
async function renderPage(
    pdf: Buffer,
    workdir: string,
    pageNumber: number,
): Promise<{ pngPath: string; width: number; height: number }> {
    const server = createServer((request, response) => {
        const url = new URL(request.url ?? "/", "http://127.0.0.1");
        if (url.pathname === "/verify.html") {
            response.writeHead(200, { "content-type": "text/html" });
            response.end(`<!doctype html><canvas id="page"></canvas><script type="module">
import * as pdfjs from "/pdf.mjs";
pdfjs.GlobalWorkerOptions.workerSrc = "/pdf.worker.mjs";
const doc = await pdfjs.getDocument({ data: await (await fetch("/doc.pdf")).arrayBuffer() }).promise;
const page = await doc.getPage(${pageNumber});
const viewport = page.getViewport({ scale: 1.5 });
const canvas = document.getElementById("page");
canvas.width = viewport.width;
canvas.height = viewport.height;
await page.render({ canvasContext: canvas.getContext("2d"), viewport }).promise;
window.__metrics = { width: viewport.width, height: viewport.height };
document.title = "rendered";
</script>`);
            return;
        }
        const name = url.pathname.slice(1);
        const payload = name === "doc.pdf" ? pdf : name === "pdf.mjs" || name === "pdf.worker.mjs" ? readFileSync(join(pdfjsBuild, name)) : null;
        if (!payload) {
            response.writeHead(404).end("missing");
            return;
        }
        response.writeHead(200, { "content-type": name.endsWith(".mjs") ? "text/javascript" : "application/pdf" });
        response.end(payload);
    });
    await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", () => resolve()));
    const port = (server.address() as { port: number }).port;
    const browser = await chromium.launch();
    try {
        const page = await browser.newPage({ viewport: { width: 1000, height: 1400 } });
        await page.goto(`http://127.0.0.1:${port}/verify.html`);
        await page.waitForFunction(() => document.title === "rendered", null, { timeout: 30_000 });
        const metrics = (await page.evaluate(() => (window as unknown as { __metrics: { width: number; height: number } }).__metrics)) as {
            width: number;
            height: number;
        };
        const pngPath = join(workdir, `complex-report-page-${pageNumber}.png`);
        await page.locator("#page").screenshot({ path: pngPath });
        return { pngPath, ...metrics };
    } finally {
        await browser.close();
        await new Promise<void>((resolve) => server.close(() => resolve()));
    }
}

function decodeXmlText(xml: string): string {
    return xml
        .replace(/<w:tab\/>/g, " ")
        .replace(/<w:br\/>/g, "\n")
        .replace(/<[^>]+>/g, "")
        .replace(/&quot;/g, '"')
        .replace(/&apos;/g, "'")
        .replace(/&lt;/g, "<")
        .replace(/&gt;/g, ">")
        .replace(/&amp;/g, "&");
}

function normalize(text: string): string {
    return text.replace(/\s+/g, " ").trim();
}

/** Turns a Markdown fixture into the body and footnote text a writer must keep. */
function expectedText(markdown: string): { body: string[]; footnotes: string[] } {
    const body: string[] = [];
    const footnotes: string[] = [];
    for (const raw of markdown.split(/\r?\n/)) {
        const line = raw.trim();
        if (line.startsWith("```")) {
            continue;
        }
        if (!line) {
            continue;
        }
        const footnote = /^\[\^[^\]]+\]:\s+(.*)$/.exec(line);
        if (footnote) {
            footnotes.push(normalize(footnote[1]));
            continue;
        }
        if (/^\|[\s:|-]+\|$/.test(line)) {
            continue;
        }
        const items = line.startsWith("|")
            ? line.split("|").map((cell) => cell.trim()).filter(Boolean)
            : [line.replace(/^#{1,6}\s+/, "").replace(/^\s*[-*+]\s+/, "").replace(/^\s*\d+\.\s+/, "")];
        for (const item of items) {
            const text = normalize(
                item
                    .replace(/!\[([^\]]*)\]\([^)]*\)/g, "$1")
                    .replace(/\[([^\]]+)\]\([^)]*\)/g, "$1")
                    .replace(/\[\^[^\]]+\]/g, "")
                    .replace(/(\*\*|~~|`|\*)/g, ""),
            );
            if (text) {
                body.push(text);
            }
        }
    }
    return { body, footnotes };
}

test("PDF export reopens with measurable pages, text and a rendered PNG", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-pdf-reverse-"));
    try {
        const { pdf, reportText } = await convertToPdf(workdir);
        const document = await pdfjs.getDocument({ data: new Uint8Array(pdf) }).promise;
        let text = "";
        const sizes: Array<{ page: number; width: number; height: number }> = [];
        for (let pageNumber = 1; pageNumber <= document.numPages; pageNumber += 1) {
            const page = await document.getPage(pageNumber);
            const viewport = page.getViewport({ scale: 1 });
            sizes.push({ page: pageNumber, width: viewport.width, height: viewport.height });
            const content = await page.getTextContent();
            text += `${content.items.map((item) => item.str ?? "").join(" ")}\n`;
        }
        const characters = text.replace(/\s/g, "").length;
        const normalized = normalize(text);
        const firstPage = await renderPage(pdf, workdir, 1);

        console.log(
            JSON.stringify({
                target: "pdf",
                pages: document.numPages,
                characters,
                sizes,
                bytes: pdf.byteLength,
                screenshot: firstPage.pngPath,
                screenshotSize: { width: Math.round(firstPage.width), height: Math.round(firstPage.height) },
                reportStatus: (JSON.parse(reportText) as { status: string }).status,
            }),
        );

        assert.equal(document.numPages, 2, "the fixture must span two pages");
        assert.equal(characters > 300, true, `expected readable text, got ${characters} characters`);
        for (const size of sizes) {
            assert.ok(Math.abs(size.width - 595.28) < 1, `page ${size.page} width ${size.width} is not A4`);
            assert.ok(Math.abs(size.height - 841.89) < 1, `page ${size.page} height ${size.height} is not A4`);
        }
        assert.match(normalized, /Document Family Report/);
        assert.match(normalized, /alpha/);
        assert.match(normalized, /deferred/);
        assert.match(normalized, /graph LR/, "a diagram fence falls back to its source text today");
        assert.match(normalized, /vega-lite/, "a chart fence falls back to its source text today");
        assert.equal(
            normalized.includes("The footnote body."),
            false,
            "the routed PDF writer drops footnote bodies today",
        );
        assert.equal(pdf.includes("/Subtype /Image"), false, "the routed PDF writer embeds no image XObject today");
        assert.equal(await exists(firstPage.pngPath), true, "the page must render to a real PNG");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("DOCX export round-trips structure and text with zero differences", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-docx-reverse-"));
    try {
        const input = join(workdir, "complex-report-docx.md");
        const output = join(workdir, "complex-report.docx");
        await writeFile(input, await readFile(join(fixtures, "complex-report-docx.md"), "utf8"), "utf8");

        const result = runPanduck([
            "convert",
            input,
            "--from",
            "markdown",
            "--to",
            "docx",
            "-o",
            output,
            "--overwrite",
            "--diagnostics",
            "silent",
        ]);
        assert.equal(result.code, 0, result.stderr || result.stdout);

        const members = readStoredZip(await readFile(output));
        const documentXml = members.get("word/document.xml")?.toString("utf8") ?? "";
        const footnotesXml = members.get("word/footnotes.xml")?.toString("utf8") ?? "";
        const bodyText = normalize(decodeXmlText(documentXml));
        const footnoteText = normalize(decodeXmlText(footnotesXml));
        const expected = expectedText(await readFile(join(fixtures, "complex-report-docx.md"), "utf8"));

        const missingBody = expected.body.filter((token) => !bodyText.includes(token));
        const missingFootnotes = expected.footnotes.filter((token) => !footnoteText.includes(token));
        const paragraphBlocks = (documentXml.match(/<w:p\b/g) ?? []).length;
        const tableRows = (documentXml.match(/<w:tr\b/g) ?? []).length;
        const headings = {
            heading1: (documentXml.match(/w:val="Heading1"/g) ?? []).length,
            heading2: (documentXml.match(/w:val="Heading2"/g) ?? []).length,
            heading3: (documentXml.match(/w:val="Heading3"/g) ?? []).length,
        };
        const characters = bodyText.replace(/\s/g, "").length;

        console.log(
            JSON.stringify({
                target: "docx",
                bytes: (await readFile(output)).byteLength,
                paragraphBlocks,
                characters,
                differences: missingBody.length + missingFootnotes.length,
                missingBody,
                missingFootnotes,
                tableRows,
                headings,
                diagramStyle: documentXml.includes('w:val="Diagram:mermaid"'),
                chartStyle: documentXml.includes('w:val="Chart:vega-lite"'),
                footnoteBody: footnoteText.includes("The footnote body."),
            }),
        );

        assert.equal(missingBody.length + missingFootnotes.length, 0, JSON.stringify({ missingBody, missingFootnotes }));
        assert.equal(paragraphBlocks > 20, true, `expected a real multi-block document, got ${paragraphBlocks} paragraphs`);
        assert.equal(characters > 300, true, `expected readable text, got ${characters} characters`);
        assert.equal(tableRows, 3, "the table must keep three rows");
        assert.equal(headings.heading1, 1);
        assert.equal(headings.heading2 >= 5, true);
        assert.equal(headings.heading3 >= 1, true);
        assert.equal(documentXml.includes('w:val="Diagram:mermaid"'), true);
        assert.equal(documentXml.includes('w:val="Chart:vega-lite"'), true);
        assert.equal(footnoteText.includes("The footnote body."), true);
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("writer-side unsupported constructs fail as writer failures", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-docx-unsupported-"));
    try {
        const input = join(workdir, "inline-math.md");
        const output = join(workdir, "inline-math.docx");
        await writeFile(input, "# Math\n\nInline $E=mc^2$ only.\n", "utf8");

        const result = runPanduck([
            "convert",
            input,
            "--from",
            "markdown",
            "--to",
            "docx",
            "-o",
            output,
            "--overwrite",
            "--diagnostics",
            "json",
        ]);

        assert.equal(result.code, 5, result.stderr || result.stdout);
        assert.equal(await exists(output), false, "a writer refusal must not publish a partial DOCX");
        const report = JSON.parse(result.stdout) as {
            status: string;
            diagnostics?: Array<{ code: string }>;
        };
        assert.equal(report.status, "blocked");
        assert.equal(report.diagnostics?.[0]?.code, "notedown.format.unsupported");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});
