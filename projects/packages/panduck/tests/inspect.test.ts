import assert from "node:assert/strict";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { minimalDocumentXml } from "./fixtures/docx-xml.js";
import { runPanduck } from "./helpers/cli.js";
import { storedZip } from "./helpers/stored-zip.js";

test("inspect decode reports docx member payload sizes", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-inspect-"));
    const input = join(workdir, "sample.docx");
    await writeFile(
        input,
        storedZip([{ path: "word/document.xml", payload: minimalDocumentXml }]),
    );

    const result = runPanduck([
        "inspect",
        input,
        "--from",
        "docx",
        "--stage",
        "decode",
        "--json",
    ]);
    assert.equal(result.code, 0, result.stderr || result.stdout);
    const report = JSON.parse(result.stdout) as {
        detection?: { format?: string };
        pipeline?: { stages?: string[] };
        decoded_parts?: Array<{ path: string; decoded_size: number }>;
    };
    assert.equal(report.detection?.format, "docx");
    assert.deepEqual(report.pipeline?.stages, ["decode"]);
    assert.equal(report.decoded_parts?.[0]?.path, "word/document.xml");
    assert.equal(report.decoded_parts?.[0]?.decoded_size, minimalDocumentXml.length);
});

test("inspect decode filters to a single opc part", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-inspect-"));
    const input = join(workdir, "sample.docx");
    await writeFile(
        input,
        storedZip([{ path: "word/document.xml", payload: minimalDocumentXml }]),
    );

    const result = runPanduck([
        "inspect",
        input,
        "--from",
        "docx",
        "--stage",
        "decode",
        "--path",
        "word/document.xml",
        "--json",
    ]);
    assert.equal(result.code, 0, result.stderr || result.stdout);
    assert.match(result.stdout, /word\/document\.xml/);
    assert.doesNotMatch(result.stdout, /word\/numbering\.xml/);
});

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
