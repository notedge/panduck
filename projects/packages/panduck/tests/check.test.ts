import assert from "node:assert/strict";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { footnoteDocumentXml, minimalDocumentXml } from "./fixtures/docx-xml.js";
import { runPanduck } from "./helpers/cli.js";
import { storedZip } from "./helpers/stored-zip.js";

test("check wired docx route reports conversion losses", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-check-"));
    const input = join(workdir, "footnote.docx");
    await writeFile(input, storedZip([{ path: "word/document.xml", payload: footnoteDocumentXml }]));

    const result = runPanduck(["check", input, "--format", "markdown", "--json", "--diagnostics", "silent"]);
    assert.equal(result.code, 0, result.stderr || result.stdout);
    assert.match(result.stdout, /"operation": "check"/);
    assert.match(result.stdout, /reader\.docx\.footnote_body/);
    assert.match(result.stdout, /success_with_loss/);
});

test("check wired docx route infers markdown target", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-check-"));
    const input = join(workdir, "sample.docx");
    await writeFile(input, storedZip([{ path: "word/document.xml", payload: minimalDocumentXml }]));

    const result = runPanduck(["check", input, "--json", "--diagnostics", "silent"]);
    assert.equal(result.code, 0, result.stderr || result.stdout);
    assert.match(result.stdout, /"writer": "markdown"/);
});

test("check partial report for unwired route", () => {
    const result = runPanduck(["check", "package.json", "--from", "markdown", "--format", "docx", "--json"]);
    assert.equal(result.code, 0);
    assert.match(result.stdout, /panduck\.check\.partial/);
});
