import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { runPanduck } from "./helpers/cli.js";

type Rect = { x: number; y: number; width: number; height: number };
type Point = { x: number; y: number };
type Edge = { source: string; segments: Array<[Point, Point]> };
type Plate = { source: string; rect: Rect };

const BYPASS_SOURCE = [
    "# Routing",
    "",
    "Before the diagram and after it there is prose.",
    "",
    "```mermaid",
    "flowchart TD",
    "  A[Start] --> B{Condition}",
    "  B -->|yes| C[Apply rules]",
    "  B -->|no| D[Reject]",
    "  C --> E{Has loss?}",
    "  E -->|yes| F[Record loss]",
    "  E -->|no| G[Write target]",
    "  F -->|no| G",
    "  F -->|yes| H[Report]",
    "  H --> G",
    "```",
    "",
    "Closing prose.",
    "",
].join("\n");

const CLASS_SOURCE = [
    "# Classes",
    "",
    "```mermaid",
    "classDiagram",
    "  class Parser {",
    "    +parse(source) Document",
    "  }",
    "  class Renderer {",
    "    +render(scene) Svg",
    "  }",
    "  Parser --> Renderer",
    "```",
    "",
].join("\n");

function attr(tag: string, name: string): string | undefined {
    return new RegExp(`\\s${name}="([^"]*)"`).exec(tag)?.[1];
}

function num(tag: string, name: string): number {
    const raw = attr(tag, name);
    assert.ok(raw !== undefined, `missing attribute ${name} on ${tag}`);
    return Number(raw);
}

function inlineSvg(html: string): string {
    const start = html.indexOf("<svg ");
    assert.ok(start >= 0, "the export must inline an SVG diagram");
    const end = html.indexOf("</svg>", start);
    assert.ok(end > start, "the inline SVG must be closed");
    return html.slice(start, end + "</svg>".length);
}

function viewBox(svg: string): { width: number; height: number } {
    const match = /viewBox="0 0 ([0-9.]+) ([0-9.]+)"/.exec(svg);
    assert.ok(match, "the SVG must declare a viewBox");
    return { width: Number(match[1]), height: Number(match[2]) };
}

/** Bounding box of a `<rect>`, `<circle>`, `<ellipse>` or polygon `<path>`. */
function shapeBox(tag: string): Rect {
    if (tag.startsWith("<rect")) {
        return { x: num(tag, "x"), y: num(tag, "y"), width: num(tag, "width"), height: num(tag, "height") };
    }
    if (tag.startsWith("<circle")) {
        const r = num(tag, "r");
        return { x: num(tag, "cx") - r, y: num(tag, "cy") - r, width: r * 2, height: r * 2 };
    }
    if (tag.startsWith("<ellipse")) {
        const rx = num(tag, "rx");
        const ry = num(tag, "ry");
        return { x: num(tag, "cx") - rx, y: num(tag, "cy") - ry, width: rx * 2, height: ry * 2 };
    }
    const points = pathPoints(tag);
    assert.ok(points.length > 0, `node path must carry points: ${tag}`);
    const xs = points.map((point) => point.x);
    const ys = points.map((point) => point.y);
    const x = Math.min(...xs);
    const y = Math.min(...ys);
    return { x, y, width: Math.max(...xs) - x, height: Math.max(...ys) - y };
}

function pathPoints(tag: string): Point[] {
    const d = attr(tag, "d");
    assert.ok(d, `path must carry a d attribute: ${tag}`);
    const numbers = d.match(/-?[0-9]+(?:\.[0-9]+)?/g) ?? [];
    const points: Point[] = [];
    for (let index = 0; index + 1 < numbers.length; index += 2) {
        points.push({ x: Number(numbers[index]), y: Number(numbers[index + 1]) });
    }
    return points;
}

function roles(svg: string, role: string): string[] {
    return [...svg.matchAll(new RegExp(`<[a-z][^>]*data-role="${role}"[^>]*>`, "g"))].map((match) => match[0]);
}

function edges(svg: string): Edge[] {
    return [...svg.matchAll(/<g [^>]*data-role="edge"[^>]*>([\s\S]*?)<\/g>/g)].map((match) => {
        const head = match[0].slice(0, match[1].length === 0 ? match[0].indexOf(">") : match[0].indexOf(">") + 1);
        const source = attr(head, "data-source") ?? "";
        const points = [...match[1].matchAll(/<path [^>]*d="([^"]*)"[^>]*\/>/g)]
            .flatMap((path) => {
                const numbers = (path[1].match(/-?[0-9]+(?:\.[0-9]+)?/g) ?? []).map(Number);
                const list: Point[] = [];
                for (let index = 0; index + 1 < numbers.length; index += 2) {
                    list.push({ x: numbers[index], y: numbers[index + 1] });
                }
                return list;
            });
        const segments: Array<[Point, Point]> = [];
        for (let index = 0; index + 1 < points.length; index += 1) {
            segments.push([points[index], points[index + 1]]);
        }
        return { source, segments };
    });
}

function plates(svg: string): Plate[] {
    return roles(svg, "edge-label-bg").map((tag) => ({
        source: attr(tag, "data-source") ?? "",
        rect: shapeBox(tag),
    }));
}

function endpointsOf(source: string): string[] {
    return source.split(/-->|->|-\.-/).map((part) => part.trim()).filter(Boolean);
}

/** Liang-Barsky clipping: does the segment cross the rectangle interior? */
function segmentHitsRect(from: Point, to: Point, rect: Rect, inflate = 0): boolean {
    const left = rect.x - inflate;
    const right = rect.x + rect.width + inflate;
    const top = rect.y - inflate;
    const bottom = rect.y + rect.height + inflate;
    const dx = to.x - from.x;
    const dy = to.y - from.y;
    let t0 = 0;
    let t1 = 1;
    for (const [p, q] of [
        [-dx, from.x - left],
        [dx, right - from.x],
        [-dy, from.y - top],
        [dy, bottom - from.y],
    ] as Array<[number, number]>) {
        if (p === 0) {
            if (q < 0) {
                return false;
            }
            continue;
        }
        const t = q / p;
        if (p < 0) {
            if (t > t1) {
                return false;
            }
            if (t > t0) {
                t0 = t;
            }
        } else {
            if (t < t0) {
                return false;
            }
            if (t < t1) {
                t1 = t;
            }
        }
    }
    // Half-open intervals keep a route that only touches the border from counting as an entry.
    return t1 - t0 > 1e-9;
}

function insideCanvas(rect: Rect, canvas: { width: number; height: number }, tol = 0.01): boolean {
    return (
        rect.x >= -tol &&
        rect.y >= -tol &&
        rect.x + rect.width <= canvas.width + tol &&
        rect.y + rect.height <= canvas.height + tol
    );
}

function palette(svg: string, selector: string): { light: string; dark: string } {
    const style = /<style>([\s\S]*?)<\/style>/.exec(svg)?.[1];
    assert.ok(style, "the SVG must inline its theme tokens");
    const escaped = selector.replace(/[.*+?^$()|[\]\\]/g, "\\$&");
    const mediaIndex = style.indexOf("@media (prefers-color-scheme: dark)");
    assert.ok(mediaIndex > 0, "the style must declare the dark media query");
    const find = (part: string): string | undefined =>
        new RegExp(`\\.diagram \\.${escaped} \\{ fill: #([0-9a-f]{6})`).exec(part)?.[1];
    const light = find(style.slice(0, mediaIndex));
    assert.ok(light, `missing light token for ${selector}: ${style}`);
    const dark = find(style.slice(mediaIndex));
    assert.ok(dark, `missing dark token for ${selector}: ${style}`);
    return { light, dark };
}

function luminance(hex: string): number {
    const channels = [0, 2, 4].map((offset) => Number.parseInt(hex.slice(offset, offset + 2), 16) / 255);
    const linear = channels.map((value) => (value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4));
    return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2];
}

function contrast(a: string, b: string): number {
    const [high, low] = [luminance(a), luminance(b)].sort((left, right) => right - left);
    return (high + 0.05) / (low + 0.05);
}

async function convert(
    markdown: string,
    name: string,
): Promise<{ html: string; svg: string; report: { status: string; losses: Array<{ code: string }> } }> {
    const workdir = await mkdtemp(join(tmpdir(), `panduck-${name}-`));
    try {
        const input = join(workdir, "source.md");
        const output = join(workdir, "source.html");
        await writeFile(input, markdown, "utf8");
        const run = runPanduck([
            "convert",
            input,
            "--from",
            "markdown",
            "--to",
            "html",
            "--output",
            output,
            "--diagnostics",
            "json",
        ]);
        assert.equal(run.code, 0, run.stderr || run.stdout);
        const html = await readFile(output, "utf8");
        const report = JSON.parse(run.stdout) as { status: string; losses: Array<{ code: string }> };
        return { html, svg: inlineSvg(html), report };
    } finally {
        await rm(workdir, { recursive: true, force: true });
    }
}

test("emitted edges never enter a non-endpoint node and nothing leaves the canvas", async () => {
    const { svg } = await convert(BYPASS_SOURCE, "bypass");
    const canvas = viewBox(svg);
    const nodes = roles(svg, "node").map((tag) => ({ id: attr(tag, "data-source") ?? "", rect: shapeBox(tag) }));
    const clusters = roles(svg, "cluster").map(shapeBox);
    const routes = edges(svg);

    assert.ok(nodes.length >= 8, `the fixture must emit every node: ${nodes.length}`);
    assert.ok(routes.length >= 8, `the fixture must emit every edge: ${routes.length}`);

    for (const node of nodes) {
        assert.ok(insideCanvas(node.rect, canvas), `node ${node.id} is clipped: ${JSON.stringify(node.rect)}`);
    }
    for (const cluster of clusters) {
        assert.ok(insideCanvas(cluster, canvas), `cluster is clipped: ${JSON.stringify(cluster)}`);
    }
    for (const plate of plates(svg)) {
        assert.ok(insideCanvas(plate.rect, canvas), `label plate ${plate.source} is clipped`);
    }

    const offenders: string[] = [];
    for (const route of routes) {
        const ends = new Set(endpointsOf(route.source));
        for (const [from, to] of route.segments) {
            assert.ok(
                from.x >= -0.01 && from.y >= -0.01 && to.x <= canvas.width + 0.01 && to.y <= canvas.height + 0.01,
                `route ${route.source} leaves the canvas at ${JSON.stringify([from, to])}`,
            );
            for (const node of nodes) {
                if (ends.has(node.id)) {
                    continue;
                }
                // 4.75 mirrors the compiler clearance; a route may only touch the border.
                if (segmentHitsRect(from, to, node.rect, 0)) {
                    offenders.push(`${route.source} enters ${node.id}`);
                }
            }
        }
    }
    assert.deepEqual(offenders, [], `routes must detour around foreign nodes: ${offenders.join(", ")}`);

    for (const route of routes) {
        for (const [from, to] of route.segments) {
            for (const cluster of clusters) {
                assert.ok(
                    !segmentHitsRect(from, to, cluster, 0),
                    `route ${route.source} enters a cluster: ${JSON.stringify([from, to])}`,
                );
            }
        }
    }
});

test("edge label plates cover their labels and shield them from foreign routes", async () => {
    const { svg } = await convert(BYPASS_SOURCE, "plates");
    const labels = roles(svg, "edge-label").map((tag) => ({
        source: attr(tag, "data-source") ?? "",
        x: num(tag, "x"),
        y: num(tag, "y"),
        fontSize: num(tag, "font-size"),
        anchor: attr(tag, "text-anchor") ?? "start",
    }));
    const plateList = plates(svg);
    const routes = edges(svg);

    assert.ok(labels.length >= 4, `branch labels must survive: ${labels.length}`);
    assert.equal(plateList.length, labels.length, "every edge label needs one backing plate");

    for (const label of labels) {
        const plate = plateList.find((candidate) => candidate.source === label.source);
        assert.ok(plate, `no plate for label ${label.source}`);
        assert.equal(attr(roles(svg, "edge-label-bg").find((tag) => attr(tag, "data-source") === label.source) ?? "", "rx"), "2");
        assert.ok(
            label.x >= plate.rect.x && label.x <= plate.rect.x + plate.rect.width,
            `label ${label.source} is horizontally outside its plate`,
        );
        assert.ok(
            label.y >= plate.rect.y &&
                label.y <= plate.rect.y + plate.rect.height &&
                plate.rect.height >= label.fontSize,
            `label ${label.source} is vertically outside its plate`,
        );
        for (const route of routes) {
            if (endpointsOf(route.source).length === 0 || route.source === label.source) {
                continue;
            }
            for (const [from, to] of route.segments) {
                assert.ok(
                    !segmentHitsRect(from, to, plate.rect, 0),
                    `foreign route ${route.source} crosses the plate of ${label.source}`,
                );
            }
        }
    }
});

test("the emitted palette keeps WCAG AA contrast in light and dark", async () => {
    const { svg } = await convert(BYPASS_SOURCE, "contrast");
    const nodeFill = palette(svg, "diagram-node");
    const nodeText = palette(svg, "diagram-label");
    const edgeLabelText = palette(svg, "diagram-edge-label");
    const plateFill = palette(svg, "diagram-edge-label-bg");
    // Page backgrounds the SVG is designed to sit on.
    const pages = { light: "ffffff", dark: "0d1117" };

    for (const scheme of ["light", "dark"] as const) {
        const background = pages[scheme];
        assert.ok(
            contrast(nodeText[scheme], nodeFill[scheme]) >= 4.5,
            `${scheme}: node text ${contrast(nodeText[scheme], nodeFill[scheme]).toFixed(2)}:1`,
        );
        assert.ok(
            contrast(edgeLabelText[scheme], plateFill[scheme]) >= 4.5,
            `${scheme}: label text ${contrast(edgeLabelText[scheme], plateFill[scheme]).toFixed(2)}:1`,
        );
        assert.ok(
            contrast(edgeLabelText[scheme], background) >= 4.5,
            `${scheme}: edge stroke ${contrast(edgeLabelText[scheme], background).toFixed(2)}:1`,
        );
        assert.ok(
            contrast(nodeText[scheme], background) >= 4.5,
            `${scheme}: body text ${contrast(nodeText[scheme], background).toFixed(2)}:1`,
        );
    }
    assert.notEqual(nodeFill.light, nodeFill.dark, "the two schemes must not share one palette");
});

test("sequence diagrams render through the time-ordered plan", async () => {
    const source = [
        "# Interaction",
        "",
        "```mermaid",
        "sequenceDiagram",
        "  participant C as Client",
        "  participant S as Server",
        "  C->>S: request",
        "  S->>S: validate",
        "  S-->>C: response",
        "```",
        "",
    ].join("\n");
    const first = await convert(source, "sequence-first");
    const second = await convert(source, "sequence-second");

    assert.equal(first.html, second.html, "the same source must render byte identically");
    assert.match(first.html, /<figure class="diagram" data-diagram-language="mermaid"/);
    assert.ok(!first.html.includes("<pre class=\"diagram\""), "a time-ordered fence must not fall back to source text");
    assert.equal(roles(first.svg, "node").length, 2, "both participants need a lifeline");
    assert.ok(edges(first.svg).length >= 2, "both messages must become edges");
    assert.ok(["success", "success_with_loss"].includes(first.report.status), first.report.status);
    for (const loss of first.report.losses) {
        assert.match(loss.code, /^davinci\.diagram\./, `unexpected loss from another domain: ${loss.code}`);
    }
});

test("class diagrams render through the spec and rerender byte identically", async () => {
    const first = await convert(CLASS_SOURCE, "class-first");
    const second = await convert(CLASS_SOURCE, "class-second");

    assert.equal(first.html, second.html, "the same source must render byte identically");
    assert.match(first.html, /<figure class="diagram" data-diagram-language="mermaid"/);
    assert.ok(!first.html.includes("<pre class=\"diagram\""), "a supported diagram must not fall back to source text");
    assert.ok(roles(first.svg, "node").length >= 2, "class boxes must become scene nodes");
    assert.ok(edges(first.svg).length >= 1, "the association must become an edge");
    for (const tag of roles(first.svg, "node")) {
        assert.ok(insideCanvas(shapeBox(tag), viewBox(first.svg)), `class node is clipped: ${tag}`);
    }
});
