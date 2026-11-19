use std::collections::HashMap;

use document_formats::{
    export::{
        docx::export_docx_bytes,
        html::{RenderedDiagram, export_html, export_html_with_diagrams},
        markdown::export_markdown,
        pdf::export_pdf_bytes,
    },
    import::{docx::import_docx_bytes, html::import_html_bytes, markdown::import_markdown_bytes, pdf::import_pdf_bytes},
};
use document_ir::{Block, DocumentGraph, Inline, NodeId};

/// The source bodies include the fuzz boundaries a diagram fence must keep:
/// an empty body, a long body, backticks, a tilde run, a nested backtick fence,
/// and a Mermaid statement the diagram lowering does not know.
const DIAGRAM_SOURCES: &[(&str, &str)] = &[
    ("empty", ""),
    ("single-line", "flowchart TD"),
    ("ordinary", "flowchart TD\n  A[Start] --> B{Done?}\n  B -->|yes| C[Stop]"),
    ("backticks", "flowchart LR\n  A[\"`code`\"] --> B"),
    ("tilde-run", "flowchart TD\n  A[~~~] --> B"),
    ("nested-backtick-fence", "flowchart TD\n```\n  A --> B\n```"),
    ("unsupported-statement", "sequenceDiagram\n  Alice->>Bob: Hi\n  Note over Alice,Bob: unsupported"),
    ("escaped-characters", "flowchart TD\n  A[\"a & b < c > d ?\"] --> B"),
    (
        "long",
        "flowchart TD\n  N0[Alpha] --> N1[Beta]\n  N1 --> N2[Gamma]\n  N2 --> N3[Delta]\n  N3 --> N4[Epsilon]\n  N4 --> N5[Zeta]\n  N5 --> N6[Eta]\n  N6 --> N7[Theta]\n  N7 --> N8[Iota]\n  N8 --> N9[Kappa]\n  N9 --> N10[Lambda]\n  N10 --> N11[Mu]\n  N11 --> N12[Nu]",
    ),
];

/// DOT rises through the same fence, block, and export surfaces as Mermaid. The
/// corpus covers the same fence boundaries plus a rejected HTML label, which
/// must still survive the format round trips as authored text.
const DOT_SOURCES: &[(&str, &str)] = &[
    ("empty", ""),
    ("single-line", "digraph G"),
    ("ordinary", "digraph pipeline {\n  rankdir=LR;\n  A [shape=box]\n  A -> B [label=\"go\"]\n}"),
    ("backticks", "digraph {\n  A [label=\"`code`\"]\n}"),
    ("tilde-run", "digraph {\n  A [label=\"~~~\"]\n}"),
    ("nested-backtick-fence", "digraph {\n```\n  A -> B\n```\n}"),
    ("rejected-html-label", "digraph {\n  A [label=<b>bold</b>]\n}"),
    ("escaped-characters", "digraph {\n  A [label=\"a & b < c > d ?\"]\n}"),
    (
        "long",
        "digraph chain {\n  N0 -> N1\n  N1 -> N2\n  N2 -> N3\n  N3 -> N4\n  N4 -> N5\n  N5 -> N6\n  N6 -> N7\n  N7 -> N8\n  N8 -> N9\n  N9 -> N10\n  N10 -> N11\n  N11 -> N12\n}",
    ),
];

/// PlantUML rises through the same fence surfaces as Mermaid and DOT. The
/// corpus keeps the fence boundaries plus one rejected directive, which must
/// still reopen as authored text.
const PLANTUML_SOURCES: &[(&str, &str)] = &[
    ("empty", ""),
    ("single-line", "@startuml"),
    ("ordinary", "@startuml\nclass Order {\n  +String id\n  -void submit()\n}\nclass Item\nOrder o-- Item : contains\n@enduml"),
    ("backticks", "@startuml\nparticipant \"`code`\" as P\n@enduml"),
    ("tilde-run", "@startuml\n' ~~~\nclass A\n@enduml"),
    ("nested-backtick-fence", "@startuml\n```\nclass A\n```\n@enduml"),
    ("rejected-include", "@startuml\n!include other.puml\nclass A\n@enduml"),
    ("escaped-characters", "@startuml\nclass \"a & b < c > d ?\" as Q\n@enduml"),
    (
        "long",
        "@startuml\nclass N0\nclass N1\nclass N2\nN0 --> N1\nN1 --> N2\nN2 --> N3\nN3 --> N4\nN4 --> N5\nN5 --> N6\nN6 --> N7\nN7 --> N8\nN8 --> N9\nN9 --> N10\nN10 --> N11\nN11 --> N12\n@enduml",
    ),
];

/// Every fence language shares one corpus shape so a new language cannot
/// silently skip a round-trip surface.
const DIAGRAM_CORPORA: &[(&str, &[(&str, &str)])] = &[("mermaid", DIAGRAM_SOURCES), ("dot", DOT_SOURCES), ("plantuml", PLANTUML_SOURCES)];

fn fenced(language: &str, source: &str) -> String {
    let run = source.split(|character| character == '\n').map(longest_backtick_run).max().unwrap_or(0);
    let fence = "`".repeat((run + 1).max(3));
    format!("{fence}{language}\n{source}\n{fence}\n")
}

fn longest_backtick_run(line: &str) -> usize {
    let mut longest = 0;
    let mut current = 0;
    for character in line.chars() {
        if character == '`' {
            current += 1;
            longest = longest.max(current);
        }
        else {
            current = 0;
        }
    }
    longest
}

fn import_diagram(language: &str, source: &str) -> DocumentGraph {
    import_markdown_bytes("diagram.md", &fenced(language, source)).expect("import diagram markdown")
}

fn diagram_block(graph: &DocumentGraph, expected_language: &str) -> String {
    let block = graph
        .blocks
        .iter()
        .map(|node| &node.block)
        .find(|block| matches!(block, Block::Diagram { .. }))
        .unwrap_or_else(|| panic!("diagram expected: {:#?}", graph.blocks));
    let Block::Diagram { language, source } = block
    else {
        unreachable!()
    };
    assert_eq!(language, expected_language);
    source.clone()
}

#[test]
fn markdown_diagram_sources_survive_markdown_and_html_round_trips() {
    for &(language, corpus) in DIAGRAM_CORPORA {
        for (name, source) in corpus {
            let label = format!("{language}/{name}");
            let graph = import_diagram(language, source);
            assert_eq!(diagram_block(&graph, language), *source, "{label}");

            let markdown = export_markdown(&graph).expect("export diagram markdown");
            let reopened = import_markdown_bytes(&format!("{name}.md"), &markdown).expect("reopen diagram markdown");
            assert_eq!(diagram_block(&reopened, language), *source, "{label}\n{markdown}");

            let html = export_html(&graph).expect("export diagram html");
            assert!(html.contains(&format!("data-diagram-language=\"{language}\"")), "{label}\n{html}");
            let from_html = import_html_bytes(&format!("{name}.html"), html.as_bytes()).expect("reopen diagram html");
            assert_eq!(diagram_block(&from_html, language), *source, "{label}\n{html}");
        }
    }
}

#[test]
fn rendered_diagram_figures_round_trip_the_authored_source() {
    for &(language, corpus) in DIAGRAM_CORPORA {
        for (name, source) in corpus {
            let label = format!("{language}/{name}");
            let graph = import_diagram(language, source);
            let diagrams = graph
                .blocks
                .iter()
                .filter_map(|node| match &node.block {
                    Block::Diagram { .. } => {
                        Some((node.id, RenderedDiagram { svg: "<svg><text>rendered &amp; escaped</text></svg>".to_string() }))
                    }
                    _ => None,
                })
                .collect::<HashMap<NodeId, RenderedDiagram>>();
            assert_eq!(diagrams.len(), 1, "{label}");

            let html = export_html_with_diagrams(&graph, &diagrams).expect("export rendered diagram html");
            assert!(html.contains("<figure class=\"diagram\""), "{label}\n{html}");
            assert!(html.contains(&format!("data-diagram-language=\"{language}\"")), "{label}\n{html}");
            let from_html = import_html_bytes(&format!("{name}.rendered.html"), html.as_bytes()).expect("reopen rendered diagram html");
            assert_eq!(diagram_block(&from_html, language), *source, "{label}\n{html}");
        }
    }
}

#[test]
fn rendered_diagram_svg_with_self_closing_children_keeps_the_following_block() {
    let markdown = "```mermaid\nflowchart TD\n  A --> B\n```\n\nClosing paragraph.\n";
    let graph = import_markdown_bytes("rendered.md", markdown).expect("import rendered markdown");
    let diagrams = graph
        .blocks
        .iter()
        .filter_map(|node| match &node.block {
            Block::Diagram { .. } => Some((
                node.id,
                RenderedDiagram {
                    svg: "<svg viewBox=\"0 0 20 20\"><rect width=\"20\" height=\"20\"/><text x=\"4\" y=\"12\">A</text></svg>".to_string(),
                },
            )),
            _ => None,
        })
        .collect::<HashMap<NodeId, RenderedDiagram>>();
    let html = export_html_with_diagrams(&graph, &diagrams).expect("export rendered diagram html");
    let reopened = import_html_bytes("rendered.html", html.as_bytes()).expect("reopen rendered diagram html");
    assert_eq!(diagram_block(&reopened, "mermaid"), "flowchart TD\n  A --> B");
    let Block::Paragraph { content } = &reopened.blocks.last().expect("closing paragraph").block
    else {
        panic!("a self-closing SVG child must not swallow the closing paragraph: {:?}", reopened.blocks)
    };
    assert!(content.iter().map(Inline::plain_text).collect::<String>().contains("Closing paragraph"));
}

#[test]
fn mixed_diagram_languages_keep_typed_fences_through_html() {
    let markdown =
        "```mermaid\nflowchart LR\n  A --> B\n```\n\n```plantuml\n@startuml\nAlice -> Bob\n@enduml\n```\n\n```dot\ndigraph { a -> b }\n```\n";
    let graph = import_markdown_bytes("mixed.md", markdown).expect("import mixed diagram markdown");
    let diagrams = graph
        .blocks
        .iter()
        .filter_map(|node| match &node.block {
            Block::Diagram { .. } => Some((
                node.id,
                RenderedDiagram {
                    svg: "<svg viewBox=\"0 0 20 20\"><rect width=\"20\" height=\"20\"/><text x=\"4\" y=\"12\">A</text></svg>".to_string(),
                },
            )),
            _ => None,
        })
        .collect::<HashMap<NodeId, RenderedDiagram>>();
    let html = export_html_with_diagrams(&graph, &diagrams).expect("export mixed diagram html");
    let reopened = import_html_bytes("mixed.html", html.as_bytes()).expect("reopen mixed diagram html");
    let languages = reopened
        .blocks
        .iter()
        .filter_map(|node| match &node.block {
            Block::Diagram { language, .. } => Some(language.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(languages, vec!["mermaid", "plantuml", "dot"], "{:?}", reopened.blocks);
    let back = export_markdown(&reopened).expect("export mixed diagram markdown");
    assert!(back.contains("```mermaid\n"), "{back}");
    assert!(back.contains("```plantuml\n"), "{back}");
    assert!(back.contains("```dot\n"), "{back}");
}

#[test]
fn docx_diagram_paragraphs_preserve_language_and_source() {
    for &(language, corpus) in DIAGRAM_CORPORA {
        for (name, source) in corpus {
            let label = format!("{language}/{name}");
            let graph = import_diagram(language, source);
            let bytes = export_docx_bytes(&graph).expect("export diagram docx");
            let reopened = import_docx_bytes(&format!("{name}.docx"), &bytes).expect("reopen diagram docx");
            assert_eq!(diagram_block(&reopened, language), *source, "{label}");
        }
    }
}

#[test]
fn pdf_diagram_export_keeps_every_source_line_intact() {
    for &(language, corpus) in DIAGRAM_CORPORA {
        for (name, source) in corpus {
            let label = format!("{language}/{name}");
            let graph = import_diagram(language, source);
            let bytes = export_pdf_bytes(&graph).expect("export diagram pdf");
            let reopened = import_pdf_bytes(&format!("{name}.pdf"), &bytes).expect("reopen diagram pdf");
            let text = reopened
                .blocks
                .iter()
                .filter_map(|node| match &node.block {
                    Block::Paragraph { content } => Some(content.iter().map(Inline::plain_text).collect::<String>()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            for line in source.lines().filter(|line| !line.trim().is_empty()) {
                assert!(text.contains(line.trim()), "{label}: `{line}` missing from PDF text\n{text}");
            }
        }
    }
}
