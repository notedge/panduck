import assert from "node:assert/strict";
import { mkdtemp, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { Converter } from "../src/converter.js";
import { defineConversion, serializeConversionPlan } from "../src/conversion-plan.js";
import type { ConversionPlan } from "../src/conversion-plan.js";
import { imageDocumentXml, imageRelsXml } from "./fixtures/docx-xml.js";
import { storedZip } from "./helpers/stored-zip.js";

function sampleHtml(body: string): string {
    return `<!doctype html><html><head></head><body>${body}</body></html>`;
}

const duplicateImageDocumentXml = Buffer.from(`<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
            xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
            xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing">
  <w:body>
    <w:p>
      <w:r>
        <w:drawing><wp:inline><wp:docPr descr="First"/><a:graphic><a:graphicData><a:blip r:embed="rId1"/></a:graphicData></a:graphic></wp:inline></w:drawing>
      </w:r>
    </w:p>
    <w:p>
      <w:r>
        <w:drawing><wp:inline><wp:docPr descr="Second"/><a:graphic><a:graphicData><a:blip r:embed="rId2"/></a:graphicData></a:graphic></wp:inline></w:drawing>
      </w:r>
    </w:p>
  </w:body>
</w:document>`, "utf8");

const duplicateImageRelsXml = Buffer.from(`<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/alpha.png"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/beta.png"/>
</Relationships>`, "utf8");

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
        transforms: [{ name: "no-such-pass" }],
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

test("markdown-project publish deduplicates identical asset bytes and rewrites references", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-publish-dedupe-"));
    const outputDir = join(workdir, "site");
    const samePng = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x00, 0x73, 0x61, 0x6d, 0x65]);
    try {
        const docx = storedZip([
            { path: "word/document.xml", payload: duplicateImageDocumentXml },
            { path: "word/_rels/document.xml.rels", payload: duplicateImageRelsXml },
            { path: "word/media/alpha.png", payload: samePng },
            { path: "word/media/beta.png", payload: samePng },
        ]);
        const converter = new Converter();
        const plan = defineConversion({
            id: "publish-dedupe-check",
            input: { format: "docx", label: "duplicate.docx" },
            output: { format: "markdown" },
            publish: { kind: "markdown-project", directory: outputDir },
        });

        const result = await converter.toMarkdown(docx, plan);

        assert.notEqual(result.status, "failed");
        assert.equal(result.publishPath, outputDir);

        const index = await readFile(join(outputDir, "index.md"), "utf8");
        const refs = [...index.matchAll(/!\[(First|Second)\]\(([^)]+)\)/g)];
        assert.equal(refs.length, 2, `both image references must survive publish: ${index}`);
        const firstPath = refs[0][2];
        const secondPath = refs[1][2];
        assert.match(firstPath, /^assets\/[0-9a-f]{16}\.png$/);
        assert.equal(secondPath, firstPath, "identical bytes must share one content-addressed path");
        assert.doesNotMatch(index, /media\/(alpha|beta)\.png/, "source asset names must be rewritten");

        const assets = await readdir(join(outputDir, "assets"));
        assert.deepEqual(assets, [firstPath.slice("assets/".length)], "dedupe must materialize exactly one asset file");
        const publishedBytes = await readFile(join(outputDir, firstPath));
        assert.deepEqual(publishedBytes, samePng);

        const report = JSON.parse(result.reportJson);
        const assetOutput = report.outputs.find(
            (output: { path?: string }) => output.path === "assets/",
        );
        assert.equal(assetOutput?.published_asset_count, 1);
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("markdown-project republish owns the output directory and removes stale files", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-publish-republish-"));
    const outputDir = join(workdir, "nested", "site");
    const makeDocx = () =>
        storedZip([
            { path: "word/document.xml", payload: imageDocumentXml },
            { path: "word/_rels/document.xml.rels", payload: imageRelsXml },
            { path: "word/media/logo.png", payload: Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a]) },
        ]);
    try {
        const converter = new Converter();
        const plan = defineConversion({
            id: "publish-republish-check",
            input: { format: "docx", label: "image.docx" },
            output: { format: "markdown" },
            publish: { kind: "markdown-project", directory: outputDir },
        });

        const first = await converter.toMarkdown(makeDocx(), plan);
        assert.notEqual(first.status, "failed");
        assert.equal(first.publishPath, outputDir, "publish must create missing parent directories");

        await writeFile(join(outputDir, "stale.md"), "stale", "utf8");

        const second = await converter.toMarkdown(makeDocx(), plan);
        assert.notEqual(second.status, "failed");
        assert.equal(second.publishPath, outputDir);
        await assert.rejects(readFile(join(outputDir, "stale.md"), "utf8"));

        const index = await readFile(join(outputDir, "index.md"), "utf8");
        assert.match(index, /!\[Logo\]\(assets\/[0-9a-f]{16}\.png\)/);
        const assets = await readdir(join(outputDir, "assets"));
        assert.ok(assets.length >= 1, "republish must still materialize the docx image");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("publish kind none writes nothing and returns no publishPath", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-publish-none-"));
    try {
        const converter = new Converter();
        const plan = defineConversion({
            id: "publish-none-check",
            input: { format: "html" },
            output: { format: "markdown" },
            publish: { kind: "none" },
        });

        const result = await converter.toMarkdown(sampleHtml("<h1>No publish</h1><p>Body.</p>"), plan);

        assert.notEqual(result.status, "failed");
        assert.equal(result.publishPath, undefined);
        assert.deepEqual(await readdir(workdir), [], "publish none must not write files");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("unsupported publish kind is rejected instead of silently treated as none", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-publish-unknown-"));
    const outputDir = join(workdir, "site");
    try {
        const converter = new Converter();
        const base = defineConversion({
            id: "publish-unknown-check",
            input: { format: "html" },
            output: { format: "markdown" },
        });
        const plan = {
            ...base,
            publish: { kind: "single-file", directory: outputDir },
        } as unknown as ConversionPlan;

        const result = await converter.toMarkdown(sampleHtml("<p>Body.</p>"), plan);

        assert.equal(result.status, "failed");
        assert.equal(result.text, "");
        assert.equal(result.publishPath, undefined);
        const diagnostic = result.diagnostics.find((d) => d.code === "panduck.convert.unsupported-publish-kind");
        assert.ok(diagnostic, "unsupported publish kinds must produce an error diagnostic");
        assert.equal(diagnostic.severity, "error");
        assert.deepEqual(
            diagnostic.message.args.find(([key]) => key === "publishKind")?.[1],
            { text: "single-file" },
        );
        await assert.rejects(readdir(outputDir));
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("in-process transform handlers are diagnosed and never executed", async () => {
    const converter = new Converter();
    let handlerCalls = 0;
    const plan = defineConversion({
        id: "handler-check",
        input: { format: "html" },
        transforms: [
            {
                name: "replace-text",
                params: { search: "Hello", replacement: "Bonjour" },
                pass: () => {
                    handlerCalls += 1;
                    throw new Error("in-process handler must not execute");
                },
            },
        ],
        output: { format: "markdown" },
    });

    const result = await converter.toMarkdown(sampleHtml("<h1>Hello World</h1>"), plan);

    assert.equal(handlerCalls, 0, "the declared handler must not be invoked");
    assert.notEqual(result.status, "failed");
    assert.match(result.text, /Bonjour World/, "the named built-in pass still runs");
    const diagnostic = result.diagnostics.find((d) => d.code === "panduck.transform.non-executable-handler");
    assert.ok(diagnostic, "unexecutable handlers must not be silently ignored");
    assert.equal(diagnostic.severity, "warning");
    assert.deepEqual(
        diagnostic.message.args.find(([key]) => key === "passName")?.[1],
        { text: "replace-text" },
    );
    const descriptor = JSON.parse(serializeConversionPlan(plan));
    assert.equal(descriptor.transforms[0].pass, undefined, "handlers must stay off the wire");
});

test("empty extracted scope payload fails with a diagnostic instead of empty Markdown", async () => {
    const converter = new Converter();
    const plan = defineConversion({
        id: "empty-scope-check",
        input: { format: "html", sourceUrl: "https://example.com/empty" },
        output: { format: "markdown" },
        publish: { kind: "none" },
    });

    const result = await converter.toMarkdown(
        {
            url: "https://example.com/empty",
            bytes: () => new Uint8Array(),
            html: async () => "",
        },
        plan,
    );

    assert.equal(result.status, "failed");
    assert.equal(result.text, "", "no Markdown may be emitted for an empty scope");
    assert.equal(result.publishPath, undefined);

    const diagnostic = result.diagnostics.find((d) => d.code === "panduck.convert.empty-input");
    assert.ok(diagnostic, "empty scope must produce a structured diagnostic");
    assert.equal(diagnostic.severity, "error");
    assert.deepEqual(
        diagnostic.message.args.find(([key]) => key === "url")?.[1],
        { text: "https://example.com/empty" },
    );

    const report = JSON.parse(result.reportJson);
    assert.equal(report.status, "blocked");
    assert.deepEqual(report.outputs, []);
});
