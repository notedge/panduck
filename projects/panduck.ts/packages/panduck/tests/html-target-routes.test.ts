import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

import { runPanduck } from "./helpers/cli.js";
import { storedZip } from "./helpers/stored-zip.js";
import { minimalDocumentXml } from "./fixtures/docx-xml.js";

const legacyDocPath = join(
    dirname(fileURLToPath(import.meta.url)),
    "../../panduck-fixtures/fixtures/acorn/doc/legacy_plain.doc",
);

const fixturesRoot = join(dirname(fileURLToPath(import.meta.url)), "../../panduck-fixtures/fixtures");

const sourceFixtures: Record<string, string> = {
    tex: join(fixturesRoot, "formats/tex/sections.tex"),
    typst: join(fixturesRoot, "formats/typst/heading.typ"),
};

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

const pageParagraphHtml =
    'Text with <strong>weight</strong> and a <a href="https://example.com/doc">link</a>.';

const routePage = `<!doctype html>
<html>
  <head><title>Route Page</title></head>
  <body>
    <h1>Route Heading</h1>
    <p>${pageParagraphHtml}</p>
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
    operation: string;
    status: string;
    inputs: Array<{ path: string; format?: string }>;
    outputs: Array<{ path: string; published: boolean }>;
    losses: Array<{ code: string; status: string }>;
    diagnostics?: Array<{ code: string; severity: string }>;
    coverage: { state: string } | null;
};

/**
 * Locks the report contract every route must satisfy, independent of how much
 * of a particular target format is implemented.
 */
function assertReportInvariants(report: RouteReport, from: string, to: string): void {
    const label = `${from}->${to}`;
    assert.equal(report.operation, "convert", `${label}: operation`);
    assert.equal(report.inputs.length, 1, `${label}: one input`);
    assert.equal(report.inputs[0]?.format, from, `${label}: input format`);
    assert.ok(report.outputs.length > 0, `${label}: at least one output`);
    assert.ok(
        report.outputs.every((output) => output.published),
        `${label}: every listed output is published`,
    );
    assert.ok(report.coverage, `${label}: a conversion report must carry coverage`);
    assert.ok(
        ["ready", "partial"].includes(report.coverage?.state ?? ""),
        `${label}: an executed route cannot report unavailable coverage`,
    );
    assert.ok(
        ["success", "success_with_loss"].includes(report.status),
        `${label}: an executed route cannot report blocked`,
    );
    if (report.status === "success") {
        assert.equal(report.losses.length, 0, `${label}: success cannot carry losses`);
    } else {
        assert.ok(report.losses.length > 0, `${label}: success_with_loss must declare its losses`);
    }
    for (const loss of report.losses) {
        assert.ok(loss.code.length > 0, `${label}: every loss must carry a stable code`);
    }
    for (const diagnostic of report.diagnostics ?? []) {
        assert.ok(diagnostic.code.length > 0, `${label}: every diagnostic must carry a stable code`);
        assert.ok(diagnostic.severity.length > 0, `${label}: every diagnostic must carry a severity`);
    }
}

/**
 * Converts one input to the target format and reopens the written artifact
 * through that format's own import path back to Markdown.
 *
 * Reopening the artifact rather than trusting writer success is the point of
 * these route probes: a route may report `published` while losing a heading,
 * an inline style or a link target.
 */
async function convertAndReopen(input: {
    workdir: string;
    bytes: Buffer;
    label: string;
    from: string;
    to: string;
}): Promise<{ artifact: Buffer; markdown: string; report: RouteReport }> {
    const { workdir, bytes, label, from, to } = input;
    const sourcePath = join(workdir, label);
    const artifactPath = join(workdir, `${from}-to-${to}.${to}`);
    const reportPath = join(workdir, `${from}-to-${to}.report.json`);
    const backPath = join(workdir, `${from}-to-${to}.md`);
    await writeFile(sourcePath, bytes);

    const convert = runPanduck([
        "convert",
        sourcePath,
        "--from",
        from,
        "--to",
        to,
        "-o",
        artifactPath,
        "--report",
        reportPath,
        "--diagnostics",
        "silent",
    ]);
    assert.equal(convert.code, 0, convert.stderr || convert.stdout);

    const reopen = runPanduck([
        "convert",
        artifactPath,
        "--from",
        to,
        "--to",
        "markdown",
        "-o",
        backPath,
        "--diagnostics",
        "silent",
    ]);
    assert.equal(reopen.code, 0, reopen.stderr || reopen.stdout);

    const parsedReport = JSON.parse(await readFile(reportPath, "utf8")) as RouteReport;
    assertReportInvariants(parsedReport, from, to);

    return {
        artifact: await readFile(artifactPath),
        markdown: await readFile(backPath, "utf8"),
        report: parsedReport,
    };
}

test("epub to html writes page blocks and reopens as the same heading and link", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-epub-html-"));
    try {
        const { artifact, markdown, report } = await convertAndReopen({
            workdir,
            bytes: bookEpub(),
            label: "route-book.epub",
            from: "epub",
            to: "html",
        });
        const html = artifact.toString("utf8");

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
        const { artifact, markdown, report } = await convertAndReopen({
            workdir,
            bytes: headingDocx(),
            label: "route.docx",
            from: "docx",
            to: "html",
        });
        const html = artifact.toString("utf8");

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

test("pdf to html writes the inferred heading and text layer and reopens as the same outline", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-pdf-html-"));
    try {
        const markdownPath = join(workdir, "route.md");
        const pdfPath = join(workdir, "route.pdf");
        await writeFile(markdownPath, "# Route Doc\n\nBody with weight and a link.\n");
        const toPdf = runPanduck([
            "convert",
            markdownPath,
            "--from",
            "markdown",
            "--to",
            "pdf",
            "-o",
            pdfPath,
            "--diagnostics",
            "silent",
        ]);
        assert.equal(toPdf.code, 0, toPdf.stderr || toPdf.stdout);

        const { artifact, markdown, report } = await convertAndReopen({
            workdir,
            bytes: await readFile(pdfPath),
            label: "route.pdf",
            from: "pdf",
            to: "html",
        });
        const html = artifact.toString("utf8");

        assert.match(html, /<h1[^>]*>Route Doc<\/h1>/, "the inferred heading must reach the page");
        assert.match(html, /<p[^>]*>Body with weight and a link\.<\/p>/, "the text layer must reach the page");

        assert.match(markdown, /^# Route Doc$/m, "reopening must recover the inferred heading level");
        assert.match(markdown, /^Body with weight and a link\.$/m, "reopening must recover the text layer");

        assert.equal(report.status, "success_with_loss", "partial coverage must be declared, not hidden");
        assert.deepEqual(
            report.losses.map((loss) => loss.code),
            [
                "import.pdf.text_spacing",
                "import.pdf.heading_level_inferred",
                "import.pdf.footnote_semantics",
                "import.pdf.partial_coverage",
            ],
            "PDF import must keep declaring the same partial-coverage facts",
        );
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("html to epub keeps the heading, styled run and link through EPUB import", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-html-epub-"));
    try {
        const { markdown, report } = await convertAndReopen({
            workdir,
            bytes: Buffer.from(routePage),
            label: "route.html",
            from: "html",
            to: "epub",
        });

        assert.match(markdown, /^# Route Heading$/m, "reopening must recover the heading level");
        assert.match(markdown, /Text with \*\*weight\*\* and a \[link\]\(https:\/\/example\.com\/doc\)\./);

        assert.equal(report.status, "success_with_loss", "partial coverage must be declared, not hidden");
        assert.deepEqual(
            report.losses.map((loss) => loss.code),
            ["import.html.partial_coverage"],
            "the only declared loss must be the HTML import partial coverage marker",
        );
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("notedown to html writes the heading, inline style, link and list then reopens them", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-notedown-html-"));
    try {
        const { artifact, markdown, report } = await convertAndReopen({
            workdir,
            bytes: Buffer.from(
                "# Note Title\n\nBody with **bold** and a [link](https://example.com/doc).\n\n- one\n- two\n",
            ),
            label: "route.nd",
            from: "notedown",
            to: "html",
        });
        const html = artifact.toString("utf8");

        assert.match(html, /<h1>Note Title<\/h1>/, "the heading must reach the page");
        assert.match(html, /<strong>bold<\/strong>/, "the styled run must stay strong");
        assert.match(html, /<a href="https:\/\/example\.com\/doc">link<\/a>/, "the link must keep its target");
        assert.match(html, /<ul><li>one<\/li><li>two<\/li><\/ul>/, "the list must keep both items");

        assert.match(markdown, /^# Note Title$/m, "reopening must recover the heading level");
        assert.match(markdown, /Body with \*\*bold\*\* and a \[link\]\(https:\/\/example\.com\/doc\)\./);
        assert.match(markdown, /^- one$/m, "reopening must recover the list item");
        assert.match(markdown, /^- two$/m, "reopening must recover the list item");

        assert.equal(report.status, "success_with_loss", "partial coverage must be declared, not hidden");
        assert.deepEqual(
            report.losses.map((loss) => loss.code),
            ["import.notedown.partial_coverage"],
            "the only declared loss must be the notedown import partial coverage marker",
        );
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("html to pdf writes the heading and text layer and reopens as the same outline", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-html-pdf-"));
    try {
        const { artifact, markdown, report } = await convertAndReopen({
            workdir,
            bytes: Buffer.from(routePage),
            label: "route.html",
            from: "html",
            to: "pdf",
        });

        assert.match(artifact.subarray(0, 5).toString(), /^%PDF-/, "the artifact must be a PDF file");
        assert.match(markdown, /^# Route Heading$/m, "reopening must recover the heading level");
        assert.match(
            markdown,
            /^Text with weight and a link\.$/m,
            "reopening must recover the text layer",
        );

        assert.equal(report.status, "success_with_loss", "partial coverage must be declared, not hidden");
        assert.deepEqual(
            report.losses.map((loss) => loss.code),
            ["import.html.partial_coverage"],
            "the only declared loss must be the HTML import partial coverage marker",
        );
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("docx to pdf writes the heading and paragraph and reopens as the same structure", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-docx-pdf-"));
    try {
        const { artifact, markdown, report } = await convertAndReopen({
            workdir,
            bytes: headingDocx(),
            label: "route.docx",
            from: "docx",
            to: "pdf",
        });

        assert.match(artifact.subarray(0, 5).toString(), /^%PDF-/, "the artifact must be a PDF file");
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

test("pdf to docx writes the inferred heading and text layer and reopens as the same outline", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-pdf-docx-"));
    try {
        const markdownPath = join(workdir, "route.md");
        const pdfPath = join(workdir, "route.pdf");
        await writeFile(markdownPath, "# Route Doc\n\nBody with weight and a link.\n");
        const toPdf = runPanduck([
            "convert",
            markdownPath,
            "--from",
            "markdown",
            "--to",
            "pdf",
            "-o",
            pdfPath,
            "--diagnostics",
            "silent",
        ]);
        assert.equal(toPdf.code, 0, toPdf.stderr || toPdf.stdout);

        const { artifact, markdown, report } = await convertAndReopen({
            workdir,
            bytes: await readFile(pdfPath),
            label: "route.pdf",
            from: "pdf",
            to: "docx",
        });

        assert.match(artifact.subarray(0, 2).toString(), /^PK/, "the artifact must be a DOCX package");
        assert.match(markdown, /^# Route Doc$/m, "reopening must recover the inferred heading level");
        assert.match(markdown, /^Body with weight and a link\.$/m, "reopening must recover the text layer");

        assert.equal(report.status, "success_with_loss", "partial coverage must be declared, not hidden");
        assert.deepEqual(
            report.losses.map((loss) => loss.code),
            [
                "import.pdf.text_spacing",
                "import.pdf.heading_level_inferred",
                "import.pdf.footnote_semantics",
                "import.pdf.partial_coverage",
            ],
            "PDF import must keep declaring the same partial-coverage facts",
        );
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("html to docx keeps the heading, styled run and link through DOCX import", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-html-docx-"));
    try {
        const { markdown, report } = await convertAndReopen({
            workdir,
            bytes: Buffer.from(routePage),
            label: "route.html",
            from: "html",
            to: "docx",
        });

        assert.match(markdown, /^# Route Heading$/m, "reopening must recover the heading level");
        assert.match(markdown, /Text with \*\*weight\*\* and a \[link\]\(https:\/\/example\.com\/doc\)\./);

        assert.equal(report.status, "success_with_loss", "partial coverage must be declared, not hidden");
        assert.deepEqual(
            report.losses.map((loss) => loss.code),
            ["import.html.partial_coverage"],
            "the only declared loss must be the HTML import partial coverage marker",
        );
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

const docTargets = [
    { to: "markdown", prefix: undefined },
    { to: "html", prefix: "<" },
    { to: "docx", prefix: "PK" },
    { to: "pdf", prefix: "%PDF-" },
    { to: "epub", prefix: "PK" },
] as const;

for (const { to, prefix } of docTargets) {
    test(`doc to ${to} keeps both paragraphs through ${to} reopen`, async () => {
        const workdir = await mkdtemp(join(tmpdir(), `panduck-doc-${to}-`));
        try {
            const { artifact, markdown, report } = await convertAndReopen({
                workdir,
                bytes: await readFile(legacyDocPath),
                label: "legacy.doc",
                from: "doc",
                to,
            });

            if (prefix) {
                assert.ok(
                    artifact.subarray(0, prefix.length).toString().startsWith(prefix),
                    `the artifact must be a ${to} file`,
                );
            }

            assert.match(markdown, /Hello legacy DOC/, "reopening must recover the first paragraph");
            assert.match(markdown, /Second paragraph/, "reopening must recover the second paragraph");

            assert.equal(report.status, "success_with_loss", "partial coverage must be declared, not hidden");
            assert.deepEqual(
                report.losses.map((loss) => loss.code),
                ["import.doc.partial_coverage"],
                "the only declared loss must be the DOC import partial coverage marker",
            );
        } finally {
            await rm(workdir, { recursive: true, force: true });
        }
    });
}

/** Writes a markdown seed, converts it to PDF, and returns the PDF bytes. */
async function seedPdf(workdir: string): Promise<Buffer> {
    const markdownPath = join(workdir, "seed.md");
    const pdfPath = join(workdir, "seed.pdf");
    await writeFile(markdownPath, "# Route Doc\n\nBody with weight and a link.\n");
    const converted = runPanduck([
        "convert",
        markdownPath,
        "--from",
        "markdown",
        "--to",
        "pdf",
        "-o",
        pdfPath,
        "--diagnostics",
        "silent",
    ]);
    assert.equal(converted.code, 0, converted.stderr || converted.stdout);
    return readFile(pdfPath);
}

const pdfLossCodes = [
    "import.pdf.text_spacing",
    "import.pdf.heading_level_inferred",
    "import.pdf.footnote_semantics",
    "import.pdf.partial_coverage",
];

for (const to of ["docx", "epub"] as const) {
    test(`docx to ${to} keeps the heading and paragraph through ${to} reopen`, async () => {
        const workdir = await mkdtemp(join(tmpdir(), `panduck-docx-${to}-`));
        try {
            const { artifact, markdown, report } = await convertAndReopen({
                workdir,
                bytes: headingDocx(),
                label: "route.docx",
                from: "docx",
                to,
            });

            assert.ok(artifact.subarray(0, 2).toString().startsWith("PK"), `the artifact must be a ${to} package`);
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
}

for (const to of ["pdf", "epub"] as const) {
    test(`pdf to ${to} keeps the inferred heading and text layer through ${to} reopen`, async () => {
        const workdir = await mkdtemp(join(tmpdir(), `panduck-pdf-${to}-`));
        try {
            const { artifact, markdown, report } = await convertAndReopen({
                workdir,
                bytes: await seedPdf(workdir),
                label: "route.pdf",
                from: "pdf",
                to,
            });

            assert.ok(
                artifact.subarray(0, to === "pdf" ? 5 : 2).toString().startsWith(to === "pdf" ? "%PDF-" : "PK"),
                `the artifact must be a ${to} file`,
            );
            assert.match(markdown, /^# Route Doc$/m, "reopening must recover the inferred heading level");
            assert.match(markdown, /Body with weight and a link\./, "reopening must recover the text layer");

            assert.equal(report.status, "success_with_loss", "partial coverage must be declared, not hidden");
            assert.deepEqual(
                report.losses.map((loss) => loss.code),
                pdfLossCodes,
                "PDF import must keep declaring the same partial-coverage facts",
            );
        } finally {
            await rm(workdir, { recursive: true, force: true });
        }
    });
}

/** Writes a long single-flow HTML seed and returns a multi-page PDF from it. */
async function seedPagedPdf(workdir: string): Promise<Buffer> {
    const paragraphs = Array.from(
        { length: 160 },
        (_, index) => `Paragraph ${index + 1} keeps the page long enough to spill over.`,
    ).join("\n\n");
    const htmlPath = join(workdir, "seed-paged.html");
    const pdfPath = join(workdir, "seed-paged.pdf");
    await writeFile(
        htmlPath,
        `<!doctype html><html><head><title>Chrome Title</title></head><body><h1>Body Heading</h1><p>${paragraphs}</p></body></html>`,
    );
    const converted = runPanduck([
        "convert",
        htmlPath,
        "--from",
        "html",
        "--to",
        "pdf",
        "-o",
        pdfPath,
        "--diagnostics",
        "silent",
    ]);
    assert.equal(converted.code, 0, converted.stderr || converted.stdout);
    return readFile(pdfPath);
}

test("pdf to pdf keeps the running chrome out of the reopened body", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-pdf-pdf-chrome-"));
    try {
        const { artifact, markdown } = await convertAndReopen({
            workdir,
            bytes: await seedPagedPdf(workdir),
            label: "route.pdf",
            from: "pdf",
            to: "pdf",
        });

        const pages = (artifact.toString("latin1").match(/\/Type \/Page(?!s)/g) ?? []).length;
        assert.ok(pages >= 2, `the fixture must span pages, got ${pages}`);
        assert.doesNotMatch(markdown, /Chrome Title/, "the running header must not reopen as body text");
        assert.doesNotMatch(markdown, /^\s*\d+\s*$/m, "the folio must not reopen as body text");
        assert.match(markdown, /Paragraph 160/, "the authored body must survive");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

for (const to of ["pdf", "epub"] as const) {
    test(`epub to ${to} keeps the chapter heading and text through ${to} reopen`, async () => {
        const workdir = await mkdtemp(join(tmpdir(), `panduck-epub-${to}-`));
        try {
            const { artifact, markdown, report } = await convertAndReopen({
                workdir,
                bytes: bookEpub(),
                label: "route.epub",
                from: "epub",
                to,
            });

            assert.ok(
                artifact.subarray(0, to === "pdf" ? 5 : 2).toString().startsWith(to === "pdf" ? "%PDF-" : "PK"),
                `the artifact must be a ${to} file`,
            );
            assert.match(markdown, /^# Chapter Body$/m, "reopening must recover the chapter heading");
            if (to === "epub") {
                assert.match(
                    markdown,
                    /Text with \*\*weight\*\* and a \[link\]\(https:\/\/example\.com\/doc\)\./,
                    "the epub writer must keep styled inline text and the link",
                );
            } else {
                assert.match(
                    markdown,
                    /Text with weight and a link\./,
                    "the pdf text layer must keep the running text after inline styles are flattened",
                );
            }

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
}

test("html to html keeps the heading, styled run and link through HTML reopen", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-html-html-"));
    try {
        const { artifact, markdown, report } = await convertAndReopen({
            workdir,
            bytes: Buffer.from(routePage),
            label: "route.html",
            from: "html",
            to: "html",
        });

        assert.match(artifact.toString("utf8"), /<h1>Route Heading<\/h1>/, "the page must reach the writer");
        assert.match(
            artifact.toString("utf8"),
            /data-document-title="Route Page"/,
            "the head title must stay in document metadata instead of the body",
        );
        assert.doesNotMatch(
            markdown,
            /Route Page/,
            "reopening must not turn the head title into a body paragraph",
        );
        assert.match(markdown, /^# Route Heading$/m, "reopening must recover the heading level");
        assert.match(
            markdown,
            /Text with \*\*weight\*\* and a \[link\]\(https:\/\/example\.com\/doc\)\./,
            "reopening must recover the styled run and the link target",
        );

        assert.equal(report.status, "success_with_loss", "partial coverage must be declared, not hidden");
        assert.deepEqual(
            report.losses.map((loss) => loss.code),
            ["import.html.partial_coverage"],
            "the only declared loss must be the HTML import partial coverage marker",
        );
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("html table headers survive the writer and the markdown reopen", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-html-table-header-"));
    try {
        const page = `<!doctype html>
<html><head><title>Table Page</title></head>
<body>
<h1>Table Heading</h1>
<table>
<thead><tr><th>Name</th><th>Value</th></tr></thead>
<tbody><tr><td>Alpha</td><td>1</td></tr><tr><td>Beta</td><td>2</td></tr></tbody>
</table>
</body></html>`;
        const { artifact, markdown, report } = await convertAndReopen({
            workdir,
            bytes: Buffer.from(page),
            label: "route.html",
            from: "html",
            to: "html",
        });

        const text = artifact.toString("utf8");
        assert.match(text, /<thead>/, "the writer must keep the header section");
        assert.match(text, /<th[^>]*><p>Name<\/p><\/th>/, "the writer must keep the header cells as paragraph children");
        assert.match(markdown, /^\| Name \| Value \|$/m, "reopening must recover the header row");
        assert.match(markdown, /^\| --- \| --- \|$/m, "reopening must emit a Markdown header separator");
        assert.match(markdown, /^\| Alpha \| 1 \|$/m, "reopening must recover the body row");

        assert.equal(report.status, "success_with_loss", "partial coverage must be declared, not hidden");
        assert.deepEqual(
            report.losses.map((loss) => loss.code),
            ["import.html.partial_coverage"],
            "a leading header table must not declare a table section loss",
        );
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("html header rows outside the leading run are marked as head after body", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-html-table-sections-"));
    try {
        const page = `<!doctype html>
<html><body>
<table><tbody><tr><td>Body</td></tr></tbody><thead><tr><th>Late</th></tr></thead></table>
</body></html>`;
        const { markdown, report } = await convertAndReopen({
            workdir,
            bytes: Buffer.from(page),
            label: "route.html",
            from: "html",
            to: "markdown",
        });

        assert.match(markdown, /^\| Body \|$/m, "the leading body row must reopen");
        assert.match(markdown, /^\| Late \|$/m, "the trailing header row must reopen");
        const codes = report.losses.map((loss) => loss.code);
        assert.ok(
            codes.includes("import.html.head_after_body"),
            "a head section authored after body rows must be declared",
        );
        assert.ok(
            !codes.includes("import.html.table_sections"),
            "the head band is representable and must not be reported as a table section loss",
        );
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

for (const to of ["markdown", "docx", "pdf", "epub"] as const) {
    test(`notedown to ${to} keeps the heading, prose and list through ${to} reopen`, async () => {
        const workdir = await mkdtemp(join(tmpdir(), `panduck-notedown-${to}-`));
        try {
            const { artifact, markdown, report } = await convertAndReopen({
                workdir,
                bytes: Buffer.from(
                    "# Note Title\n\nBody with **bold** and a [link](https://example.com/doc).\n\n- one\n- two\n",
                ),
                label: "route.nd",
                from: "notedown",
                to,
            });

            assert.ok(
                to === "markdown"
                    ? true
                    : artifact
                          .subarray(0, to === "pdf" ? 5 : 2)
                          .toString()
                          .startsWith(to === "pdf" ? "%PDF-" : "PK"),
                `the artifact must be a ${to} file`,
            );
            assert.match(markdown, /^# Note Title$/m, "reopening must recover the heading level");
            if (to === "pdf") {
                assert.match(markdown, /Body with bold and a link\./, "the pdf text layer must keep the running text");
            } else {
                assert.match(
                    markdown,
                    /Body with \*\*bold\*\* and a \[link\]\(https:\/\/example\.com\/doc\)\./,
                    "the writer must keep styled inline text and the link",
                );
            }
            assert.match(markdown, /^- one$/m, "reopening must recover the list item");
            assert.match(markdown, /^- two$/m, "reopening must recover the list item");

            assert.equal(report.status, "success_with_loss", "partial coverage must be declared, not hidden");
            assert.deepEqual(
                report.losses.map((loss) => loss.code),
                ["import.notedown.partial_coverage"],
                "the only declared loss must be the notedown import partial coverage marker",
            );
        } finally {
            await rm(workdir, { recursive: true, force: true });
        }
    });
}

test("notedown to epub keeps the authored chapter order through epub reopen", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-notedown-epub-order-"));
    try {
        const { markdown, report } = await convertAndReopen({
            workdir,
            bytes: Buffer.from(
                "Prelude line.\n\n# Alpha\n\nFirst body.\n\n# Beta\n\nSecond body.\n",
            ),
            label: "route.nd",
            from: "notedown",
            to: "epub",
        });

        const positions = [
            "Prelude line.",
            "# Alpha",
            "First body.",
            "# Beta",
            "Second body.",
        ].map((text) => markdown.indexOf(text));
        assert.ok(
            positions.every((position) => position >= 0),
            "every authored block must reopen from the EPUB spine",
        );
        assert.ok(positions[0] < positions[1], "the leading prelude must stay before the first heading");
        assert.ok(
            positions[1] < positions[2] && positions[2] < positions[3] && positions[3] < positions[4],
            "each heading must open the chapter that owns the following body",
        );

        assert.equal(report.status, "success_with_loss", "partial coverage must be declared, not hidden");
        assert.deepEqual(
            report.losses.map((loss) => loss.code),
            ["import.notedown.partial_coverage"],
            "the only declared loss must be the notedown import partial coverage marker",
        );
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

for (const from of ["tex", "typst"] as const) {
    for (const to of ["markdown", "docx", "pdf", "epub"] as const) {
        test(`${from} to ${to} keeps the authored text through ${to} reopen`, async () => {
            const workdir = await mkdtemp(join(tmpdir(), `panduck-${from}-${to}-`));
            try {
                const { artifact, markdown, report } = await convertAndReopen({
                    workdir,
                    bytes: await readFile(sourceFixtures[from]),
                    label: `route.${from}`,
                    from,
                    to,
                });

                if (to !== "markdown") {
                    assert.ok(
                        artifact
                            .subarray(0, to === "pdf" ? 5 : 2)
                            .toString()
                            .startsWith(to === "pdf" ? "%PDF-" : "PK"),
                        `the artifact must be a ${to} file`,
                    );
                }

                if (from === "tex") {
                    assert.match(markdown, /One/, "reopening must recover the section title");
                    assert.match(markdown, /Two/, "reopening must recover the subsection title");
                    assert.match(markdown, /Three/, "reopening must recover the subsubsection title");
                    assert.match(markdown, /Body\./, "reopening must recover the section body");
                } else {
                    assert.match(markdown, /^# Title$/m, "reopening must recover the heading level");
                    assert.match(markdown, /Intro text\./, "reopening must recover the first paragraph");
                    assert.match(markdown, /Second paragraph\./, "reopening must recover the second paragraph");
                }

                // TeX carries its section numbers and labels as inline attributes.
                // DOCX and PDF have no slot for inline attribute identity, and the
                // writers name that drop instead of hiding it, so the text survives
                // while the authored attributes are reported as lost.
                const expectedLosses =
                    from === "tex" && to === "docx"
                        ? ["export.docx.inline_attributes_dropped"]
                        : from === "tex" && to === "pdf"
                          ? [
                                "export.pdf.inline_attributes_dropped",
                                "export.pdf.inline_attributes_dropped",
                                "export.pdf.inline_attributes_dropped",
                            ]
                          : [];
                assert.deepEqual(
                    report.losses.map((loss) => loss.code),
                    expectedLosses,
                    "the declared losses must match the slots the target really has",
                );
                assert.equal(
                    report.status,
                    expectedLosses.length === 0 ? "success" : "success_with_loss",
                    "the status must follow the declared losses",
                );
            } finally {
                await rm(workdir, { recursive: true, force: true });
            }
        });
    }
}

test("tex to pdf keeps the three authored heading levels through pdf reopen", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-tex-pdf-levels-"));
    try {
        const { markdown, report } = await convertAndReopen({
            workdir,
            bytes: await readFile(sourceFixtures.tex),
            label: "route.tex",
            from: "tex",
            to: "pdf",
        });

        assert.match(markdown, /^# 1 One$/m, "the section must reopen at heading level one");
        assert.match(markdown, /^## 1\.1 Two$/m, "the subsection must reopen at heading level two");
        assert.match(markdown, /^### 1\.1\.1 Three$/m, "the subsubsection must reopen at heading level three");
        assert.match(markdown, /^Body\.$/m, "the section body must stay ahead of the subsection");

        // The heading levels reopen from the PDF outline, but the TeX number and
        // label attributes have no PDF slot, so the writer names them as dropped.
        assert.equal(report.status, "success_with_loss", "the named attribute drops must surface in the status");
        assert.deepEqual(
            report.losses.map((loss) => loss.code),
            [
                "export.pdf.inline_attributes_dropped",
                "export.pdf.inline_attributes_dropped",
                "export.pdf.inline_attributes_dropped",
            ],
            "the only declared losses must be the inline attribute drops",
        );
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});
