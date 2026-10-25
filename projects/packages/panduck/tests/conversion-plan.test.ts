import assert from "node:assert/strict";
import { mkdtemp, readFile, readdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { Converter } from "../src/converter.js";
import { defineConversion, serializeConversionPlan } from "../src/conversion-plan.js";
import { imageDocumentXml, imageRelsXml } from "./fixtures/docx-xml.js";
import { storedZip } from "./helpers/stored-zip.js";

function sampleHtml(body: string): string {
    return `<!doctype html><html><head></head><body>${body}</body></html>`;
}

test("serializeConversionPlan carries transform params and target kinds", () => {
    const plan = defineConversion({
        id: "serialize-check",
        input: { format: "html" },
        transforms: [
            {
                name: "replace-text",
                description: "rewrite greeting",
                params: { search: "Hello", replacement: "Bonjour" },
            },
            { name: "remove-blocks", targetKinds: ["quote"] },
        ],
        output: { format: "markdown" },
        publish: { kind: "none" },
    });

    const parsed = JSON.parse(serializeConversionPlan(plan));
    assert.deepEqual(parsed.transforms[0], {
        name: "replace-text",
        description: "rewrite greeting",
        params: { search: "Hello", replacement: "Bonjour" },
    });
    assert.deepEqual(parsed.transforms[1].targetKinds, ["quote"]);
    assert.equal(parsed.transforms[1].params, undefined);
});

test("serializeConversionPlan keeps only executable publish strategy fields", () => {
    const plan = defineConversion({
        id: "publish-shape-check",
        input: { format: "docx" },
        output: { format: "markdown" },
        publish: { kind: "markdown-project", directory: "./out" },
    });

    const parsed = JSON.parse(serializeConversionPlan(plan));
    assert.deepEqual(parsed.publish, { kind: "markdown-project", directory: "./out" });
});

test("Converter executes replace-text across the IR graph", async () => {
    const converter = new Converter();
    const plan = defineConversion({
        id: "replace-check",
        input: { format: "html" },
        transforms: [
            {
                name: "replace-text",
                description: "rewrite greeting",
                params: { search: "Hello", replacement: "Bonjour" },
            },
        ],
        output: { format: "markdown" },
    });

    const result = await converter.toMarkdown(
        sampleHtml("<h1>Hello World</h1><p>Hello there.</p>"),
        plan,
    );

    assert.notEqual(result.status, "failed");
    assert.match(result.text, /Bonjour World/);
    assert.match(result.text, /Bonjour there\./);
    assert.doesNotMatch(result.text, /Hello/);

    const report = JSON.parse(result.reportJson);
    assert.equal(report.pipeline.transforms[0].name, "replace-text");
    assert.equal(report.pipeline.transforms[0].status, "applied");
    assert.equal(report.pipeline.transforms[0].changedNodes, 2);
    assert.equal(report.pipeline.transforms[0].replacedOccurrences, 2);
    assert.ok(result.diagnostics.some((d) => d.code === "panduck.transform.pass-applied"));
});

test("Converter executes remove-blocks and keeps unrelated content", async () => {
    const converter = new Converter();
    const plan = defineConversion({
        id: "remove-check",
        input: { format: "html" },
        transforms: [{ name: "remove-blocks", targetKinds: ["quote"] }],
        output: { format: "markdown" },
    });

    const result = await converter.toMarkdown(
        sampleHtml("<blockquote>Quoted text.</blockquote><p>Keep me.</p>"),
        plan,
    );

    assert.notEqual(result.status, "failed");
    assert.doesNotMatch(result.text, /Quoted text\./);
    assert.match(result.text, /Keep me\./);

    const report = JSON.parse(result.reportJson);
    assert.equal(report.pipeline.transforms[0].status, "applied");
    assert.equal(report.pipeline.transforms[0].changedNodes, 1);
});

test("unknown transform name is skipped with a diagnostic", async () => {
    const converter = new Converter();
    const plan = defineConversion({
        id: "unsupported-check",
        input: { format: "html" },
        transforms: [{ name: "resolve-references" }],
        output: { format: "markdown" },
    });

    const result = await converter.toMarkdown(sampleHtml("<p>Unchanged body.</p>"), plan);

    assert.notEqual(result.status, "failed");
    assert.match(result.text, /Unchanged body\./);
    const report = JSON.parse(result.reportJson);
    assert.equal(report.pipeline.transforms[0].status, "unsupported");
    assert.ok(result.diagnostics.some((d) => d.code === "panduck.transform.unsupported-pass"));
});

test("ConversionPlan without transforms keeps the previous pipeline shape", async () => {
    const converter = new Converter();
    const plan = defineConversion({
        id: "no-transform-check",
        input: { format: "html" },
        output: { format: "markdown" },
    });

    const result = await converter.toMarkdown(sampleHtml("<h1>Title</h1><p>Body.</p>"), plan);

    assert.notEqual(result.status, "failed");
    const report = JSON.parse(result.reportJson);
    assert.equal(report.pipeline.transforms, undefined);
});

test("markdown-project publish materializes assets after transforms", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-publish-"));
    const outputDir = join(workdir, "site");
    try {
        const docx = storedZip([
            { path: "word/document.xml", payload: imageDocumentXml },
            { path: "word/_rels/document.xml.rels", payload: imageRelsXml },
            { path: "word/media/logo.png", payload: Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a]) },
        ]);
        const converter = new Converter();
        const plan = defineConversion({
            id: "publish-check",
            input: { format: "docx", label: "image.docx" },
            transforms: [
                {
                    name: "replace-text",
                    params: { search: "Logo", replacement: "Brand" },
                },
            ],
            output: { format: "markdown" },
            publish: { kind: "markdown-project", directory: outputDir },
        });

        const result = await converter.toMarkdown(docx, plan);

        assert.notEqual(result.status, "failed");
        assert.equal(result.publishPath, outputDir);

        const index = await readFile(join(outputDir, "index.md"), "utf8");
        assert.match(index, /!\[Brand\]\(assets\/[0-9a-f]{16}\.png\)/);

        const assets = await readdir(join(outputDir, "assets"));
        assert.ok(assets.length >= 1, "publish must materialize the docx image");

        const report = JSON.parse(result.reportJson);
        assert.equal(report.pipeline.writer, "markdown-project");
        assert.equal(report.pipeline.transforms[0].name, "replace-text");
        assert.equal(report.pipeline.transforms[0].status, "applied");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});
