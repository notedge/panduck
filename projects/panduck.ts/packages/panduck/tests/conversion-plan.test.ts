import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtemp, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { Converter } from "../src/converter.js";
import { defineConversion, serializeConversionPlan } from "../src/conversion-plan.js";
import type { ConversionPlan } from "../src/conversion-plan.js";
import { publishMarkdownProject, PublishConflictError } from "../src/publish-project.js";
import type { PublishableProject } from "../src/publish-project.js";
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

test("markdown-project materializes embedded PNG and JPEG assets", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-publish-mixed-images-"));
    const outputDir = join(workdir, "site");
    const mixedDocumentXml = Buffer.from(`<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
            xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
            xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing">
  <w:body>
    <w:p>
      <w:r>
        <w:drawing><wp:inline><wp:docPr descr="Png"/><a:graphic><a:graphicData><a:blip r:embed="rId1"/></a:graphicData></a:graphic></wp:inline></w:drawing>
      </w:r>
    </w:p>
    <w:p>
      <w:r>
        <w:drawing><wp:inline><wp:docPr descr="Jpeg"/><a:graphic><a:graphicData><a:blip r:embed="rId2"/></a:graphicData></a:graphic></wp:inline></w:drawing>
      </w:r>
    </w:p>
  </w:body>
</w:document>`, "utf8");
    const mixedRelsXml = Buffer.from(`<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/photo.png"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/photo.jpeg"/>
</Relationships>`, "utf8");
    const pngBytes = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x01, 0x02]);
    const jpegBytes = Buffer.from([0xff, 0xd8, 0xff, 0xe0, 0x03, 0x04]);
    const makeDocx = () =>
        storedZip([
            { path: "word/document.xml", payload: mixedDocumentXml },
            { path: "word/_rels/document.xml.rels", payload: mixedRelsXml },
            { path: "word/media/photo.png", payload: pngBytes },
            { path: "word/media/photo.jpeg", payload: jpegBytes },
        ]);
    try {
        const converter = new Converter();
        const plan = defineConversion({
            id: "publish-mixed-images-check",
            input: { format: "docx", label: "mixed.docx" },
            output: { format: "markdown" },
            publish: { kind: "markdown-project", directory: outputDir },
        });

        const result = await converter.toMarkdown(makeDocx(), plan);
        assert.notEqual(result.status, "failed");

        const index = await readFile(join(outputDir, "index.md"), "utf8");
        const assetPaths = [...index.matchAll(/\]\(assets\/([0-9a-f]{16}\.\w+)\)/g)].map((match) => match[1]);
        assert.equal(assetPaths.length, 2, "both embedded images must be rewritten to assets/");
        assert.ok(assetPaths.some((path) => path.endsWith(".png")), "the PNG keeps its extension");
        assert.ok(assetPaths.some((path) => path.endsWith(".jpeg")), "the JPEG keeps its extension");

        for (const path of assetPaths) {
            const expected = path.endsWith(".png") ? pngBytes : jpegBytes;
            assert.deepEqual(await readFile(join(outputDir, "assets", path)), expected);
        }

        const report = JSON.parse(result.reportJson);
        const assetOutput = report.outputs.find((output: { path?: string }) => output.path === "assets/");
        assert.equal(assetOutput?.published_asset_count, 2);
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

test("markdown-project republish reproduces the same directory tree for identical input", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-publish-deterministic-"));
    const outputDir = join(workdir, "site");
    const makeDocx = () =>
        storedZip([
            { path: "word/document.xml", payload: imageDocumentXml },
            { path: "word/_rels/document.xml.rels", payload: imageRelsXml },
            { path: "word/media/logo.png", payload: Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a]) },
        ]);
    try {
        const plan = defineConversion({
            id: "publish-deterministic-check",
            input: { format: "docx", label: "image.docx" },
            output: { format: "markdown" },
            publish: { kind: "markdown-project", directory: outputDir },
        });
        const converter = new Converter();

        const first = await converter.toMarkdown(makeDocx(), plan);
        assert.notEqual(first.status, "failed");
        const firstTree = await collectTree(outputDir);

        const second = await converter.toMarkdown(makeDocx(), plan);
        assert.notEqual(second.status, "failed");
        assert.deepEqual(
            await collectTree(outputDir),
            firstTree,
            "identical input must reproduce the same files, asset names and report",
        );
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

test("markdown-project publish rejects project paths that escape the output directory", async () => {
    const cases: Array<{ name: string; project: PublishableProject; error?: RegExp }> = [
        {
            name: "chapter-traversal",
            project: {
                indexMarkdown: "# Index",
                chapters: [{ path: "../escape.md", markdown: "# Escaped" }],
                assets: [],
            },
        },
        {
            name: "chapter-absolute",
            project: {
                indexMarkdown: "# Index",
                chapters: [{ path: "/absolute/escape.md", markdown: "# Escaped" }],
                assets: [],
            },
        },
        {
            name: "chapter-drive-letter",
            project: {
                indexMarkdown: "# Index",
                chapters: [{ path: "C:\\escape.md", markdown: "# Escaped" }],
                assets: [],
            },
        },
        {
            name: "chapter-unc",
            project: {
                indexMarkdown: "# Index",
                chapters: [{ path: "\\\\server\\share\\escape.md", markdown: "# Escaped" }],
                assets: [],
            },
        },
        {
            name: "chapter-alternate-stream",
            project: {
                indexMarkdown: "# Index",
                chapters: [{ path: "notes.md:hidden", markdown: "# Escaped" }],
                assets: [],
            },
        },
        {
            name: "chapter-control-character",
            project: {
                indexMarkdown: "# Index",
                chapters: [{ path: "bad\u0001.md", markdown: "# Escaped" }],
                assets: [],
            },
        },
        {
            name: "chapter-trailing-dot",
            project: {
                indexMarkdown: "# Index",
                chapters: [{ path: "dir./escape.md", markdown: "# Escaped" }],
                assets: [],
            },
        },
        {
            name: "asset-traversal",
            project: {
                indexMarkdown: "# Index",
                chapters: [],
                assets: [{ path: "assets/../../escape.bin", bytes: new Uint8Array([1]) }],
            },
        },
        {
            name: "chapter-reserved-index",
            project: {
                indexMarkdown: "# Index",
                chapters: [{ path: "index.md", markdown: "# Escaped" }],
                assets: [],
            },
            error: /project path collides with another project path/,
        },
        {
            name: "chapter-duplicate",
            project: {
                indexMarkdown: "# Index",
                chapters: [
                    { path: "same.md", markdown: "# One" },
                    { path: "same.md", markdown: "# Two" },
                ],
                assets: [],
            },
            error: /project path is declared twice/,
        },
        {
            name: "chapter-file-over-directory",
            project: {
                indexMarkdown: "# Index",
                chapters: [
                    { path: "x/y.md", markdown: "# Nested" },
                    { path: "x", markdown: "# File" },
                ],
                assets: [],
            },
            error: /project path is declared twice/,
        },
        {
            name: "chapter-directory-over-file",
            project: {
                indexMarkdown: "# Index",
                chapters: [
                    { path: "x", markdown: "# File" },
                    { path: "x/y.md", markdown: "# Nested" },
                ],
                assets: [],
            },
            error: /project path collides with another project path/,
        },
        {
            name: "asset-and-chapter-share-a-path",
            project: {
                indexMarkdown: "# Index",
                chapters: [{ path: "assets/x.png", markdown: "# Escaped" }],
                assets: [{ path: "assets/x.png", bytes: new Uint8Array([1]) }],
            },
            error: /project path is declared twice/,
        },
        {
            name: "asset-outside-the-assets-directory",
            project: {
                indexMarkdown: "# Index",
                chapters: [],
                assets: [{ path: "media/x.png", bytes: new Uint8Array([1]) }],
            },
            error: /asset path must start with assets\//,
        },
    ];

    for (const item of cases) {
        const workdir = await mkdtemp(join(tmpdir(), `panduck-publish-${item.name}-`));
        const outputDir = join(workdir, "site");
        try {
            await assert.rejects(
                publishMarkdownProject(outputDir, item.project, "{}"),
                item.error ?? /project path escapes the output directory/,
                item.name,
            );
            await assert.rejects(readdir(outputDir), { code: "ENOENT" }, `${item.name} must not publish a directory`);
            assert.deepEqual(
                await readdir(workdir),
                [],
                `${item.name} must leave no staging directory or escaped file`,
            );
        } finally {
            await rm(workdir, { recursive: true, force: true });
        }
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

/** Relative path and byte digest for every file under a directory. */
async function collectTree(directory: string): Promise<Array<[string, string]>> {
    const entries = await readdir(directory, { withFileTypes: true });
    const tree: Array<[string, string]> = [];
    for (const entry of entries) {
        const full = join(directory, entry.name);
        if (entry.isDirectory()) {
            for (const [relative, digest] of await collectTree(full)) {
                tree.push([join(entry.name, relative), digest]);
            }
        } else {
            tree.push([entry.name, (await readFile(full)).toString("base64")]);
        }
    }
    return tree.sort(([left], [right]) => left.localeCompare(right));
}

const stableProject: PublishableProject = {
    indexMarkdown: "# First\n",
    chapters: [{ path: "chapters/one.md", markdown: "# One\n" }],
    assets: [{ path: "assets/a.bin", bytes: new Uint8Array([1, 2, 3]) }],
};

test("failed publish restores the previous project and leaves no staging or backup", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-publish-rollback-"));
    const outputDir = join(workdir, "site");
    try {
        await publishMarkdownProject(outputDir, stableProject, "{\"version\":1}");
        const before = await collectTree(outputDir);
        assert.deepEqual(await readdir(workdir), ["site"], "a successful publish must leave only the output directory");

        await assert.rejects(
            publishMarkdownProject(
                outputDir,
                {
                    indexMarkdown: "# Second\n",
                    chapters: [{ path: "chapters/two.md", markdown: "# Two\n" }],
                    assets: [{ path: "assets/b.bin", bytes: null as unknown as Uint8Array }],
                },
                "{\"version\":2}",
            ),
        );
        assert.deepEqual(
            await collectTree(outputDir),
            before,
            "a failure during materialization must leave the previous project byte-identical",
        );
        assert.deepEqual(await readdir(workdir), ["site"], "a failed publish must leave no staging or backup");

        const index = await readFile(join(outputDir, "index.md"), "utf8");
        assert.equal(index, "# First\n", "the previous project must stay readable after rollback");

        await assert.rejects(
            publishMarkdownProject(
                outputDir,
                {
                    indexMarkdown: "# Third\n",
                    chapters: [{ path: "index.md", markdown: "# Clobber" }],
                    assets: [],
                },
                "{}",
            ),
            /project path collides with another project path/,
        );
        assert.deepEqual(await collectTree(outputDir), before, "a rejected layout must not touch the published project");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("concurrent publishes to one directory serialize and the last project wins", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-publish-concurrent-"));
    const outputDir = join(workdir, "site");
    const second: PublishableProject = {
        indexMarkdown: "# Second\n",
        chapters: [{ path: "chapters/two.md", markdown: "# Two\n" }],
        assets: [{ path: "assets/b.bin", bytes: new Uint8Array([4, 5]) }],
    };
    try {
        const [first, latest] = await Promise.all([
            publishMarkdownProject(outputDir, stableProject, "{\"version\":1}"),
            publishMarkdownProject(outputDir, second, "{\"version\":2}"),
        ]);
        assert.equal(first.outputDir, outputDir);
        assert.equal(latest.outputDir, outputDir);

        assert.equal(await readFile(join(outputDir, "index.md"), "utf8"), "# Second\n");
        assert.equal(await readFile(join(outputDir, "chapters", "two.md"), "utf8"), "# Two\n");
        await assert.rejects(readFile(join(outputDir, "chapters", "one.md"), "utf8"), { code: "ENOENT" });
        assert.equal(await readFile(join(outputDir, "panduck.report.json"), "utf8"), "{\"version\":2}");
        assert.deepEqual(await readdir(workdir), ["site"], "serialized publishes must leave no staging or backup");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("a live publish lock yields a conflict diagnostic and keeps the project readable", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-publish-lock-"));
    const outputDir = join(workdir, "site");
    const lockPath = join(workdir, ".site.lock");
    try {
        await publishMarkdownProject(outputDir, stableProject, "{\"version\":1}");
        const before = await collectTree(outputDir);
        await writeFile(lockPath, JSON.stringify({ pid: process.pid, started_at: Date.now() }), "utf8");

        await assert.rejects(
            publishMarkdownProject(
                outputDir,
                { indexMarkdown: "# Second\n", chapters: [], assets: [] },
                "{\"version\":2}",
            ),
            (error: unknown) =>
                error instanceof PublishConflictError &&
                /locked by another publisher/.test(error.message),
        );
        assert.deepEqual(await collectTree(outputDir), before, "the lock holder's project must stay untouched");
        assert.deepEqual(
            await readdir(workdir).then((entries) => entries.sort()),
            [".site.lock", "site"],
            "a conflict must not create staging or backup directories",
        );

        const docx = storedZip([
            { path: "word/document.xml", payload: imageDocumentXml },
            { path: "word/_rels/document.xml.rels", payload: imageRelsXml },
            { path: "word/media/logo.png", payload: Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a]) },
        ]);
        const converter = new Converter();
        const result = await converter.toMarkdown(
            docx,
            defineConversion({
                id: "publish-lock-check",
                input: { format: "docx", label: "image.docx" },
                output: { format: "markdown" },
                publish: { kind: "markdown-project", directory: outputDir },
            }),
        );
        assert.equal(result.status, "failed");
        const conflict = result.diagnostics.find((diagnostic) => diagnostic.code === "panduck.publish.conflict");
        assert.ok(conflict, "the CLI surface must report the publish conflict code");
        assert.equal(conflict.severity, "error");
        assert.deepEqual(await collectTree(outputDir), before, "the diagnostic path must not touch the published project");

        await rm(lockPath, { force: true });
        const republished = await publishMarkdownProject(outputDir, stableProject, "{\"version\":1}");
        assert.equal(republished.outputDir, outputDir);
        assert.deepEqual(await readdir(workdir), ["site"], "the lock must be released after the publish");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

test("a lock left by a dead publisher is reclaimed", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-publish-stale-lock-"));
    const outputDir = join(workdir, "site");
    const lockPath = join(workdir, ".site.lock");
    try {
        const dead = spawnSync(process.execPath, ["-e", ""], { stdio: "ignore" });
        assert.equal(typeof dead.pid, "number", "the probe process must report its pid");
        await writeFile(lockPath, JSON.stringify({ pid: dead.pid, started_at: Date.now() }), "utf8");

        const published = await publishMarkdownProject(outputDir, stableProject, "{\"version\":1}");
        assert.equal(published.outputDir, outputDir);
        assert.equal(await readFile(join(outputDir, "index.md"), "utf8"), "# First\n");
        assert.deepEqual(await readdir(workdir), ["site"], "the stale lock must be replaced and released");
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
});

