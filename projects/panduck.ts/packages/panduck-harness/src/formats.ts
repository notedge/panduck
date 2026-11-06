import { loadFormatManifest, readFixtureBytes, type FormatCase } from "@notedge/panduck-fixtures";

import {
    exportFormatSample,
    importFormatSample,
    probeNavigationSample,
    type NavigationEntry,
} from "./adapter.js";
import { aggregateCategory } from "./assertions.js";
import type { AssertionResult, CaseObservation, CaseResult } from "./types.js";

type FormatRun = { readonly observation: CaseObservation; readonly result: CaseResult };

type CaseOutcome = {
    readonly success: boolean;
    readonly error?: string;
    readonly graphJson: string | null;
    readonly lossesJson: string;
    readonly kind: string;
    readonly sheets: readonly string[];
    readonly slides: number;
    readonly navigation: readonly NavigationEntry[];
};

type InlineFacts = {
    text: string;
    styles: string[];
    noteLabels: string[];
};

type Facts = {
    blockCount: number;
    sections: Array<{ level: number; title: string }>;
    paragraphs: string[];
    orderedNumbers: number[][];
    styles: string[];
    noteLabels: string[];
    text: string;
};

/** Runs every `document.formats` sample through the one public route it needs. */
export function formatRuns(): readonly FormatRun[] {
    return loadFormatManifest().cases.map(describeFormatCase);
}

function describeFormatCase(definition: FormatCase): FormatRun {
    const started = performance.now();
    const expect = definition.expect;
    const label = definition.path.split("/").pop() ?? definition.path;
    const isNavigation = String(expect.kind ?? "") === "nav-toc";
    const outcome = isNavigation
        ? runNavigation(label, readFixtureBytes(`formats/${definition.path}`))
        : runImport(definition, label);
    const facts = outcome.graphJson === null ? null : readFacts(outcome.graphJson);
    const exported = readExport(definition, outcome);
    const assertions: AssertionResult[] = [];
    const check = (name: string, ok: boolean, detail: string): void => {
        assertions.push(
            ok
                ? { assertion: { kind: "corpus-integrity", check: name }, status: "pass", detail }
                : {
                      assertion: { kind: "corpus-integrity", check: name },
                      status: "fail",
                      detail,
                      category: outcome.error?.startsWith("missing-entry:") ? "undetermined" : "implementation-defect",
                  },
        );
    };

    check("format-entry", outcome.success, entryDetail(outcome));

    if (isNavigation) {
        const entries = outcome.navigation;
        const first = entries[0];
        if (Number.isInteger(expect.entries)) {
            const expected = Number(expect.entries);
            check("nav-entry-count", entries.length === expected, `entries ${entries.length}, expected ${expected}`);
        }
        if (typeof expect.firstLabel === "string") {
            check(
                "nav-first-label",
                first?.label === expect.firstLabel,
                `first label ${JSON.stringify(first?.label ?? null)}, expected ${JSON.stringify(expect.firstLabel)}`,
            );
        }
        if (typeof expect.firstHref === "string") {
            check(
                "nav-first-href",
                first?.href === expect.firstHref,
                `first href ${JSON.stringify(first?.href ?? null)}, expected ${JSON.stringify(expect.firstHref)}`,
            );
        }
        if (typeof expect.firstChildLabel === "string") {
            check(
                "nav-first-child-label",
                first?.children[0]?.label === expect.firstChildLabel,
                `first child label ${JSON.stringify(first?.children[0]?.label ?? null)}, expected ${JSON.stringify(expect.firstChildLabel)}`,
            );
        }
    } else {
        if (expect.kind !== undefined && expect.kind !== "nav-toc") {
            check(
                "format-kind",
                outcome.kind === expect.kind,
                `kind ${JSON.stringify(outcome.kind)}, expected ${JSON.stringify(expect.kind)}`,
            );
        }
        if (Number.isInteger(expect.slides)) {
            const expected = Number(expect.slides);
            check("format-slides", outcome.slides === expected, `slides ${outcome.slides}, expected ${expected}`);
        }
        if (Array.isArray(expect.sheets)) {
            const expected = (expect.sheets as unknown[]).map(String);
            const same = expected.length === outcome.sheets.length && expected.every((sheet, index) => sheet === outcome.sheets[index]);
            check("format-sheets", same, `sheets ${JSON.stringify(outcome.sheets)}, expected ${JSON.stringify(expected)}`);
        }
    }

    if (Array.isArray(expect.contains) && !isNavigation) {
        const values = (expect.contains as unknown[]).map(String);
        const target = exported?.target ?? "graph";
        const body = exported?.body ?? facts?.text ?? "";
        for (const value of values) {
            check(
                `contains-${value}`,
                body.includes(value),
                `${target} ${body.includes(value) ? "contains" : "does not contain"} ${JSON.stringify(value)}`,
            );
        }
    }

    if (Array.isArray(expect.lossCodes)) {
        const observed = readLossCodes(outcome.lossesJson);
        const expected = (expect.lossCodes as unknown[]).map(String);
        const ok = expected.length === 0
            ? observed.length === 0
            : expected.every((code) => observed.includes(code));
        check(
            "loss-codes",
            ok,
            expected.length === 0
                ? `loss codes ${JSON.stringify(observed)}, expected none`
                : `loss codes ${JSON.stringify(observed)}, expected to include ${JSON.stringify(expected)}`,
        );
    }

    if (facts !== null) {
        applyGraphFacts(definition, facts, check);
    }

    const details = {
        format: definition.format,
        container: definition.container,
        classification: definition.classification,
        route: isNavigation ? "navigation" : definition.format === "pptx" || definition.format === "xlsx" ? "office-model" : "ir",
        blockCount: facts?.blockCount ?? 0,
        kind: outcome.kind,
        sheets: outcome.sheets,
        slides: outcome.slides,
        lossCodes: readLossCodes(outcome.lossesJson),
        error: outcome.error ?? null,
    };
    const category = aggregateCategory(assertions);
    return {
        observation: {
            caseId: definition.id,
            kind: "convert",
            legs: [],
            durationMs: Math.round(performance.now() - started),
            capabilities: { version: "unavailable", formats: [], conversions: [] },
            details,
        },
        result: {
            id: definition.id,
            title: `${definition.format} ${definition.classification} sample`,
            kind: "convert",
            status: category === undefined ? "pass" : "fail",
            category,
            assertions,
            durationMs: 0,
        },
    };
}

function entryDetail(outcome: CaseOutcome): string {
    if (outcome.error !== undefined) {
        return outcome.error;
    }
    return outcome.success ? "sample read through its public entry" : "public entry reported failure";
}

function runNavigation(label: string, bytes: Uint8Array): CaseOutcome {
    const probe = probeNavigationSample(label, bytes);
    return {
        success: probe.success && probe.entries.length > 0,
        error: probe.error,
        graphJson: null,
        lossesJson: "[]",
        kind: "nav-toc",
        sheets: [],
        slides: 0,
        navigation: probe.entries,
    };
}

function runImport(definition: FormatCase, label: string): CaseOutcome {
    const bytes = caseBytes(definition);
    const imported = importFormatSample(definition.format, label, bytes);
    return {
        success: imported.success,
        error: imported.error,
        graphJson: imported.graphJson,
        lossesJson: imported.lossesJson,
        kind: imported.kind,
        sheets: imported.sheets,
        slides: imported.slides,
        navigation: [],
    };
}

function readExport(
    definition: FormatCase,
    outcome: CaseOutcome,
): { readonly target: "text" | "html"; readonly body: string } | null {
    const expect = definition.expect;
    const requested = typeof expect.export === "string" ? expect.export : definition.format === "markdown" ? "markdown" : undefined;
    if (requested === undefined || outcome.graphJson === null) {
        return null;
    }
    const target = requested === "html" ? "html" : "text";
    const exported = exportFormatSample(requested, outcome.graphJson);
    const body = target === "html" ? (exported.html ?? "") : (exported.text ?? "");
    return { target, body };
}

function applyGraphFacts(
    definition: FormatCase,
    facts: Facts,
    check: (name: string, ok: boolean, detail: string) => void,
): void {
    const expect = definition.expect;
    if (Number.isInteger(expect.blocks)) {
        const expected = Number(expect.blocks);
        check("block-count", facts.blockCount === expected, `blocks ${facts.blockCount}, expected ${expected}`);
    }
    if (Array.isArray(expect.listNumbers)) {
        const expected = (expect.listNumbers as unknown[]).map(Number);
        const observed = facts.orderedNumbers[0] ?? [];
        const same = expected.length === observed.length && expected.every((value, index) => value === observed[index]);
        check("list-numbers", same, `ordered list numbers ${JSON.stringify(observed)}, expected ${JSON.stringify(expected)}`);
    }
    if (Array.isArray(expect.noteLabels)) {
        const expected = (expect.noteLabels as unknown[]).map(String);
        const same = expected.length === facts.noteLabels.length && expected.every((value, index) => value === facts.noteLabels[index]);
        check("note-labels", same, `note labels ${JSON.stringify(facts.noteLabels)}, expected ${JSON.stringify(expected)}`);
    }
    if (Array.isArray(expect.sectionLevels)) {
        const expected = (expect.sectionLevels as unknown[]).map(Number);
        const observed = facts.sections.map((section) => section.level);
        const same = expected.length === observed.length && expected.every((value, index) => value === observed[index]);
        check("section-levels", same, `section levels ${JSON.stringify(observed)}, expected ${JSON.stringify(expected)}`);
    }
    if (Array.isArray(expect.sectionTitles)) {
        const expected = (expect.sectionTitles as unknown[]).map(String);
        const observed = facts.sections.map((section) => section.title);
        const same = expected.length === observed.length && expected.every((value, index) => value === observed[index]);
        check("section-titles", same, `section titles ${JSON.stringify(observed)}, expected ${JSON.stringify(expected)}`);
    }
    if (Array.isArray(expect.paragraphs)) {
        const expected = (expect.paragraphs as unknown[]).map(String);
        const same =
            expected.length === facts.paragraphs.length && expected.every((value, index) => value === facts.paragraphs[index]);
        check("paragraphs", same, `paragraphs ${JSON.stringify(facts.paragraphs)}, expected ${JSON.stringify(expected)}`);
    }
    if (Array.isArray(expect.styles)) {
        const expected = (expect.styles as unknown[]).map(String);
        const same = expected.length === facts.styles.length && expected.every((value, index) => value === facts.styles[index]);
        check("styles", same, `styles ${JSON.stringify(facts.styles)}, expected ${JSON.stringify(expected)}`);
    }
    if (typeof expect.plainText === "string") {
        check(
            "plain-text",
            facts.text === expect.plainText,
            `plain text ${JSON.stringify(facts.text)}, expected ${JSON.stringify(expect.plainText)}`,
        );
    }
}

function readFacts(graphJson: string): Facts {
    const graph = JSON.parse(graphJson) as { blocks?: unknown[] };
    const facts: Facts = {
        blockCount: Array.isArray(graph.blocks) ? graph.blocks.length : 0,
        sections: [],
        paragraphs: [],
        orderedNumbers: [],
        styles: [],
        noteLabels: [],
        text: "",
    };
    for (const block of graph.blocks ?? []) {
        const value = block as { block?: Record<string, unknown> };
        const node = value.block;
        if (node === undefined) {
            continue;
        }
        const kind = Object.keys(node)[0];
        const body = node[kind] as Record<string, unknown>;
        if (kind === "Section") {
            const title = inlineText(body.title);
            facts.sections.push({ level: Number(body.level ?? 0), title });
            facts.text += title;
        } else if (kind === "Paragraph") {
            walk(body.content, facts);
            facts.paragraphs.push(inlineText(body.content));
        } else if (kind === "List") {
            const items = Array.isArray(body.items) ? body.items : [];
            if (body.ordered === true) {
                facts.orderedNumbers.push(
                    items.map((item) => Number((item as Record<string, unknown>).number ?? NaN)).filter((number) => Number.isFinite(number)),
                );
            }
            for (const item of items) {
                walk((item as Record<string, unknown>).content, facts);
                walk((item as Record<string, unknown>).children, facts);
            }
        } else if (kind === "Table") {
            for (const row of Array.isArray(body.rows) ? body.rows : []) {
                for (const cell of Array.isArray((row as Record<string, unknown>).cells) ? (row as Record<string, unknown>).cells as unknown[] : []) {
                    walk((cell as Record<string, unknown>).content, facts);
                }
            }
        }
    }
    return facts;
}

function walk(inlines: unknown, facts: Facts): void {
    if (!Array.isArray(inlines)) {
        return;
    }
    for (const inline of inlines) {
        if (inline === null || typeof inline !== "object") {
            continue;
        }
        const wrapper = inline as Record<string, unknown>;
        const kind = Object.keys(wrapper)[0];
        const body = wrapper[kind] as Record<string, unknown>;
        if (kind === "Text") {
            facts.text += String(body.text ?? "");
        } else if (kind === "Styled") {
            const children = body.children as unknown;
            facts.styles.push(`${String(body.style ?? "")}:${inlineText(children)}`);
            walk(children, facts);
        } else if (kind === "Attributed") {
            walk(body.children, facts);
        } else if (kind === "Link") {
            walk(body.display, facts);
        } else if (kind === "Note") {
            facts.noteLabels.push(String(body.label ?? ""));
        }
    }
}

function inlineText(inlines: unknown): string {
    if (!Array.isArray(inlines)) {
        return "";
    }
    let text = "";
    for (const inline of inlines) {
        if (inline === null || typeof inline !== "object") {
            continue;
        }
        const wrapper = inline as Record<string, unknown>;
        const kind = Object.keys(wrapper)[0];
        const body = wrapper[kind] as Record<string, unknown>;
        if (kind === "Text") {
            text += String(body.text ?? "");
        } else if (kind === "Styled") {
            text += inlineText(body.children);
        } else if (kind === "Attributed") {
            text += inlineText(body.children);
        } else if (kind === "Link") {
            text += inlineText(body.display);
        }
    }
    return text;
}

function readLossCodes(json: string): string[] {
    try {
        const rows = JSON.parse(json) as unknown;
        return Array.isArray(rows)
            ? rows.map((row) => String((row as Record<string, unknown>).code ?? "")).filter((code) => code.length > 0)
            : [];
    } catch {
        return [];
    }
}

function caseBytes(definition: FormatCase): Uint8Array {
    const raw = readFixtureBytes(`formats/${definition.path}`);
    if (definition.container === "xhtml") {
        return epubPackage(raw);
    }
    if (definition.container === "package-part") {
        if (definition.format !== "docx") {
            throw new Error(`package-part sample \`${definition.id}\` is not DOCX and has no part wrapper`);
        }
        return storedZip([{ path: "word/document.xml", payload: raw }]);
    }
    return raw;
}

const containerXml = Buffer.from(
    `<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
</container>`,
);

function epubPackage(xhtml: Uint8Array): Uint8Array {
    const opf = Buffer.from(
        `<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Sample</dc:title><dc:language>en</dc:language></metadata>
  <manifest><item id="ch1" href="chapter.xhtml" media-type="application/xhtml+xml"/></manifest>
  <spine><itemref idref="ch1"/></spine>
</package>`,
    );
    return storedZip([
        { path: "mimetype", payload: Buffer.from("application/epub+zip") },
        { path: "META-INF/container.xml", payload: containerXml },
        { path: "OEBPS/content.opf", payload: opf },
        { path: "OEBPS/chapter.xhtml", payload: xhtml },
    ]);
}

function crc32(data: Uint8Array): number {
    let crc = 0xffffffff;
    for (const byte of data) {
        crc ^= byte;
        for (let bit = 0; bit < 8; bit += 1) {
            crc = (crc >>> 1) ^ (0xedb88320 & -(crc & 1));
        }
    }
    return (crc ^ 0xffffffff) >>> 0;
}

function storedZip(entries: ReadonlyArray<{ path: string; payload: Uint8Array }>): Buffer {
    const local: Buffer[] = [];
    const central: Buffer[] = [];
    for (const entry of entries) {
        const name = Buffer.from(entry.path, "utf8");
        const payload = Buffer.from(entry.payload);
        const crc = crc32(payload);
        const offset = Buffer.concat(local).length;
        local.push(Buffer.from("PK\x03\x04"), Buffer.from([20, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
        const tail = Buffer.alloc(8);
        tail.writeUInt32LE(crc, 0);
        tail.writeUInt32LE(payload.length, 4);
        local.push(tail, Buffer.from(new Uint32Array([payload.length]).buffer));
        const lengths = Buffer.alloc(4);
        lengths.writeUInt16LE(name.length, 0);
        local.push(lengths, name, payload);

        central.push(Buffer.from("PK\x01\x02"));
        const fixed = Buffer.alloc(42);
        fixed.writeUInt16LE(0x14, 0);
        fixed.writeUInt16LE(0x14, 2);
        fixed.writeUInt32LE(crc, 12);
        fixed.writeUInt32LE(payload.length, 16);
        fixed.writeUInt32LE(payload.length, 20);
        fixed.writeUInt16LE(name.length, 24);
        fixed.writeUInt32LE(offset, 38);
        central.push(fixed, name);
    }
    const cdOffset = Buffer.concat(local).length;
    const directory = Buffer.concat(central);
    local.push(directory, Buffer.from("PK\x05\x06"), Buffer.from([0, 0, 0, 0]));
    local.push(Buffer.from(new Uint16Array([entries.length, entries.length]).buffer));
    const end = Buffer.alloc(8);
    end.writeUInt32LE(directory.length, 0);
    end.writeUInt32LE(cdOffset, 4);
    local.push(end, Buffer.from([0, 0]));
    return Buffer.concat(local);
}
