import assert from "node:assert/strict";
import { mkdtemp, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { Converter } from "../src/converter.js";
import { defineConversion } from "../src/conversion-plan.js";
import { imageDocumentXml, imageRelsXml } from "./fixtures/docx-xml.js";
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

async function collectTree(directory: string): Promise<Array<[string, string]>> {
    const entries = await readdir(directory, { withFileTypes: true });
    const tree: Array<[string, string]> = [];
    for (const entry of entries) {
        const full = join(directory, entry.name);
        if (entry.isDirectory()) {
            for (const [relative, digest] of await collectTree(full)) {
                tree.push([join(entry.name, relative), digest]);
            }
        } else {
            tree.push([entry.name, (await readFile(full)).toString("base64")]);
        }
    }
    return tree.sort(([left], [right]) => left.localeCompare(right));
}

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
        assert.deepEqual(lossCodes, [
            "export.markdown_project.navigation_omitted",
            "import.epub.asset_unresolved",
            "import.epub.partial_coverage",
        ]);
        const navigationLoss = (report.losses as Array<{ code: string; message: string }>).find(
            (loss) => loss.code === "export.markdown_project.navigation_omitted",
        );
        assert.ok(navigationLoss?.message.includes("OEBPS/nav.xhtml"), "the nav loss must name the skipped member");
        assert.deepEqual(report.unresolved_assets, ["OEBPS/images/missing.png"]);
        assert.equal(
            lossCodes.filter((code) => code === "import.epub.asset_unresolved").length,
            report.unresolved_assets.length,
            "every unresolved asset must be counted by the matching IR loss",
        );
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

        const again = await converter.toMarkdown(
            projectBookEpub(),
            defineConversion({
                id: "epub-project-again",
                input: { format: "epub", label: "project-book.epub" },
                output: { format: "markdown" },
                publish: { kind: "markdown-project", directory: join(workdir, "book-again") },
            }),
        );
        assert.equal(again.status, "success_with_loss");
        assert.deepEqual(
            await collectTree(join(workdir, "book-again")),
            await collectTree(outputDir),
            "the same EPUB input must publish a byte-identical project tree",
        );
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

        const reopened = await new Converter().toMarkdown(
            index,
            defineConversion({
                id: "doc-project-reopen",
                input: { format: "markdown", label: "index.md" },
                output: { format: "markdown" },
            }),
        );
        assert.notEqual(reopened.status, "failed");
        assert.match(reopened.text, /Hello legacy DOC/);
        assert.match(reopened.text, /Second paragraph/);
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

        const reopened = await converter.toMarkdown(
            index,
            defineConversion({
                id: "pdf-project-reopen",
                input: { format: "markdown", label: "index.md" },
                output: { format: "markdown" },
            }),
        );
        assert.notEqual(reopened.status, "failed");
        assert.match(reopened.text, /# Report/);
        assert.match(reopened.text, /Hello PDF paragraph\./);
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("docx project route materializes the embedded image, matches the plain losses and reopens", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-project-docx-"));
    const outputDir = join(workdir, "document");
    const logoBytes = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x00, 0x01, 0x02]);
    const makeDocx = () =>
        storedZip([
            { path: "word/document.xml", payload: imageDocumentXml },
            { path: "word/_rels/document.xml.rels", payload: imageRelsXml },
            { path: "word/media/logo.png", payload: logoBytes },
        ]);
    try {
        const converter = new Converter();
        const projectPlan = defineConversion({
            id: "docx-project-check",
            input: { format: "docx", label: "image.docx" },
            output: { format: "markdown" },
            publish: { kind: "markdown-project", directory: outputDir },
        });
        const plainPlan = defineConversion({
            id: "docx-project-plain",
            input: { format: "docx", label: "image.docx" },
            output: { format: "markdown" },
        });

        const project = await converter.toMarkdown(makeDocx(), projectPlan);
        const plain = await converter.toMarkdown(makeDocx(), plainPlan);
        assert.notEqual(project.status, "failed", JSON.stringify(project.diagnostics));
        assert.equal(project.publishPath, outputDir);

        const index = await readFile(join(outputDir, "index.md"), "utf8");
        const assetRef = index.match(/!\[Logo\]\(assets\/([0-9a-f]{16}\.png)\)/);
        assert.ok(assetRef, `the embedded image must become a content-addressed asset: ${index}`);
        assert.deepEqual(await readFile(join(outputDir, "assets", assetRef[1])), logoBytes);
        assert.doesNotMatch(index, /word\/media\/logo\.png/, "the package path must be rewritten");

        const projectReport = JSON.parse(project.reportJson);
        const plainReport = JSON.parse(plain.reportJson);
        assert.equal(projectReport.pipeline.writer, "markdown-project");
        assert.deepEqual(projectReport.unresolved_assets, [], "a materialized image is not an unresolved asset");
        const projectLosses = (projectReport.losses as Array<{ code: string }>).map((loss) => loss.code).sort();
        const plainLosses = (plainReport.losses as Array<{ code: string }>).map((loss) => loss.code).sort();
        assert.deepEqual(projectLosses, plainLosses, "the project report must declare the same DOCX import losses");
        assert.deepEqual(projectReport.coverage, plainReport.coverage);
        const assetOutput = projectReport.outputs.find((output: { path?: string }) => output.path === "assets/");
        assert.equal(assetOutput?.published_asset_count, 1);

        const reopened = await converter.toMarkdown(
            index,
            defineConversion({
                id: "docx-project-reopen",
                input: { format: "markdown", label: "index.md" },
                output: { format: "markdown" },
            }),
        );
        assert.notEqual(reopened.status, "failed");
        assert.match(reopened.text, /!\[Logo\]\(assets\/[0-9a-f]{16}\.png\)/, "the asset reference must survive reopen");
        assert.doesNotMatch(reopened.text, /word\/media\/logo\.png/);
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

/** A minimal PDF whose content stream paints text and one DCT JPEG XObject. */
const imagePagePdf = Buffer.from(
    "%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /XObject << /Im1 7 0 R >> >> /Contents 4 0 R >>\nendobj\n4 0 obj\n<< /Length 36 >>\nstream\nBT (Image page body.) Tj ET\n/Im1 Do\nendstream\nendobj\n7 0 obj\n<< /Subtype /Image /Filter /DCTDecode /Width 1 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Length 4 >>\nstream\n\xFF\xD8\xFF\xD9\nendstream\nendobj\n%%EOF",
    "latin1",
);

const imagePageJpeg = Buffer.from([0xff, 0xd8, 0xff, 0xd9]);

test("pdf project route materializes an image XObject and keeps text plus explicit partial loss", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-project-pdf-image-"));
    const outputDir = join(workdir, "report");
    try {
        const converter = new Converter();
        const project = await converter.toMarkdown(
            imagePagePdf,
            defineConversion({
                id: "pdf-image-project-check",
                input: { format: "pdf", label: "image-page.pdf" },
                output: { format: "markdown" },
                publish: { kind: "markdown-project", directory: outputDir },
            }),
        );
        const plain = await converter.toMarkdown(
            imagePagePdf,
            defineConversion({
                id: "pdf-image-project-plain",
                input: { format: "pdf", label: "image-page.pdf" },
                output: { format: "markdown" },
            }),
        );
        assert.equal(project.status, "success_with_loss", JSON.stringify(project.diagnostics));

        const index = await readFile(join(outputDir, "index.md"), "utf8");
        assert.match(index, /Image page body\./);
        const assetRef = index.match(/!\[\]\(assets\/([0-9a-f]{16}\.[a-z0-9]+)\)/);
        assert.ok(assetRef, `the PDF image XObject must become a content-addressed asset: ${index}`);
        assert.deepEqual(await readFile(join(outputDir, "assets", assetRef[1])), imagePageJpeg);

        const projectReport = JSON.parse(project.reportJson);
        const plainReport = JSON.parse(plain.reportJson);
        const projectLosses = (projectReport.losses as Array<{ code: string }>).map((loss) => loss.code).sort();
        const plainLosses = (plainReport.losses as Array<{ code: string }>).map((loss) => loss.code).sort();
        assert.ok(
            projectLosses.every((code) => code.startsWith("import.pdf.")),
            `the PDF reimport must only declare PDF import losses: ${projectLosses.join(", ")}`,
        );
        assert.ok(projectLosses.includes("import.pdf.partial_coverage"));
        assert.deepEqual(projectLosses, plainLosses, "the project report must declare the same PDF import losses");
        assert.deepEqual(projectReport.coverage, plainReport.coverage);
        assert.deepEqual(projectReport.unresolved_assets, []);
        const assetOutput = projectReport.outputs.find((output: { path?: string }) => output.path === "assets/");
        assert.equal(assetOutput?.published_asset_count, 1);

        const reopened = await converter.toMarkdown(
            index,
            defineConversion({
                id: "pdf-image-project-reopen",
                input: { format: "markdown", label: "index.md" },
                output: { format: "markdown" },
            }),
        );
        assert.notEqual(reopened.status, "failed");
        assert.match(reopened.text, /Image page body\./);
        assert.match(reopened.text, /!\[\]\(assets\/[0-9a-f]{16}\.[a-z0-9]+\)/);
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});
