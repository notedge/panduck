//! Chart fences are a separate IR pipeline from diagrams. The chart source
//! must survive Markdown and HTML round trips, carry its authored span, and
//! never reopen as `Block::Diagram`.

use std::collections::HashMap;

use document_formats::{
    export::{
        html::{RenderedDiagram, export_html, export_html_with_diagrams},
        markdown::export_markdown,
    },
    import::{html::import_html_bytes, markdown::import_markdown_bytes},
};
use document_ir::{Block, DocumentGraph, NodeId};

const SOURCE: &str = r#"{"mark":"bar","data":{"values":[{"category":"A","value":3},{"category":"B","value":5}]},"encoding":{"x":{"field":"category","type":"nominal"},"y":{"field":"value","type":"quantitative"}}}"#;

fn fenced(language: &str, source: &str) -> String {
    format!("```{language}\n{source}\n```\n")
}

fn chart_block(graph: &DocumentGraph) -> (&str, &str, NodeId) {
    let node = graph
        .blocks
        .iter()
        .find(|node| matches!(node.block, Block::Chart { .. }))
        .unwrap_or_else(|| panic!("chart expected: {:#?}", graph.blocks));
    let Block::Chart { language, source } = &node.block
    else {
        unreachable!()
    };
    (language, source, node.id)
}

fn import_chart(language: &str) -> DocumentGraph {
    import_markdown_bytes("chart.md", &fenced(language, SOURCE)).expect("import chart markdown")
}

#[test]
fn chart_fence_becomes_a_chart_block_with_its_authored_span() {
    for language in ["vega-lite", "vega", "chart"] {
        let graph = import_chart(language);
        let (found, source, _) = chart_block(&graph);
        assert_eq!(found, language);
        assert_eq!(source, SOURCE);
        assert!(
            !graph.blocks.iter().any(|node| matches!(node.block, Block::Diagram { .. })),
            "chart fences must not enter the diagram pipeline"
        );

        let node = graph.blocks.iter().find(|node| matches!(node.block, Block::Chart { .. })).expect("chart node");
        let span = graph.block_span(node.id).expect("chart block keeps its authored span");
        let text = fenced(language, SOURCE);
        assert_eq!(&text[span.start as usize..span.end as usize], SOURCE);
    }
}

#[test]
fn chart_source_round_trips_through_markdown_and_html() {
    for language in ["vega-lite", "vega", "chart"] {
        let graph = import_chart(language);
        let (_, source, _) = chart_block(&graph);
        assert_eq!(source, SOURCE);

        let markdown = export_markdown(&graph).expect("export chart markdown");
        assert!(markdown.contains(&format!("```{language}\n")), "{markdown}");
        let reopened = import_markdown_bytes("reopened.md", &markdown).expect("reopen chart markdown");
        let (reopened_language, reopened_source, _) = chart_block(&reopened);
        assert_eq!(reopened_language, language);
        assert_eq!(reopened_source, SOURCE);

        let html = export_html(&graph).expect("export chart html");
        assert!(html.contains(&format!("data-chart-language=\"{language}\"")), "{html}");
        let from_html = import_html_bytes("reopened.html", html.as_bytes()).expect("reopen chart html");
        let (html_language, html_source, _) = chart_block(&from_html);
        assert_eq!(html_language, language);
        assert_eq!(html_source, SOURCE);
    }
}

#[test]
fn rendered_chart_figure_keeps_the_source_marker() {
    let graph = import_chart("vega-lite");
    let (_, source, id) = chart_block(&graph);
    let charts = HashMap::from([(id, RenderedDiagram { svg: "<svg><rect width=\"10\" height=\"10\"/></svg>".to_string() })]);
    let html = export_html_with_diagrams(&graph, &charts).expect("export rendered chart html");

    assert!(html.contains("<figure class=\"chart\""), "{html}");
    assert!(html.contains("data-chart-language=\"vega-lite\""), "{html}");
    assert!(html.contains("<svg><rect"), "{html}");

    let from_html = import_html_bytes("rendered.html", html.as_bytes()).expect("reopen rendered chart html");
    let (language, round_tripped, _) = chart_block(&from_html);
    assert_eq!(language, "vega-lite");
    assert_eq!(round_tripped, source);
}
