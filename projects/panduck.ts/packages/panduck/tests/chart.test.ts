import assert from "node:assert/strict";
import { mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { runPanduck } from "./helpers/cli.js";

const BAR_SPEC = '{"title":"Revenue","width":480,"height":320,"data":{"values":[{"region":"East","revenue":120},{"region":"West","revenue":90},{"region":"North","revenue":60}]},"mark":"bar","encoding":{"x":{"field":"region","type":"nominal"},"y":{"field":"revenue","type":"quantitative"}}}';
const LINE_SPEC = '{"title":"Trend","width":480,"height":320,"data":{"values":[{"t":1,"v":10},{"t":2,"v":14},{"t":3,"v":9}]},"mark":"line","encoding":{"x":{"field":"t","type":"quantitative"},"y":{"field":"v","type":"quantitative"}}}';
const POINT_SPEC = '{"title":"Scatter","width":480,"height":320,"data":{"values":[{"t":1,"v":10},{"t":2,"v":14},{"t":3,"v":9}]},"mark":"point","encoding":{"x":{"field":"t","type":"quantitative"},"y":{"field":"v","type":"quantitative"}}}';

const REJECTIONS: Array<{ label: string; code: string; spec: string }> = [
    {
        label: "transform",
        code: "davinci.chart.unsupported-transform",
        spec: '{"data":{"values":[{"a":1}]},"transform":[{"filter":"datum.a > 0"}],"mark":"bar","encoding":{"x":{"field":"a","type":"quantitative"},"y":{"field":"a","type":"quantitative"}}}',
    },
    {
        label: "composite-view",
        code: "davinci.chart.unsupported-view",
        spec: '{"layer":[{"mark":"bar","encoding":{"x":{"field":"a","type":"quantitative"}}}]}',
    },
    {
        label: "remote-data",
        code: "davinci.chart.remote-data",
        spec: '{"data":{"url":"https://example.com/data.json"},"mark":"bar","encoding":{"x":{"field":"a","type":"quantitative"}}}',
    },
];

type CliLoss = {
    code: string;
    message: string;
    status: string;
    span: { start: number; end: number } | null;
};

type CliReport = {
    status: string;
    losses: CliLoss[];
};

function chartDoc(language: string, spec: string): string {
    return ["# Chart", "", "```" + language, spec, "```", ""].join("\n");
}

async function convertHtml(markdown: string, name: string): Promise<{ html: string; report: CliReport }> {
    const workdir = await mkdtemp(join(tmpdir(), `panduck-${name}-`));
    const input = join(workdir, "source.md");
    const output = join(workdir, "source.html");
    await writeFile(input, markdown, "utf8");

    const run = runPanduck(["convert", input, "--from", "markdown", "--to", "html", "-o", output, "--diagnostics", "json"]);
    assert.equal(run.code, 0, run.stderr || run.stdout);
    return { html: await readFile(output, "utf8"), report: JSON.parse(run.stdout) as CliReport };
}

test("bar chart fences render real rect geometry on the declared canvas", async () => {
    const { html, report } = await convertHtml(chartDoc("vega-lite", BAR_SPEC), "chart-bar");

    assert.match(html, /<figure class="chart" data-chart-language="vega-lite"/);
    assert.match(html, /<svg[^>]*width="480"[^>]*height="320"[^>]*viewBox="0 0 480 320"/);
    assert.match(
        html,
        /<rect id="chart-bar-0" data-role="mark" data-source="East" x="67.870968" y="56" width="106.83871" height="208" fill="#4a7fbf"\/>/,
        "the first bar must carry compiled geometry",
    );
    assert.match(
        html,
        /<rect id="chart-bar-1" data-role="mark" data-source="West" x="186.580645" y="160" width="106.83871" height="104" fill="#4a7fbf"\/>/,
        "the second bar must carry compiled geometry",
    );
    assert.doesNotMatch(html, /<pre class="chart"/, "a rendered chart must not keep the source fallback");
    assert.equal(report.status, "success");
    assert.deepEqual(report.losses, []);
});

test("line chart fences render one trained polyline", async () => {
    const { html, report } = await convertHtml(chartDoc("chart", LINE_SPEC), "chart-line");

    assert.match(html, /<figure class="chart" data-chart-language="chart"/);
    assert.match(html, /<svg[^>]*width="480"[^>]*height="320"[^>]*viewBox="0 0 480 320"/);
    assert.match(
        html,
        /<path id="chart-line-0" data-role="mark" data-source="all" d="M56 222\.4 L240 56 L424 264" fill="none" stroke="#4a7fbf" stroke-width="2"\/>/,
    );
    assert.equal((html.match(/data-role="mark"/g) ?? []).length, 1, "a line chart has one mark, not one per row");
    assert.deepEqual(report.losses, []);
});

test("point chart fences render one circle per row", async () => {
    const { html, report } = await convertHtml(chartDoc("vega", POINT_SPEC), "chart-point");

    assert.match(html, /<figure class="chart" data-chart-language="vega"/);
    for (const expected of [
        /<circle id="chart-point-0" data-role="mark" data-source="1" cx="56" cy="222\.4" r="5" fill="#4a7fbf"\/>/,
        /<circle id="chart-point-1" data-role="mark" data-source="2" cx="240" cy="56" r="5" fill="#4a7fbf"\/>/,
        /<circle id="chart-point-2" data-role="mark" data-source="3" cx="424" cy="264" r="5" fill="#4a7fbf"\/>/,
    ]) {
        assert.match(html, expected);
    }
    assert.deepEqual(report.losses, []);
});

test("refused charts keep the visible source, the stable code, and a real span", async () => {
    for (const rejection of REJECTIONS) {
        const document = chartDoc("vega-lite", rejection.spec);
        const { html, report } = await convertHtml(document, `chart-reject-${rejection.label}`);

        assert.match(html, /<pre class="chart" data-chart-language="vega-lite"/, `${rejection.label}: the source fallback must stay visible`);
        assert.doesNotMatch(html, /<figure/, `${rejection.label}: a refused fence must not emit a figure`);
        assert.equal(report.status, "success_with_loss", rejection.label);

        const loss = report.losses.find((entry) => entry.code === rejection.code);
        assert.ok(loss, `${rejection.label}: expected ${rejection.code}, got ${JSON.stringify(report.losses)}`);
        assert.equal(loss.status, "Unsupported", rejection.label);
        assert.ok(loss.span, `${rejection.label}: the CLI report must carry the authored span`);
        assert.ok(loss.span.start > 0 && loss.span.end > loss.span.start, `${rejection.label}: ${JSON.stringify(loss.span)}`);
        assert.ok(
            document.slice(loss.span.start, loss.span.end).includes(rejection.spec.slice(0, 20)),
            `${rejection.label}: the span must address the authored fence body`,
        );
    }
});

test("chart denials fail the conversion while the fallback is still published", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-chart-deny-"));
    const input = join(workdir, "rejected.md");
    const output = join(workdir, "rejected.html");
    await writeFile(input, chartDoc("vega-lite", REJECTIONS[0].spec), "utf8");

    const run = runPanduck(["convert", input, "--from", "markdown", "--to", "html", "-o", output, "--loss", "deny", "--diagnostics", "json"]);
    assert.equal(run.code, 1, `deny policy must exit 1: ${run.stderr}`);
    const report = JSON.parse(run.stdout) as CliReport;
    assert.equal(report.status, "success_with_loss");
    assert.match(await readFile(output, "utf8"), /<pre class="chart" data-chart-language="vega-lite"/);
});

test("chart fences survive the html to markdown roundtrip and render identically again", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-chart-roundtrip-"));
    const input = join(workdir, "bar.md");
    const html = join(workdir, "bar.html");
    const back = join(workdir, "back.md");
    await writeFile(input, chartDoc("vega-lite", BAR_SPEC), "utf8");

    const toHtml = runPanduck(["convert", input, "--from", "markdown", "--to", "html", "-o", html, "--diagnostics", "silent"]);
    assert.equal(toHtml.code, 0, toHtml.stderr || toHtml.stdout);
    const toMarkdown = runPanduck(["convert", html, "--from", "html", "--to", "markdown", "-o", back, "--diagnostics", "silent"]);
    assert.equal(toMarkdown.code, 0, toMarkdown.stderr || toMarkdown.stdout);

    const markdown = await readFile(back, "utf8");
    assert.match(markdown, /```vega-lite\n/, "the chart fence language must survive the HTML leg");
    assert.ok(markdown.includes('"mark":"bar"'), "the authored spec must survive the HTML leg");

    const rerenderPath = join(workdir, "back.html");
    const rerender = runPanduck(["convert", back, "--from", "markdown", "--to", "html", "-o", rerenderPath, "--diagnostics", "json"]);
    assert.equal(rerender.code, 0, rerender.stderr || rerender.stdout);
    const rerendered = await readFile(rerenderPath, "utf8");
    assert.match(rerendered, /<figure class="chart" data-chart-language="vega-lite"/, "the reopened fence must render again");
    assert.equal(
        (rerendered.match(/<rect id="chart-bar-\d+" data-role="mark"/g) ?? []).length,
        (await readFile(html, "utf8")).match(/<rect id="chart-bar-\d+" data-role="mark"/g)?.length,
        "the reopened fence must reproduce the same marks",
    );
    assert.deepEqual((JSON.parse(rerender.stdout) as CliReport).losses, [], "the reopened fence must stay loss-free");
});
