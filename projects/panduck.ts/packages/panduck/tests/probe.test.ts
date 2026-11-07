import assert from "node:assert/strict";
import test from "node:test";

import { loadPanduckNode } from "../src/node/index.js";

test("panduck native bindings expose version and markdown format", () => {
    const binding = loadPanduckNode();
    const formats = binding.supportedFormats();
    assert.ok(binding.panduckVersion().length > 0);
    assert.ok(formats.includes("markdown"));
    assert.equal(binding.isSupportedFormat("markdown"), true);
});

test("panduck native bindings expose wired conversion routes", () => {
    const binding = loadPanduckNode();
    assert.ok(binding.supportsConversion?.("docx", "markdown"));
    assert.ok(binding.supportsConversion?.("markdown", "markdown"));
    assert.ok(binding.supportsConversion?.("markdown", "docx"));
    assert.ok(binding.supportsConversion?.("docx", "docx"));
    const routes = binding.supportedConversions?.() ?? [];
    assert.ok(routes.includes("docx:markdown"));
    assert.ok(routes.includes("markdown:markdown"));
    assert.ok(routes.includes("markdown:docx"));
    assert.ok(routes.includes("docx:docx"));
});

test("pandoc-json is advertised and served as a partial route in both directions", () => {
    const binding = loadPanduckNode();
    assert.ok(binding.supportedFormats().includes("pandoc-json"));
    assert.equal(binding.isSupportedFormat("pandoc-json"), true);
    assert.equal(binding.supportsConversion?.("pandoc-json", "markdown"), true);
    assert.equal(binding.supportsConversion?.("markdown", "pandoc-json"), true);
    assert.equal(binding.supportsConversion?.("pandoc-json", "pandoc-json"), true);
    assert.equal(binding.supportsConversion?.("pandoc-json", "doc"), false);

    const routes = binding.supportedConversions?.() ?? [];
    assert.ok(routes.includes("pandoc-json:markdown"));
    assert.ok(routes.includes("markdown:pandoc-json"));
    assert.ok(!routes.includes("pandoc-json:doc"), "an unavailable writer must not be advertised");
});

test("a route reported by supportsConversion actually imports and exports", () => {
    const binding = loadPanduckNode();
    const source = JSON.stringify({
        "pandoc-api-version": [1, 23, 1],
        meta: {},
        blocks: [{ t: "Para", c: [{ t: "Str", c: "Hello Pandoc" }] }],
    });
    const imported = binding.importDocument?.("pandoc-json", "sample.json", Buffer.from(source));
    assert.ok(imported?.success, imported?.diagnosticsJson);
    const graphJson = imported?.graphJson;
    assert.ok(graphJson, "a wired reader must return the serialized graph");

    const exported = binding.exportDocument?.("pandoc-json", graphJson);
    assert.ok(exported?.success, exported?.diagnosticsJson);
    const document = JSON.parse(exported?.text ?? "null") as {
        "pandoc-api-version": number[];
        blocks: Array<{ t: string; c?: unknown }>;
    };
    assert.deepEqual(document["pandoc-api-version"], [1, 23, 1]);
    assert.match(JSON.stringify(document.blocks), /Hello Pandoc/);
});

test("pandoc-json keeps typed metadata across import and export without silent loss", () => {
    const binding = loadPanduckNode();
    const source = JSON.stringify({
        "pandoc-api-version": [1, 23, 1],
        meta: {
            title: { t: "MetaInlines", c: [{ t: "Str", c: "Typed metadata" }] },
            author: { t: "MetaList", c: [{ t: "MetaString", c: "Ada" }] },
            project: { t: "MetaString", c: "panduck" },
            draft: { t: "MetaBool", c: true },
            review: {
                t: "MetaMap",
                c: {
                    phase: { t: "MetaString", c: "two" },
                    stages: {
                        t: "MetaList",
                        c: [{ t: "MetaString", c: "alpha" }, { t: "MetaString", c: "beta" }],
                    },
                },
            },
        },
        blocks: [{ t: "Para", c: [{ t: "Str", c: "Body" }] }],
    });
    const lossCodes = (json: string): string[] =>
        (JSON.parse(json) as Array<{ code: string }>).map((loss) => loss.code);

    const imported = binding.importDocument?.("pandoc-json", "meta.json", Buffer.from(source));
    assert.ok(imported?.success, imported?.diagnosticsJson);
    const graphJson = imported?.graphJson;
    assert.ok(graphJson, "a wired reader must return the serialized graph");

    const graph = JSON.parse(graphJson) as {
        metadata: { properties?: Array<{ key: string; value: unknown }> };
    };
    const properties = graph.metadata.properties ?? [];
    assert.deepEqual(
        Object.fromEntries(properties.map((entry) => [entry.key, entry.value])),
        {
            project: { text: "panduck" },
            draft: { boolean: true },
            review: {
                map: [
                    { key: "phase", value: { text: "two" } },
                    { key: "stages", value: { list: [{ text: "alpha" }, { text: "beta" }] } },
                ],
            },
        },
        "unmapped metadata keys must keep their typed payload",
    );
    assert.ok(
        !lossCodes(imported?.lossesJson ?? "[]").includes("import.pandoc-json.meta_unmapped"),
        "typed properties must not be reported as unmapped",
    );

    const exported = binding.exportDocument?.("pandoc-json", graphJson);
    assert.ok(exported?.success, exported?.diagnosticsJson);
    const document = JSON.parse(exported?.text ?? "null") as {
        meta: Record<string, unknown>;
    };
    assert.deepEqual(document.meta.title, { t: "MetaInlines", c: [{ t: "Str", c: "Typed metadata" }] });
    assert.deepEqual(document.meta.project, { t: "MetaString", c: "panduck" });
    assert.deepEqual(document.meta.draft, { t: "MetaBool", c: true });
    assert.deepEqual(document.meta.review, {
        t: "MetaMap",
        c: {
            phase: { t: "MetaString", c: "two" },
            stages: { t: "MetaList", c: [{ t: "MetaString", c: "alpha" }, { t: "MetaString", c: "beta" }] },
        },
    });
    assert.ok(
        !lossCodes(exported?.lossesJson ?? "[]").includes("export.pandoc-json.metadata_number"),
        "text, bool, list and map metadata must export without a numeric loss",
    );
});

test("pandoc-json keeps quote paragraphs as child blocks across a roundtrip", () => {
    const binding = loadPanduckNode();
    const source = JSON.stringify({
        "pandoc-api-version": [1, 23, 1],
        meta: {},
        blocks: [
            {
                t: "BlockQuote",
                c: [{ t: "Para", c: [{ t: "Str", c: "quoted" }, { t: "Space" }, { t: "Str", c: "text" }] }],
            },
            { t: "Para", c: [{ t: "Str", c: "after" }] },
        ],
    });

    const imported = binding.importDocument?.("pandoc-json", "quote.json", Buffer.from(source));
    assert.ok(imported?.success, imported?.diagnosticsJson);
    const graphJson = imported?.graphJson;
    assert.ok(graphJson, "a wired reader must return the serialized graph");

    const graph = JSON.parse(graphJson) as {
        blocks: Array<{ id: number; block: Record<string, unknown> }>;
    };
    const quoteNode = graph.blocks.find((node) => "Quote" in node.block);
    assert.ok(quoteNode, "the block quote must survive import");
    const quote = quoteNode.block.Quote as { children: number[]; content?: unknown };
    assert.equal(quote.content, undefined, "the quote must no longer carry inline content");
    assert.equal(quote.children.length, 1, "the quote paragraph must become one child block");
    const paragraph = graph.blocks.find((node) => node.id === quote.children[0]);
    assert.ok(paragraph && "Paragraph" in paragraph.block, "the quote child must be a paragraph block");

    const exported = binding.exportDocument?.("pandoc-json", graphJson);
    assert.ok(exported?.success, exported?.diagnosticsJson);
    const document = JSON.parse(exported?.text ?? "null") as {
        blocks: Array<{ t: string; c: Array<{ t: string; c: Array<{ t: string; c: unknown }> }> }>;
    };
    assert.equal(document.blocks[0].t, "BlockQuote");
    assert.equal(document.blocks[0].c.length, 1, "the exported quote must hold exactly its paragraph");
    assert.equal(document.blocks[0].c[0].t, "Para");
    const inlines = JSON.stringify(document.blocks[0].c[0].c);
    assert.match(inlines, /quoted/);
    assert.match(inlines, /text/);
});

test("pandoc-json keeps table bands and cell child blocks across a roundtrip", () => {
    const binding = loadPanduckNode();
    const cell = (text: string) => [
        ["", [], []],
        "AlignDefault",
        1,
        1,
        [{ t: "Para", c: [{ t: "Str", c: text }] }],
    ];
    const row = (text: string) => [["", [], []], [cell(text)]];
    const source = JSON.stringify({
        "pandoc-api-version": [1, 23, 1],
        meta: {},
        blocks: [
            {
                t: "Table",
                c: [
                    ["", [], []],
                    [null, [{ t: "Plain", c: [{ t: "Str", c: "Captioned" }] }]],
                    [["AlignDefault", 0]],
                    [["", [], []], [row("Head")]],
                    [[["", [], []], 0, [], [row("Body")]]],
                    [["", [], []], [row("Foot")]],
                ],
            },
        ],
    });

    const imported = binding.importDocument?.("pandoc-json", "table.json", Buffer.from(source));
    assert.ok(imported?.success, imported?.diagnosticsJson);
    const graphJson = imported?.graphJson;
    assert.ok(graphJson, "a wired reader must return the serialized graph");

    const graph = JSON.parse(graphJson) as {
        blocks: Array<{
            id: number;
            block: {
                Table?: {
                    head: Array<{ cells: Array<{ children: number[] }> }>;
                    body: Array<{ cells: Array<{ children: number[] }> }>;
                    foot: Array<{ cells: Array<{ children: number[] }> }>;
                };
            };
        }>;
    };
    const tableNode = graph.blocks.find((node) => "Table" in node.block);
    assert.ok(tableNode?.block.Table, "the table must survive import");
    const table = tableNode.block.Table;
    assert.deepEqual([table.head.length, table.body.length, table.foot.length], [1, 1, 1], "all three bands must survive");
    for (const band of [table.head, table.body, table.foot]) {
        for (const cellEntry of band[0].cells) {
            assert.equal(cellEntry.children.length, 1, "each cell must own one paragraph child");
            const child = graph.blocks.find((node) => node.id === cellEntry.children[0]);
            assert.ok(child, "the cell child must exist in `blocks`");
        }
    }

    const exported = binding.exportDocument?.("pandoc-json", graphJson);
    assert.ok(exported?.success, exported?.diagnosticsJson);
    const document = JSON.parse(exported?.text ?? "null") as {
        blocks: Array<{ t: string; c: unknown[] }>;
    };
    const exportedTable = document.blocks[0].c;
    const headRows = exportedTable[3] as [unknown, unknown[]];
    const bodyRows = exportedTable[4] as Array<[unknown, number, unknown[], unknown[]]>;
    const footRows = exportedTable[5] as [unknown, unknown[]];
    assert.equal(headRows[1].length, 1, "the header band must export one row");
    assert.equal(bodyRows[0][3].length, 1, "the body band must export one row");
    assert.equal(footRows[1].length, 1, "the footer band must export one row");
    const text = JSON.stringify(exportedTable);
    for (const value of ["Head", "Body", "Foot"]) {
        assert.match(text, new RegExp(value), `the ${value} cell text must survive`);
    }
});
