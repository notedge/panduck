import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { test } from "node:test";

import { PANDUCK_DIAGNOSTIC_CODES } from "../src/diagnostic-codes.js";

const SOURCE_ROOT = join(dirname(fileURLToPath(import.meta.url)), "..", "src");
const CATALOG_FILE = join(SOURCE_ROOT, "diagnostic-codes.ts");

/** Strings that contain a dot but are file names, not diagnostic codes. */
const NON_CODE_LITERALS = new Set(["panduck.report.json"]);

function sourceFiles(directory: string): string[] {
    const files: string[] = [];
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
        const path = join(directory, entry.name);
        if (entry.isDirectory()) {
            files.push(...sourceFiles(path));
        } else if (entry.name.endsWith(".ts") && path !== CATALOG_FILE) {
            files.push(path);
        }
    }
    return files;
}

test("frozen codes are unique and stay inside the panduck namespace", () => {
    const values = Object.values(PANDUCK_DIAGNOSTIC_CODES);
    assert.equal(new Set(values).size, values.length, "duplicate code in PANDUCK_DIAGNOSTIC_CODES");
    for (const value of values) {
        assert.match(value, /^panduck\.[a-z0-9]+(?:[._-][a-z0-9]+)*$/, `code ${value} is outside the panduck namespace`);
    }
});

test("emitting sites reference the catalog instead of writing literals", () => {
    for (const file of sourceFiles(SOURCE_ROOT)) {
        const source = readFileSync(file, "utf8");
        for (const match of source.matchAll(/"panduck\.[a-z0-9._-]+"/g)) {
            const literal = match[0].slice(1, -1);
            assert.ok(
                NON_CODE_LITERALS.has(literal),
                `${file} write diagnostic code ${literal} as a literal, use PANDUCK_DIAGNOSTIC_CODES`,
            );
        }
    }
});
