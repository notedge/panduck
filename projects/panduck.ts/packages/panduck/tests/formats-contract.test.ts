import assert from "node:assert/strict";
import { mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { runPanduck } from "./helpers/cli.js";
import { storedZip } from "./helpers/stored-zip.js";

const WORDPROCESSING_NS = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";

type LossRecord = { code?: string; status?: string };

type ConvertReport = {
    status?: string;
    losses?: LossRecord[];
};

function documentXml(body: string): Buffer {
    return Buffer.from(
        `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="${WORDPROCESSING_NS}"><w:body>${body}</w:body></w:document>`,
        "utf8",
    );
}

function numberingXml(format: string): Buffer {
    return Buffer.from(
        `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="${WORDPROCESSING_NS}">
  <w:abstractNum w:abstractNumId="0"><w:lvl w:ilvl="0"><w:numFmt w:val="${format}"/></w:lvl></w:abstractNum>
  <w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num>
</w:numbering>`,
        "utf8",
    );
}

function listBody(): string {
    return [
        '<w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t>One</w:t></w:r></w:p>',
        '<w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t>Two</w:t></w:r></w:p>',
    ].join("");
}

async function convertDocx(
    workdir: string,
    label: string,
    archive: Buffer,
): Promise<{ report: ConvertReport; markdown: string }> {
    const input = join(workdir, `${label}.docx`);
    const output = join(workdir, `${label}.md`);
    await writeFile(input, archive);
    const run = runPanduck([
        "convert",
        input,
        "--from",
        "docx",
        "--to",
        "markdown",
        "-o",
        output,
        "--report",
        "-",
        "--diagnostics",
        "silent",
    ]);
    assert.equal(run.code, 0, run.stderr || run.stdout);
    return {
        report: JSON.parse(run.stdout) as ConvertReport,
        markdown: await readFile(output, "utf8"),
    };
}

test("docx import keeps bold and italic runs", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-formats-"));
    const archive = storedZip([
        {
            path: "word/document.xml",
            payload: documentXml(
                "<w:p><w:r><w:rPr><w:b/></w:rPr><w:t>Bold</w:t></w:r>" +
                    "<w:r><w:rPr><w:i/></w:rPr><w:t> italic</w:t></w:r></w:p>",
            ),
        },
    ]);
    const { markdown } = await convertDocx(workdir, "styled", archive);
    assert.match(markdown, /\*\*Bold\*\*/, markdown);
    assert.match(markdown, /\* italic\*/, markdown);
});

test("docx import resolves footnote references and their bodies", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-formats-"));
    const archive = storedZip([
        {
            path: "word/document.xml",
            payload: documentXml(
                '<w:p><w:r><w:t>See</w:t></w:r><w:r><w:footnoteReference w:id="1"/></w:r>' +
                    "<w:r><w:t> for details.</w:t></w:r></w:p>",
            ),
        },
        {
            path: "word/footnotes.xml",
            payload: Buffer.from(
                `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:footnotes xmlns:w="${WORDPROCESSING_NS}">
  <w:footnote w:id="1"><w:p><w:r><w:t>Footnote body.</w:t></w:r></w:p></w:footnote>
</w:footnotes>`,
                "utf8",
            ),
        },
    ]);
    const { markdown, report } = await convertDocx(workdir, "footnote", archive);
    assert.ok(markdown.includes("See[^1] for details."), markdown);
    assert.match(markdown, /Footnote body\./, markdown);
    assert.ok(
        !report.losses?.some((loss) => loss.code === "reader.docx.footnote_body"),
        `resolved footnote bodies must not be reported as losses: ${JSON.stringify(report.losses)}`,
    );
});

test("docx import reports a missing footnote body as an unresolved loss", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-formats-"));
    const archive = storedZip([
        {
            path: "word/document.xml",
            payload: documentXml(
                '<w:p><w:r><w:t>See</w:t></w:r><w:r><w:footnoteReference w:id="1"/></w:r>' +
                    "<w:r><w:t> for details.</w:t></w:r></w:p>",
            ),
        },
    ]);
    const { report } = await convertDocx(workdir, "orphan-footnote", archive);
    const loss = report.losses?.find((entry) => entry.code === "reader.docx.footnote_body");
    assert.ok(loss, `expected reader.docx.footnote_body, got ${JSON.stringify(report.losses)}`);
    assert.equal(loss.status, "Unresolved");
});

test("docx numbering maps decimal to ordered and bullet to unordered lists", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-formats-"));
    for (const [label, format, expected] of [
        ["decimal", "decimal", /1\. One/],
        ["bullet", "bullet", /- One/],
    ] as const) {
        const archive = storedZip([
            { path: "word/document.xml", payload: documentXml(listBody()) },
            { path: "word/numbering.xml", payload: numberingXml(format) },
        ]);
        const { markdown } = await convertDocx(workdir, label, archive);
        assert.match(markdown, expected, `${label}: ${markdown}`);
    }
});
