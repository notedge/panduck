//! 乐谱与化学结构 fence 是各自独立的 IR 管线：源文本必须在 Markdown 与 HTML
//! 往返中保真，带上作者跨度，并且不得重新打开成 `Block::Diagram` 或 `Block::Chart`。

use std::collections::HashMap;

use document_formats::{
    export::{
        html::{RenderedDiagram, export_html, export_html_with_diagrams},
        markdown::export_markdown,
    },
    import::{html::import_html_bytes, markdown::import_markdown_bytes},
};
use document_ir::{Block, DocumentGraph, NodeId};

const ABC: &str = "X:1\nT:Smoke\nM:4/4\nL:1/4\nK:C\nCDEF|";
const SMILES: &str = "CCO";

fn fenced(language: &str, source: &str) -> String {
    format!("```{language}\n{source}\n```\n")
}

fn import(language: &str, source: &str) -> DocumentGraph {
    import_markdown_bytes("asset.md", &fenced(language, source)).expect("import fence markdown")
}

fn engraving_block(graph: &DocumentGraph) -> (&str, &str, NodeId) {
    let node = graph
        .blocks
        .iter()
        .find(|node| matches!(node.block, Block::Engraving { .. }))
        .unwrap_or_else(|| panic!("score expected: {:#?}", graph.blocks));
    let Block::Engraving { language, source } = &node.block
    else {
        unreachable!()
    };
    (language, source, node.id)
}

fn depiction_block(graph: &DocumentGraph) -> (&str, &str, NodeId) {
    let node = graph
        .blocks
        .iter()
        .find(|node| matches!(node.block, Block::Depiction { .. }))
        .unwrap_or_else(|| panic!("structure expected: {:#?}", graph.blocks));
    let Block::Depiction { language, source } = &node.block
    else {
        unreachable!()
    };
    (language, source, node.id)
}

#[test]
fn engraving_fence_becomes_an_engraving_block_with_its_authored_span() {
    for language in ["abc", "musicxml"] {
        let graph = import(language, ABC);
        let (found, source, id) = engraving_block(&graph);
        assert_eq!(found, language);
        assert_eq!(source, ABC);
        assert!(
            !graph.blocks.iter().any(|node| matches!(node.block, Block::Diagram { .. } | Block::Chart { .. })),
            "score fences must not enter the diagram or chart pipeline"
        );

        let span = graph.block_span(id).expect("score block keeps its authored span");
        let text = fenced(language, ABC);
        assert_eq!(&text[span.start as usize..span.end as usize], ABC);
    }
}

#[test]
fn depiction_fence_becomes_a_depiction_block_for_every_alias() {
    for language in ["smiles", "mol", "mdl", "sdf", "rd"] {
        let graph = import(language, SMILES);
        let (found, source, _) = depiction_block(&graph);
        assert_eq!(found, language);
        assert_eq!(source, SMILES);
        assert!(
            !graph.blocks.iter().any(|node| matches!(node.block, Block::Diagram { .. } | Block::Chart { .. })),
            "structure fences must not enter the diagram or chart pipeline"
        );
    }
}

#[test]
fn engraving_source_round_trips_through_markdown_and_html() {
    let graph = import("abc", ABC);

    let markdown = export_markdown(&graph).expect("export score markdown");
    assert!(markdown.contains("```abc\n"), "{markdown}");
    let reopened = import_markdown_bytes("reopened.md", &markdown).expect("reopen score markdown");
    let (language, source, _) = engraving_block(&reopened);
    assert_eq!(language, "abc");
    assert_eq!(source, ABC);

    let html = export_html(&graph).expect("export score html");
    assert!(html.contains("data-engraving-language=\"abc\""), "{html}");
    assert!(html.contains("<pre class=\"engraving\""), "{html}");
    let from_html = import_html_bytes("reopened.html", html.as_bytes()).expect("reopen score html");
    let (html_language, html_source, _) = engraving_block(&from_html);
    assert_eq!(html_language, "abc");
    assert_eq!(html_source, ABC);
}

#[test]
fn depiction_source_round_trips_through_markdown_and_html() {
    let graph = import("smiles", SMILES);

    let markdown = export_markdown(&graph).expect("export structure markdown");
    assert!(markdown.contains("```smiles\n"), "{markdown}");
    let reopened = import_markdown_bytes("reopened.md", &markdown).expect("reopen structure markdown");
    let (language, source, _) = depiction_block(&reopened);
    assert_eq!(language, "smiles");
    assert_eq!(source, SMILES);

    let html = export_html(&graph).expect("export structure html");
    assert!(html.contains("data-depiction-language=\"smiles\""), "{html}");
    let from_html = import_html_bytes("reopened.html", html.as_bytes()).expect("reopen structure html");
    let (html_language, html_source, _) = depiction_block(&from_html);
    assert_eq!(html_language, "smiles");
    assert_eq!(html_source, SMILES);
}

#[test]
fn rendered_engraving_figure_keeps_the_source_marker() {
    let graph = import("abc", ABC);
    let (_, source, id) = engraving_block(&graph);
    let scenes = HashMap::from([(id, RenderedDiagram { svg: "<svg><g id=\"score\"/></svg>".to_string() })]);
    let html = export_html_with_diagrams(&graph, &scenes).expect("export rendered score html");

    assert!(html.contains("<figure class=\"engraving\""), "{html}");
    assert!(html.contains("data-engraving-language=\"abc\""), "{html}");
    assert!(html.contains("<svg><g id=\"score\"/></svg>"), "{html}");

    let from_html = import_html_bytes("rendered.html", html.as_bytes()).expect("reopen rendered score html");
    let (language, round_tripped, _) = engraving_block(&from_html);
    assert_eq!(language, "abc");
    assert_eq!(round_tripped, source);
}

#[test]
fn rendered_depiction_figure_keeps_the_source_marker() {
    let graph = import("smiles", SMILES);
    let (_, source, id) = depiction_block(&graph);
    let scenes = HashMap::from([(id, RenderedDiagram { svg: "<svg><g id=\"chem\"/></svg>".to_string() })]);
    let html = export_html_with_diagrams(&graph, &scenes).expect("export rendered structure html");

    assert!(html.contains("<figure class=\"depiction\""), "{html}");
    assert!(html.contains("data-depiction-language=\"smiles\""), "{html}");

    let from_html = import_html_bytes("rendered.html", html.as_bytes()).expect("reopen rendered structure html");
    let (language, round_tripped, _) = depiction_block(&from_html);
    assert_eq!(language, "smiles");
    assert_eq!(round_tripped, source);
}
