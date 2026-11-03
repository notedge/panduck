import assert from "node:assert/strict";
import { mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { runPanduck } from "./helpers/cli.js";

const CLASS_DOC = [
    "# Class Doc",
    "",
    "```plantuml",
    "@startuml",
    "class Parser {",
    "  +parse(source) Document",
    "  -tokens: List",
    "  #state: int",
    "}",
    "class Renderer {",
    "  +render(scene) Svg",
    "}",
    "class Tokenizer {",
    "  +split(text) List",
    "}",
    "Parser <|-- Renderer",
    "Renderer --> Tokenizer",
    "@enduml",
    "```",
    "",
    "Closing paragraph.",
    "",
].join("\n");

const SEQUENCE_DOC = [
    "# Sequence Doc",
    "",
    "```plantuml",
    "@startuml",
    "actor User",
    "participant Parser",
    "participant Renderer",
    "User -> Parser: parse",
    "activate Parser",
    "Parser -> Renderer: render",
    "deactivate Parser",
    "@enduml",
    "```",
    "",
].join("\n");

const REJECTIONS: Array<{ label: string; construct: string; message: RegExp; source: string[] }> = [
    {
        label: "preprocessor directives",
        construct: "!include common.iuml",
        message: /PlantUML preprocessor directives such as `!include` and `!define` are refused/,
        source: ["@startuml", "!include common.iuml", "class A", "@enduml"],
    },
    {
        label: "skinparam",
        construct: "skinparam monochrome true",
        message: /PlantUML `skinparam` statements are not part of the accepted class and sequence subset/,
        source: ["@startuml", "skinparam monochrome true", "class A", "@enduml"],
    },
    {
        label: "note",
        construct: "note right of A: hi",
        message: /PlantUML `note` statements are not part of the accepted class and sequence subset/,
        source: ["@startuml", "class A", "note right of A: hi", "@enduml"],
    },
    {
        label: "alt fragment",
        construct: "alt ok",
        message: /PlantUML `alt` statements are not part of the accepted class and sequence subset/,
        source: ["@startuml", "participant A", "alt ok", "A -> B: go", "end", "@enduml"],
    },
    {
        label: "state diagrams",
        construct: "state Idle",
        message: /PlantUML `state` diagrams are refused, only class and sequence diagrams are lowered/,
        source: ["@startuml", "state Idle", "state Done", "Idle --> Done", "@enduml"],
    },
];

type CliReport = {
    status: string;
    losses: Array<{ code: string; message: string; status: string }>;
};

async function convert(
    markdown: string,
    name: string,
): Promise<{ html: string; svg: string; report: CliReport }> {
    const result = await convertHtml(markdown, name);
    const start = result.html.indexOf("<svg ");
    const end = result.html.indexOf("</svg>", start);
    assert.ok(start >= 0 && end > start, "the export must inline one SVG scene");
    return { html: result.html, svg: result.html.slice(start, end + "</svg>".length), report: result.report };
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

function unescapeEntities(value: string): string {
    return value
        .replaceAll("&lt;", "<")
        .replaceAll("&gt;", ">")
        .replaceAll("&quot;", "\"")
        .replaceAll("&#39;", "'")
        .replaceAll("&amp;", "&");
}

function labels(svg: string): string[] {
    return [...svg.matchAll(/<text[^>]*>([^<]+)<\/text>/g)].map((match) => unescapeEntities(match[1]));
}

function edgeGroups(svg: string): Array<{ source: string; body: string }> {
    return [...svg.matchAll(/<g [^>]*data-role="edge"[^>]*>([\s\S]*?)<\/g>/g)].map((match) => ({
        source: unescapeEntities(/data-source="([^"]*)"/.exec(match[0])?.[1] ?? ""),
        body: match[1],
    }));
}

test("plantuml class fences render members, visibility markers, and both edge kinds", async () => {
    const { html, svg, report } = await convert(CLASS_DOC, "uml-class");

    assert.match(html, /<figure class="diagram" data-diagram-language="plantuml"/, "the fence must render as a figure");
    assert.match(svg, /viewBox="0 0 [0-9.]+ [0-9.]+"/);

    const text = labels(svg);
    for (const expected of ["Parser", "Renderer", "Tokenizer", "+parse(source) Document", "-tokens: List", "#state: int", "+render(scene) Svg", "+split(text) List"]) {
        assert.ok(text.includes(expected), `missing class label ${expected}: ${text.join(" | ")}`);
    }

    const edges = edgeGroups(svg);
    assert.deepEqual(
        edges.map((edge) => edge.source).sort(),
        ["Renderer->Parser", "Renderer->Tokenizer"],
        "the inheritance and association relations must both survive as edges",
    );

    const inheritance = edges.find((edge) => edge.source === "Renderer->Parser");
    const association = edges.find((edge) => edge.source === "Renderer->Tokenizer");
    assert.ok(inheritance && association);
    assert.match(inheritance.body, /<polygon[^>]*stroke-width="1.5"/, "the hollow inheritance marker must stay stroked-only");
    assert.doesNotMatch(association.body, /stroke-width="1.5"/, "the association marker must not reuse the inheritance marker style");

    assert.equal(report.status, "success", `the accepted class subset must stay loss-free: ${JSON.stringify(report.losses)}`);
    assert.deepEqual(report.losses, []);
});

test("plantuml sequence fences render participants, messages, and activation intervals", async () => {
    const { html, svg, report } = await convert(SEQUENCE_DOC, "uml-sequence");

    assert.match(html, /<figure class="diagram" data-diagram-language="plantuml"/);

    const text = labels(svg);
    for (const expected of ["User", "Parser", "Renderer", "parse", "render"]) {
        assert.ok(text.includes(expected), `missing sequence label ${expected}: ${text.join(" | ")}`);
    }

    const edges = edgeGroups(svg);
    assert.deepEqual(
        edges.map((edge) => edge.source).sort(),
        ["Parser->Renderer", "User->Parser"],
        "both messages must lower to time-ordered edges",
    );

    const activation = /<rect[^>]*data-role="decoration"[^>]*data-source="Parser"[^>]*\/>/.exec(svg)?.[0];
    assert.ok(activation, "the activate and deactivate pair must emit one decoration rect");
    const width = Number(/\swidth="([0-9.]+)"/.exec(activation)?.[1]);
    const height = Number(/\sheight="([0-9.]+)"/.exec(activation)?.[1]);
    assert.ok(width > 0 && height > 0, `the activation interval must cover the messages: ${activation}`);
    assert.ok(width <= 12, `the activation bar must stay narrow: ${activation}`);

    assert.equal(report.status, "success", `the accepted sequence subset must stay loss-free: ${JSON.stringify(report.losses)}`);
    assert.deepEqual(report.losses, []);
});

test("plantuml constructs outside the subset keep the visible source and the stable loss", async () => {
    for (const rejection of REJECTIONS) {
        const body = ["# Rejected", "", "```plantuml", ...rejection.source, "```", ""].join("\n");
        const { html, report } = await convertHtml(body, `uml-reject-${rejection.label.replace(/\s+/g, "-")}`);

        assert.match(html, /<pre class="diagram" data-diagram-language="plantuml"/, `${rejection.label}: the source fallback must stay visible`);
        assert.ok(html.includes(rejection.construct), `${rejection.label}: the authored statement must stay readable`);
        assert.doesNotMatch(html, /<figure/, `${rejection.label}: a rejected fence must not emit a figure`);

        assert.equal(report.status, "success_with_loss", rejection.label);
        const loss = report.losses.find((entry) => entry.code === "davinci.diagram.unsupported");
        assert.ok(loss, `${rejection.label}: expected davinci.diagram.unsupported, got ${JSON.stringify(report.losses)}`);
        assert.equal(loss.status, "Unsupported", rejection.label);
        assert.match(loss.message, rejection.message, rejection.label);
    }
});

test("plantuml denials fail the conversion while the fallback is still published", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-uml-deny-"));
    const body = ["# Rejected", "", "```plantuml", ...REJECTIONS[0].source, "```", ""].join("\n");
    const input = join(workdir, "rejected.md");
    const output = join(workdir, "rejected.html");
    await writeFile(input, body, "utf8");

    const run = runPanduck(["convert", input, "--from", "markdown", "--to", "html", "-o", output, "--loss", "deny", "--diagnostics", "json"]);
    assert.equal(run.code, 1, `deny policy must exit 1: ${run.stderr}`);
    const report = JSON.parse(run.stdout) as CliReport;
    assert.equal(report.status, "success_with_loss");
    assert.match(await readFile(output, "utf8"), /<pre class="diagram" data-diagram-language="plantuml"/);
});

test("plantuml fences survive the html to markdown roundtrip and render again", async () => {
    const workdir = await mkdtemp(join(tmpdir(), "panduck-uml-roundtrip-"));
    const input = join(workdir, "class.md");
    const html = join(workdir, "class.html");
    const back = join(workdir, "back.md");
    await writeFile(input, CLASS_DOC, "utf8");

    const toHtml = runPanduck(["convert", input, "--from", "markdown", "--to", "html", "-o", html, "--diagnostics", "silent"]);
    assert.equal(toHtml.code, 0, toHtml.stderr || toHtml.stdout);
    const toMarkdown = runPanduck(["convert", html, "--from", "html", "--to", "markdown", "-o", back, "--diagnostics", "silent"]);
    assert.equal(toMarkdown.code, 0, toMarkdown.stderr || toMarkdown.stdout);

    const markdown = await readFile(back, "utf8");
    assert.match(markdown, /```plantuml\n/, "the PlantUML fence language must survive the HTML leg");
    assert.match(markdown, /@startuml/);
    assert.match(markdown, /Parser <\|-- Renderer/);
    assert.match(markdown, /Renderer --> Tokenizer/);

    const rerender = runPanduck(["convert", back, "--from", "markdown", "--to", "html", "-o", join(workdir, "back.html"), "--diagnostics", "json"]);
    assert.equal(rerender.code, 0, rerender.stderr || rerender.stdout);
    const rerendered = await readFile(join(workdir, "back.html"), "utf8");
    assert.match(rerendered, /<figure class="diagram" data-diagram-language="plantuml"/, "the reopened fence must render again");
    assert.deepEqual((JSON.parse(rerender.stdout) as CliReport).losses, [], "the reopened fence must stay loss-free");
});
