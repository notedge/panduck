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
