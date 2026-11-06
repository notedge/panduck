import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { Converter } from "../src/converter.js";
import { defineConversion } from "../src/conversion-plan.js";
import { runPanduck } from "./helpers/cli.js";
import { storedZip } from "./helpers/stored-zip.js";

const containerXml = `<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>`;

const projectBookOpf = `<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Project Book</dc:title>
    <dc:language>en</dc:language>
  </metadata>
  <manifest>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
    <item id="ch1" href="ch1.xhtml" media-type="application/xhtml+xml"/>
    <item id="ch2" href="ch2.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="ch1"/>
    <itemref idref="ch2"/>
  </spine>
</package>`;

const projectBookNav = `<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
  <body>
    <nav epub:type="toc" id="toc">
      <ol>
        <li><a href="ch1.xhtml">Chapter One</a></li>
        <li><a href="ch2.xhtml">Chapter Two</a></li>
      </ol>
    </nav>
  </body>
</html>`;

const projectBookChapterOne = `<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <body>
    <h1>Chapter One</h1>
    <p>First <strong>chapter</strong> body.</p>
    <p><img src="images/cover.png" alt="Cover"/></p>
  </body>
</html>`;

const projectBookChapterTwo = `<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <body>
    <h1>Chapter Two</h1>
    <p>Second chapter body.</p>
    <p><img src="images/missing.png" alt="Missing"/></p>
  </body>
</html>`;

const coverBytes = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x00, 0x01, 0x02, 0x03]);

/** A two-chapter EPUB whose second chapter names an image the package lacks. */
function projectBookEpub(): Buffer {
    return storedZip([
        { path: "mimetype", payload: Buffer.from("application/epub+zip") },
        { path: "META-INF/container.xml", payload: Buffer.from(containerXml) },
        { path: "OEBPS/content.opf", payload: Buffer.from(projectBookOpf) },
        { path: "OEBPS/nav.xhtml", payload: Buffer.from(projectBookNav) },
        { path: "OEBPS/ch1.xhtml", payload: Buffer.from(projectBookChapterOne) },
        { path: "OEBPS/ch2.xhtml", payload: Buffer.from(projectBookChapterTwo) },
        { path: "OEBPS/images/cover.png", payload: coverBytes },
    ]);
}

const pageHtml = `<!doctype html><html><head><title>Page</title></head><body>
<h1>Page</h1>
<p>Body text with a <a href="https://example.com/doc">link</a>.</p>
<img src="https://example.com/pic.png" alt="Remote">
<img src="images/local.png" alt="Local">
</body></html>`;

test("epub project keeps spine order, materializes package assets and reports missing ones", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-project-epub-"));
    const outputDir = join(workdir, "book");
    try {
        const converter = new Converter();
        const plan = defineConversion({
            id: "epub-project-check",
            input: { format: "epub", label: "project-book.epub" },
            output: { format: "markdown" },
            publish: { kind: "markdown-project", directory: outputDir },
        });

        const result = await converter.toMarkdown(projectBookEpub(), plan);
        assert.equal(result.status, "success_with_loss", JSON.stringify(result.diagnostics));
        assert.equal(result.publishPath, outputDir);

        const index = await readFile(join(outputDir, "index.md"), "utf8");
        assert.match(index, /^# Project Book/m);
        const firstLink = index.indexOf("(chapters/000-ch1.md)");
        const secondLink = index.indexOf("(chapters/001-ch2.md)");
        assert.ok(firstLink >= 0 && secondLink >= 0, `index must link both spine chapters: ${index}`);
        assert.ok(firstLink < secondLink, "chapter links must follow the OPF spine order");

        const chapterOne = await readFile(join(outputDir, "chapters", "000-ch1.md"), "utf8");
        assert.match(chapterOne, /# Chapter One/);
        const coverLink = chapterOne.match(/!\[Cover\]\(assets\/([0-9a-f]{16}\.png)\)/);
        assert.ok(coverLink, `the package image must be rewritten to a content-addressed asset: ${chapterOne}`);
        assert.deepEqual(await readFile(join(outputDir, "assets", coverLink[1])), coverBytes);

        const chapterTwo = await readFile(join(outputDir, "chapters", "001-ch2.md"), "utf8");
        assert.match(chapterTwo, /# Chapter Two/);
        assert.match(chapterTwo, /!\[Missing\]\(images\/missing\.png\)/);

        const report = JSON.parse(result.reportJson);
        assert.equal(report.status, "success_with_loss");
        const lossCodes = (report.losses as Array<{ code: string }>).map((loss) => loss.code).sort();
        assert.deepEqual(lossCodes, ["import.epub.asset_unresolved", "import.epub.partial_coverage"]);
        assert.deepEqual(report.unresolved_assets, ["OEBPS/images/missing.png"]);
        const unresolved = result.diagnostics.find((entry) => entry.code === "import.epub.asset_unresolved");
        assert.equal(
            unresolved?.severity,
            "error",
            "the upstream loss marker keeps its severity while the product status stays success_with_loss",
        );
        const assetOutput = report.outputs.find((output: { path?: string }) => output.path === "assets/");
        assert.equal(assetOutput?.published_asset_count, 1);

        const reopened = await converter.toMarkdown(
            chapterOne,
            defineConversion({
                id: "epub-project-reopen",
                input: { format: "markdown", label: "000-ch1.md" },
                output: { format: "markdown" },
            }),
        );
        assert.notEqual(reopened.status, "failed");
        assert.match(reopened.text, /# Chapter One/);
        assert.match(reopened.text, /First \*\*chapter\*\* body\./);
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("html project keeps unresolved image sources and matches the plain conversion losses", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-project-html-"));
    const outputDir = join(workdir, "page");
    try {
        const converter = new Converter();
        const projectPlan = defineConversion({
            id: "html-project-check",
            input: { format: "html", label: "page.html" },
            output: { format: "markdown" },
            publish: { kind: "markdown-project", directory: outputDir },
        });
        const plainPlan = defineConversion({
            id: "html-plain-check",
            input: { format: "html", label: "page.html" },
            output: { format: "markdown" },
        });

        const project = await converter.toMarkdown(pageHtml, projectPlan);
        const plain = await converter.toMarkdown(pageHtml, plainPlan);
        assert.notEqual(project.status, "failed");
        assert.notEqual(plain.status, "failed");

        const index = await readFile(join(outputDir, "index.md"), "utf8");
        assert.match(index, /# Page/);
        assert.match(index, /Body text with a \[link\]\(https:\/\/example\.com\/doc\)\./);
        assert.match(index, /!\[Remote\]\(https:\/\/example\.com\/pic\.png\)/);
        assert.match(index, /!\[Local\]\(images\/local\.png\)/);

        const projectReport = JSON.parse(project.reportJson);
        assert.equal(projectReport.pipeline.writer, "markdown-project");
        assert.deepEqual(projectReport.unresolved_assets, [
            "https://example.com/pic.png",
            "images/local.png",
        ]);
        const assetOutput = projectReport.outputs.find((output: { path?: string }) => output.path === "assets/");
        assert.equal(assetOutput?.published_asset_count, 0);

        const plainReport = JSON.parse(plain.reportJson);
        const projectLosses = (projectReport.losses as Array<{ code: string }>).map((loss) => loss.code).sort();
        const plainLosses = (plainReport.losses as Array<{ code: string }>).map((loss) => loss.code).sort();
        assert.deepEqual(projectLosses, plainLosses, "the project report must declare the same IR losses");
        assert.deepEqual(projectReport.coverage, plainReport.coverage);
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

const legacyDocPath = join(
    dirname(fileURLToPath(import.meta.url)),
    "../../panduck-fixtures/fixtures/acorn/doc/legacy_plain.doc",
);

test("doc project route publishes the readable text and declares its partial loss", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-project-doc-"));
    const outputDir = join(workdir, "legacy");
    const reportPath = join(workdir, "legacy.report.json");
    try {
        const published = runPanduck([
            "convert",
            legacyDocPath,
            "--from",
            "doc",
            "--to",
            "markdown-project",
            "--output-dir",
            outputDir,
            "--report",
            reportPath,
            "--diagnostics",
            "silent",
        ]);
        assert.equal(published.code, 0, published.stderr);

        const index = await readFile(join(outputDir, "index.md"), "utf8");
        assert.match(index, /Hello legacy DOC/);
        assert.match(index, /Second paragraph/);

        const report = JSON.parse(await readFile(reportPath, "utf8"));
        const lossCodes = (report.losses as Array<{ code: string }>).map((loss) => loss.code).sort();
        assert.deepEqual(lossCodes, ["import.doc.partial_coverage"]);
        assert.equal(report.status, "success_with_loss");
        assert.ok(
            (report.outputs as Array<{ path: string }>).every((output) => !output.path.includes("assets")),
            "a DOC text project must not claim published assets",
        );
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("pdf project route republishes extracted text with the same losses as the plain route", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-project-pdf-"));
    const outputDir = join(workdir, "report");
    try {
        const converter = new Converter();
        const source = "# Report\n\nHello PDF paragraph.\n\n- one\n- two\n";
        const sourcePath = join(workdir, "report.md");
        const pdfPath = join(workdir, "report.pdf");
        await writeFile(sourcePath, source, "utf8");
        const published = runPanduck([
            "convert",
            sourcePath,
            "--from",
            "markdown",
            "--to",
            "pdf",
            "-o",
            pdfPath,
            "--diagnostics",
            "silent",
        ]);
        assert.equal(published.code, 0, published.stderr);
        const pdfBytes = new Uint8Array(await readFile(pdfPath));

        const project = await converter.toMarkdown(
            pdfBytes,
            defineConversion({
                id: "pdf-project-check",
                input: { format: "pdf", label: "report.pdf" },
                output: { format: "markdown" },
                publish: { kind: "markdown-project", directory: outputDir },
            }),
        );
        const plain = await converter.toMarkdown(
            pdfBytes,
            defineConversion({
                id: "pdf-project-plain",
                input: { format: "pdf", label: "report.pdf" },
                output: { format: "markdown" },
            }),
        );
        assert.equal(project.status, "success_with_loss", JSON.stringify(project.diagnostics));

        const index = await readFile(join(outputDir, "index.md"), "utf8");
        assert.match(index, /# Report/);
        assert.match(index, /Hello PDF paragraph\./);
        assert.match(index, /- one\n- two/);

        const projectReport = JSON.parse(project.reportJson);
        const lossCodes = (projectReport.losses as Array<{ code: string }>).map((loss) => loss.code).sort();
        assert.ok(
            lossCodes.every((code) => code.startsWith("import.pdf.")),
            `the PDF reimport must only declare PDF import losses: ${lossCodes.join(", ")}`,
        );
        assert.ok(lossCodes.includes("import.pdf.partial_coverage"));
        const assetOutput = projectReport.outputs.find((output: { path?: string }) => output.path === "assets/");
        assert.equal(assetOutput?.published_asset_count, 0);

        const plainReport = JSON.parse(plain.reportJson);
        const plainLosses = (plainReport.losses as Array<{ code: string }>).map((loss) => loss.code).sort();
        assert.deepEqual(lossCodes, plainLosses, "the project report must declare the same PDF import losses");
        assert.deepEqual(projectReport.coverage, plainReport.coverage);
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});
