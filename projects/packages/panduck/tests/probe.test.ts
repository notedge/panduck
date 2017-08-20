import assert from "node:assert/strict";
import test from "node:test";

import { loadPanduckNode } from "@notedge/panduck/node";

test("panduck native bindings expose version and markdown format", () => {
    const binding = loadPanduckNode();
    const formats = binding.supportedFormats();
    assert.ok(binding.panduckVersion().length > 0);
    assert.ok(formats.includes("markdown"));
    assert.equal(binding.isSupportedFormat("markdown"), true);
});
