import assert from "node:assert/strict";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { minimalDocumentXml } from "./fixtures/docx-xml.js";
import { runPanduck } from "./helpers/cli.js";
import { storedZip } from "./helpers/stored-zip.js";

test("inspect index lists docx opc parts", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-inspect-"));
    const input = join(workdir, "sample.docx");
    await writeFile(
        input,
        storedZip([{ path: "word/document.xml", payload: minimalDocumentXml }]),
    );

    const result = runPanduck(["inspect", input, "--from", "docx", "--stage", "index", "--json"]);
    assert.equal(result.code, 0, result.stderr || result.stdout);
    assert.match(result.stdout, /"format": "docx"/);
    assert.match(result.stdout, /word\/document\.xml/);
    assert.match(result.stdout, /"outer": "zip"/);
});
