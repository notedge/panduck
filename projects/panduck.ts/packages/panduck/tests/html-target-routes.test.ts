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
    status: string;
    losses: Array<{ code: string; status: string }>;
};

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

    return {
        artifact: await readFile(artifactPath),
        markdown: await readFile(backPath, "utf8"),
        report: JSON.parse(await readFile(reportPath, "utf8")) as RouteReport,
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

for (const to of ["docx", "pdf", "epub"] as const) {
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
                artifact.subarray(0, to === "pdf" ? 5 : 2).toString().startsWith(to === "pdf" ? "%PDF-" : "PK"),
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
