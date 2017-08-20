import assert from "node:assert/strict";
import { mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { runPanduck } from "./helpers/cli.js";
import { storedZip } from "./helpers/stored-zip.js";

const documentXml = Buffer.from(`<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Hello DOCX</w:t></w:r></w:p>
    <w:p>
      <w:pPr><w:pStyle w:val="Heading1"/></w:pPr>
      <w:r><w:t>Title</w:t></w:r>
    </w:p>
  </w:body>
</w:document>`, "utf8");

test("convert docx to markdown through CLI", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-docx-"));
    const input = join(workdir, "sample.docx");
    const output = join(workdir, "sample.md");
    await writeFile(
        input,
        storedZip([{ path: "word/document.xml", payload: documentXml }]),
    );

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

    const markdown = await readFile(output, "utf8");
    assert.match(markdown, /Hello DOCX/);
    assert.match(markdown, /# Title/);
});
