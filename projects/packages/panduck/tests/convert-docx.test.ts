import assert from "node:assert/strict";
import { mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import {
    hyperlinkDocumentXml,
    hyperlinkRelsXml,
    imageDocumentXml,
    imageRelsXml,
    minimalDocumentXml,
} from "./fixtures/docx-xml.js";
import { runPanduck } from "./helpers/cli.js";
import { storedZip } from "./helpers/stored-zip.js";

async function convertDocx(
    entries: ReadonlyArray<{ path: string; payload: Uint8Array | Buffer }>,
): Promise<string> {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-docx-"));
    const input = join(workdir, "sample.docx");
    const output = join(workdir, "sample.md");
    await writeFile(input, storedZip(entries));

    const result = runPanduck([
        "convert",
        input,
        "--to",
        "markdown",
        "-o",
        output,
        "--diagnostics",
        "silent",
    ]);
    assert.equal(result.code, 0, result.stderr || result.stdout);
    return readFile(output, "utf8");
}

test("convert docx to markdown through CLI", async () => {
    const markdown = await convertDocx([{ path: "word/document.xml", payload: minimalDocumentXml }]);
    assert.match(markdown, /Hello DOCX/);
    assert.match(markdown, /# Title/);
});

test("convert docx hyperlinks to markdown links", async () => {
    const markdown = await convertDocx([
        { path: "word/document.xml", payload: hyperlinkDocumentXml },
        { path: "word/_rels/document.xml.rels", payload: hyperlinkRelsXml },
    ]);
    assert.match(markdown, /\[Example\]\(https:\/\/example\.com\)/);
});

test("convert docx embedded images to markdown images", async () => {
    const markdown = await convertDocx([
        { path: "word/document.xml", payload: imageDocumentXml },
        { path: "word/_rels/document.xml.rels", payload: imageRelsXml },
        { path: "media/logo.png", payload: Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a]) },
    ]);
    assert.match(markdown, /!\[Logo\]\(media\/logo\.png\)/);
});
