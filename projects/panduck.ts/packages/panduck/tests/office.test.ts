import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

import { chromium } from "playwright";

type OfficeIssue = { code: string; status: string; message: string };
type SheetPage = {
    sheet: string;
    number: number;
    total: number;
    firstRow: number;
    lastRow: number;
    firstColumn: number;
    lastColumn: number;
    cellCount: number;
    id: string;
};
type SpreadsheetRender = {
    html: string;
    sheetCount: number;
    visibleSheetCount: number;
    cellCount: number;
    nonWhitespaceChars: number;
    pageCount: number;
    pages: SheetPage[];
    issues: OfficeIssue[];
};
type SlideCanvas = { index: number; widthPx: number; heightPx: number; svg: string; deviationCount: number };
type ChartScene = {
    slide: number;
    shape: number;
    kind: string | null;
    nodeCount: number;
    widthPx: number;
    heightPx: number;
    svg: string;
    error: string | null;
};
type PresentationRender = {
    html: string;
    slideCount: number;
    shapeCount: number;
    imageCount: number;
    tableCount: number;
    chartCount: number;
    notesCount: number;
    nonWhitespaceChars: number;
    slides: SlideCanvas[];
    charts: ChartScene[];
    issues: OfficeIssue[];
};
type WrapProbe = {
    slide: number;
    shapeId: number;
    shapeName: string;
    paragraph: number;
    fontSizePt: number;
    widthPx: number;
    estimatedLines: string[];
    measuredLines: string[];
};

type OfficeBindings = {
    renderSpreadsheet(label: string, bytes: Buffer): SpreadsheetRender;
    renderSlides(label: string, bytes: Buffer): PresentationRender;
    probeSlideTextWrap(label: string, bytes: Buffer): WrapProbe[];
};

const platformRequire = createRequire(import.meta.url);
const platformModule = platformRequire("@notedge/panduck-win32-x64") as OfficeBindings & { default?: OfficeBindings };
const native: OfficeBindings = platformModule.default ?? platformModule;
const fixtures = join(dirname(fileURLToPath(import.meta.url)), "../../panduck-fixtures/fixtures/formats");

/** Screenshots one element per selector and returns their absolute paths and byte sizes. */
async function capture(
    html: string,
    selectors: string[],
    name: string,
): Promise<Array<{ path: string; bytes: number }>> {
    const workdir = await mkdtemp(join(tmpdir(), `panduck-office-${name}-`));
    const source = join(workdir, `${name}.html`);
    await writeFile(source, html, "utf8");
    const evidence = join(tmpdir(), "panduck-office-evidence");
    await mkdir(evidence, { recursive: true });

    const browser = await chromium.launch();
    try {
        const tab = await browser.newPage({ viewport: { width: 1400, height: 1200 } });
        await tab.goto(`file://${source}`);
        const shots = [];
        for (const [index, selector] of selectors.entries()) {
            const element = tab.locator(selector);
            await element.waitFor({ state: "visible", timeout: 10_000 });
            const path = join(evidence, `${name}-${index + 1}.png`);
            await element.screenshot({ path });
            const bytes = (await readFile(path)).byteLength;
            assert.ok(bytes > 2_000, `${name} selector ${selector} produced only ${bytes} bytes`);
            console.log(`office evidence: ${path} (${bytes} bytes)`);
            shots.push({ path, bytes });
        }
        return shots;
    } finally {
        await browser.close();
    }
}

test("workbook fixture renders every visible sheet as a paginated grid with screenshots", async () => {
    const bytes = await readFile(join(fixtures, "xlsx/multi-sheet.xlsx"));
    const render = native.renderSpreadsheet("multi-sheet.xlsx", bytes);

    assert.equal(render.sheetCount, 3);
    assert.equal(render.visibleSheetCount, 2);
    assert.equal(render.cellCount, 49);
    assert.equal(render.nonWhitespaceChars, 411);
    assert.equal(render.pageCount, 3);
    assert.deepEqual(
        render.pages.map((page) => [page.sheet, page.number, page.total]),
        [
            ["Data", 1, 2],
            ["Data", 2, 2],
            ["Summary", 1, 1],
        ],
    );
    assert.equal(render.pages[0].cellCount, 96, "the frozen header band repeats on page one");
    assert.deepEqual(
        [...new Set(render.issues.map((issue) => issue.code))].sort(),
        [
            "import.xlsx.chart",
            "import.xlsx.conditional-formatting",
            "import.xlsx.data-validation",
            "import.xlsx.defined-name",
            "import.xlsx.table",
            "render.xlsx.frozen-repeated",
            "render.xlsx.merge-split",
        ],
    );
    assert.equal(
        render.issues.filter((issue) => issue.code === "import.xlsx.chart")[0]?.status,
        "unsupported",
        "a workbook chart stays refused",
    );

    assert.match(render.html, /page 1\/2/);
    assert.match(render.html, /\$3,970\.75/, "number formats survive the grid view");
    const shots = await capture(
        render.html,
        render.pages.map((page) => `#${page.id}`),
        "xlsx-multi-sheet",
    );
    assert.equal(shots.length, 3);
});

test("slide fixture paints three fixed canvases and lowers the chart into a scene", async () => {
    const bytes = await readFile(join(fixtures, "pptx/layouts-and-chart.pptx"));
    const render = native.renderSlides("layouts-and-chart.pptx", bytes);

    assert.equal(render.slideCount, 3);
    assert.equal(render.shapeCount, 13);
    assert.equal(render.imageCount, 2);
    assert.equal(render.tableCount, 1);
    assert.equal(render.chartCount, 1);
    assert.equal(render.notesCount, 1);
    assert.equal(render.nonWhitespaceChars, 334);
    assert.deepEqual(
        render.slides.map((slide) => [slide.index, Math.round(slide.widthPx), slide.heightPx]),
        [
            [1, 1280, 720],
            [2, 1280, 720],
            [3, 1280, 720],
        ],
    );

    assert.equal(render.charts.length, 1);
    const chart = render.charts[0];
    assert.equal(chart.slide, 2);
    assert.equal(chart.kind, "barChart");
    assert.equal(chart.nodeCount, 23, "the bar scene compiles to 23 semantic nodes");
    assert.ok(Math.abs(chart.widthPx - 691.2) < 0.1, `${chart.widthPx}`);
    assert.ok(Math.abs(chart.heightPx - 403.2) < 0.1, `${chart.heightPx}`);
    assert.ok(chart.error == null, "the bar family lowers without an error");
    assert.match(chart.svg, /<rect[^>]*data-role="mark"/, "the fragment carries painted bars");
    assert.equal((chart.svg.match(/data-role="mark"/g) ?? []).length, 4, "three bars plus the legend swatch");

    const slide = render.slides[1];
    assert.match(slide.svg, /<g transform="translate\(518\.4,182\.4\)">/, "the chart sits on its frame");
    assert.doesNotMatch(slide.svg, /chart refused/, "a lowered chart must not keep the placeholder frame");
    assert.ok(
        render.issues.every((issue) => !issue.code.startsWith("render.pptx.chart-placeholder")),
        JSON.stringify(render.issues),
    );

    const shots = await capture(
        render.html,
        render.slides.map((slide) => `#slide-${slide.index}`),
        "pptx-layouts-and-chart",
    );
    assert.equal(shots.length, 3);
});

test("real body advances wrap slide text differently from the em estimate", async () => {
    const bytes = await readFile(join(fixtures, "pptx/layouts-and-chart.pptx"));
    const probes = native.probeSlideTextWrap("layouts-and-chart.pptx", bytes);

    assert.equal(probes.length, 11);
    const estimated = probes.reduce((sum, probe) => sum + probe.estimatedLines.length, 0);
    const measured = probes.reduce((sum, probe) => sum + probe.measuredLines.length, 0);
    assert.equal(estimated, 15);
    assert.equal(measured, 12);
    console.log("estimated vs measured line counts");
    for (const probe of probes) {
        const delta = probe.measuredLines.length - probe.estimatedLines.length;
        console.log(
            `  slide ${probe.slide} ${probe.shapeName} p${probe.paragraph}: ${probe.estimatedLines.length} -> ${probe.measuredLines.length} (${delta})`,
        );
    }
    const changed = probes.filter((probe) => probe.measuredLines.length !== probe.estimatedLines.length);
    assert.equal(changed.length, 3, JSON.stringify(changed));
    assert.equal(changed[0].shapeName, "Rounded Rectangle 3");
    assert.equal(changed[0].measuredLines.length, 2, JSON.stringify(changed[0].measuredLines));
    assert.equal(changed[1].shapeName, "Content Placeholder 2");
    assert.equal(changed[1].slide, 3);
    assert.equal(changed[2].shapeName, "Content Placeholder 3");
    assert.equal(changed[2].slide, 3);
});
