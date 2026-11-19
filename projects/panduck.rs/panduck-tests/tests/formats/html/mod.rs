use std::collections::HashMap;

use document_formats::export::html::{RenderedDiagram, export_html, export_html_report, export_html_with_diagrams};
use document_ir::{
    Asset, AssetId, AssetKind, Block, DocumentGraph, DocumentId, DocumentMetadata, IdAllocator, Inline, ListItem, NodeId, RelationEndpoint,
    SemanticStatus, TableCell, TableRow,
};
use oak_core::{Builder, SourceText, parser::session::ParseSession};
use oak_html::{
    HtmlBuilder, HtmlLanguage,
    query::{HtmlDocumentView, select_css_elements},
};

#[test]
fn html_export_preserves_structure_and_escapes_content() {
    let mut ids = IdAllocator::default();
    let mut graph = DocumentGraph::new(ids.document_id());
    graph.metadata =
        DocumentMetadata { title: Some("<Title>".into()), language: None, authors: Vec::new(), tags: Vec::new(), properties: Vec::new() };
    graph.push_block(Block::Paragraph {
        content: vec![
            Inline::Text { text: "a < b".into() },
            Inline::Styled { style: "bold".into(), children: vec![Inline::Text { text: "strong".into() }] },
        ],
    });
    let html = export_html(&graph).expect("html export");
    assert!(html.contains("&lt;Title&gt;"));
    assert!(html.contains("<strong>strong</strong>"));
    assert!(html.contains("a &lt; b"));
    assert!(html.contains(r#"id="node-1" data-node-id="1""#));
}

#[test]
fn html_export_preserves_value_carrying_color_style() {
    let mut graph = DocumentGraph::new(DocumentId(60));
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Styled { style: "color:ff0000".into(), children: vec![Inline::Text { text: "red".into() }] }],
    });
    let html = export_html(&graph).expect("html color export");
    assert!(html.contains("<span style=\"color:#ff0000\">red</span>"));
}

#[test]
fn html_export_writes_section_children_and_reference_targets() {
    let mut ids = IdAllocator::default();
    let mut graph = DocumentGraph::new(ids.document_id());
    let paragraph = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "child".into() }] });
    let section =
        graph.push_block(Block::Section { level: 2, title: vec![Inline::Text { text: "Section".into() }], children: vec![paragraph] });
    graph.push_block(Block::Paragraph { content: vec![Inline::Reference { display: "go".into(), target: section }] });

    let html = export_html(&graph).expect("html export");
    assert!(html.contains("<h2>Section</h2>"));
    assert!(html.contains(r#"id="node-1" data-node-id="1"><p>child</p>"#));
    assert!(html.contains(r##"<a href="#node-2">go</a>"##));
}

#[test]
fn html_export_reopens_through_oak_html_and_css_selectors() {
    let mut graph = DocumentGraph::new(IdAllocator::default().document_id());
    graph.metadata.title = Some("Reopen <test>".into());
    graph.push_block(Block::Section { level: 2, title: vec![Inline::Text { text: "Heading".into() }], children: Vec::new() });
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Link {
            target: RelationEndpoint::ExternalUri("https://example.com".into()),
            display: vec![Inline::Text { text: "link".into() }],
            title: None,
        }],
    });

    let html = export_html(&graph).expect("html export");
    let source = SourceText::new(html.as_str());
    let language = HtmlLanguage::default();
    let builder = HtmlBuilder::new(language);
    let mut session = ParseSession::<HtmlLanguage>::default();
    let document = builder.build(&source, &[], &mut session).result.expect("Oak HTML reopen");
    let view = HtmlDocumentView::from_document(&document);

    let (_, headings) = select_css_elements(&view, "h1, h2", Default::default()).expect("heading selector");
    let (_, paragraphs) = select_css_elements(&view, "p", Default::default()).expect("paragraph selector");
    let (_, links) = select_css_elements(&view, "a[href]", Default::default()).expect("link selector");

    assert_eq!(headings.len(), 2);
    assert_eq!(paragraphs.len(), 1);
    assert_eq!(links.len(), 1);
}

#[test]
fn html_export_writes_image_inline_semantics() {
    let mut graph = DocumentGraph::new(IdAllocator::default().document_id());
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Image {
            source: "assets/diagram.png".into(),
            alt: vec![Inline::Text { text: "diagram".into() }],
            title: None,
            asset: None,
        }],
    });

    let html = export_html(&graph).expect("html export");
    assert!(html.contains(r#"<img alt="diagram" src="assets/diagram.png">"#));
}

#[test]
fn html_export_keeps_an_unrepresentable_inline_style_as_a_class() {
    let mut graph = DocumentGraph::new(IdAllocator::default().document_id());
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Styled { style: "small_caps".into(), children: vec![Inline::Text { text: "meaningful".into() }] }],
    });

    let html = export_html(&graph).expect("a style with no HTML tag still has a class slot");
    assert!(html.contains(r#"<span class="small_caps">meaningful</span>"#), "{html}");
}

#[test]
fn html_export_rejects_missing_nested_blocks_instead_of_dropping_them() {
    let mut graph = DocumentGraph::new(IdAllocator::default().document_id());
    graph.push_block(Block::Section {
        level: 1,
        title: vec![Inline::Text { text: "Broken".into() }],
        children: vec![document_ir::NodeId(999)],
    });

    let error = export_html(&graph).expect_err("missing child must be rejected");
    assert!(error.to_string().contains("invalid document graph"));
}

#[test]
fn html_export_preserves_list_children_and_their_reference_targets() {
    let mut graph = DocumentGraph::new(IdAllocator::default().document_id());
    let paragraph = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "child paragraph".into() }] });
    let code = graph.push_block(Block::Code { language: Some("rust".into()), content: "value < limit".into() });
    let quote_paragraph = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "child quote".into() }] });
    let quote = graph.push_block(Block::Quote { children: vec![quote_paragraph] });
    let nested_list = graph.push_block(Block::List {
        ordered: true,
        items: vec![document_ir::ListItem {
            content: vec![Inline::Text { text: "nested item".into() }],
            children: Vec::new(),
            number: Some(1),
            ..Default::default()
        }],
    });
    graph.push_block(Block::List {
        ordered: false,
        items: vec![document_ir::ListItem {
            content: vec![Inline::Reference { display: "nested target".into(), target: nested_list }],
            children: vec![paragraph, code, quote, nested_list],
            number: None,
            ..Default::default()
        }],
    });

    let html = export_html(&graph).expect("list children must be preserved");
    let source = SourceText::new(html.as_str());
    let builder = HtmlBuilder::new(HtmlLanguage::default());
    let mut session = ParseSession::<HtmlLanguage>::default();
    let document = builder.build(&source, &[], &mut session).result.expect("Oak HTML reopen");
    let view = HtmlDocumentView::from_document(&document);
    for selector in ["li #node-1 > p", "li #node-2 > pre > code", "li #node-4 > blockquote", "li #node-5 > ol"] {
        let (_, matches) = select_css_elements(&view, selector, Default::default()).expect("child selector");
        assert_eq!(matches.len(), 1, "{selector}: {html}");
    }
    assert_eq!(html.matches("child paragraph").count(), 1);
    assert_eq!(html.matches("child quote").count(), 1);
    assert!(html.contains("value &lt; limit"));
    assert!(html.contains(r##"href="#node-5""##));
}

#[test]
fn html_export_rejects_list_containment_cycles() {
    let mut graph = DocumentGraph::new(IdAllocator::default().document_id());
    graph.push_block(Block::List {
        ordered: false,
        items: vec![document_ir::ListItem { content: Vec::new(), children: vec![document_ir::NodeId(1)], number: None, ..Default::default() }],
    });
    let error = export_html(&graph).expect_err("containment cycles must not recurse");
    assert!(error.to_string().contains("invalid document graph"));
}

fn diagram_graph(source: &str) -> (DocumentGraph, NodeId) {
    let mut graph = DocumentGraph::new(IdAllocator::default().document_id());
    let id = graph.push_block(Block::Diagram { language: "mermaid".into(), source: source.into() });
    (graph, id)
}

#[test]
fn html_export_embeds_rendered_diagram_svg_in_a_figure() {
    let (graph, id) = diagram_graph("flowchart TD\n  A[Start] --> B[Stop]");
    let mut diagrams = HashMap::new();
    diagrams.insert(id, RenderedDiagram { svg: r##"<svg viewBox="0 0 10 10"><rect width="10" height="10"/></svg>"##.to_string() });

    let html = export_html_with_diagrams(&graph, &diagrams).expect("html export with rendered diagram");
    assert!(html.contains(r#"<figure class="diagram" data-diagram-language="mermaid""#), "{html}");
    assert!(html.contains(r##"<svg viewBox="0 0 10 10"><rect width="10" height="10"/></svg>"##), "{html}");
    assert!(html.contains("data-diagram-source-hex="), "{html}");
    assert!(!html.contains(r#"<pre class="diagram" data-diagram-language"#), "rendered diagrams must not keep the source fallback: {html}");
}

#[test]
fn html_export_falls_back_to_source_pre_without_a_rendered_scene() {
    let (graph, _) = diagram_graph("flowchart TD\n  A --> B");
    let html = export_html(&graph).expect("html export without rendered diagrams");
    assert!(html.contains(r#"<pre class="diagram" data-diagram-language="mermaid""#), "{html}");
    assert!(!html.contains("<figure"), "{html}");

    let (graph, _) = diagram_graph("flowchart TD\n  A --> B");
    let html = export_html_with_diagrams(&graph, &HashMap::new()).expect("html export with an empty diagram map");
    assert!(html.contains(r#"<pre class="diagram" data-diagram-language="mermaid""#), "{html}");
}

#[test]
fn html_export_with_diagrams_is_byte_stable() {
    let (graph, id) = diagram_graph("flowchart LR\n  A --> B");
    let mut diagrams = HashMap::new();
    diagrams.insert(id, RenderedDiagram { svg: "<svg><text>stable</text></svg>".to_string() });
    let first = export_html_with_diagrams(&graph, &diagrams).expect("first export");
    let second = export_html_with_diagrams(&graph, &diagrams).expect("second export");
    assert_eq!(first, second);
}
#[test]
fn html_export_report_names_degraded_and_unresolved_semantics() {
    let mut graph = DocumentGraph::new(IdAllocator::default().document_id());
    graph.push_block(Block::Paragraph {
        content: vec![
            Inline::Citation { key: "doe2020".into(), display: None },
            Inline::Link {
                target: RelationEndpoint::Unresolved { label: "missing".into() },
                display: vec![Inline::Text { text: "see missing".into() }],
                title: None,
            },
            Inline::Image { source: "assets/unresolved.png".into(), alt: Vec::new(), title: None, asset: None },
        ],
    });
    graph.push_block(Block::Opaque { kind: "tex.newcommand".into(), payload_hint: "\\foo".into(), status: SemanticStatus::Partial });

    let export = export_html_report(&graph).expect("html export report");
    let codes: Vec<&str> = export.losses.iter().map(|loss| loss.code.as_str()).collect();
    for expected in
        ["export.html.citation_degraded", "export.html.unresolved_link", "export.html.image_asset_unresolved", "export.html.opaque_projection"]
    {
        assert!(codes.contains(&expected), "{expected} missing from {codes:?}");
    }
    assert_eq!(export.losses.len(), 4, "{codes:?}");
    assert!(export.html.contains(r#"<span class="unresolved-link">see missing</span>"#), "{}", export.html);
    assert!(export.html.contains(r#"<span class="citation" data-citation-key="doe2020">[doe2020]</span>"#), "{}", export.html);
    assert!(export.html.contains(r#"<div class="opaque" data-opaque-kind="tex.newcommand">\foo</div>"#), "{}", export.html);

    // The degraded markup is still well-formed for the HTML importer, so a
    // reopen keeps the display text reachable instead of losing the construct.
    let source = SourceText::new(export.html.as_str());
    let builder = HtmlBuilder::new(HtmlLanguage::default());
    let mut session = ParseSession::<HtmlLanguage>::default();
    let document = builder.build(&source, &[], &mut session).result.expect("Oak HTML reopen");
    let view = HtmlDocumentView::from_document(&document);
    let (_, unresolved) = select_css_elements(&view, "span.unresolved-link", Default::default()).expect("unresolved link selector");
    let (_, citations) = select_css_elements(&view, "span.citation", Default::default()).expect("citation selector");
    assert_eq!(unresolved.len(), 1);
    assert_eq!(citations.len(), 1);
}

#[test]
fn html_export_report_keeps_resolved_asset_out_of_loss_list() {
    let mut graph = DocumentGraph::new(IdAllocator::default().document_id());
    let asset_id = graph.push_asset(Asset {
        id: AssetId(1),
        kind: AssetKind::Image,
        content_identity: Some("blake3-hash".into()),
        source: Some("assets/diagram.png".into()),
        media_type: Some("image/png".into()),
        status: SemanticStatus::Resolved,
        bytes: None,
    });
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Image {
            source: "assets/diagram.png".into(),
            alt: vec![Inline::Text { text: "diagram".into() }],
            title: None,
            asset: Some(asset_id),
        }],
    });

    let export = export_html_report(&graph).expect("html export report");
    assert!(export.losses.is_empty(), "{:?}", export.losses.iter().map(|loss| loss.code.as_str()).collect::<Vec<_>>());
    assert_eq!(export.html, export_html(&graph).expect("html export"));
}
#[test]
fn html_export_keeps_table_caption_and_ordered_list_numbers() {
    let mut graph = DocumentGraph::new(DocumentId(70));
    let first_cell = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "first".into() }] });
    graph.push_block(Block::Table {
        caption: Some("首表".into()),
        head: vec![TableRow { cells: vec![TableCell { children: vec![first_cell], colspan: 1, rowspan: 1 }] }],
        body: Vec::new(),
        foot: Vec::new(),
    });
    let nested = graph.push_block(Block::List {
        ordered: true,
        items: vec![
            ListItem { content: vec![Inline::Text { text: "three".into() }], children: Vec::new(), number: Some(3), ..Default::default() },
            ListItem { content: vec![Inline::Text { text: "five".into() }], children: Vec::new(), number: Some(5), ..Default::default() },
        ],
    });
    graph.push_block(Block::List {
        ordered: true,
        items: vec![
            ListItem { content: vec![Inline::Text { text: "five".into() }], children: vec![nested], number: Some(5), ..Default::default() },
            ListItem { content: vec![Inline::Text { text: "seven".into() }], children: Vec::new(), number: Some(7), ..Default::default() },
        ],
    });
    let last_cell = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "last".into() }] });
    graph.push_block(Block::Table {
        caption: Some("尾表".into()),
        head: vec![TableRow { cells: vec![TableCell { children: vec![last_cell], colspan: 1, rowspan: 1 }] }],
        body: Vec::new(),
        foot: Vec::new(),
    });
    graph.push_block(Block::List {
        ordered: false,
        items: vec![ListItem { content: vec![Inline::Text { text: "plain".into() }], ..Default::default() }],
    });

    let html = export_html(&graph).expect("html export");
    assert!(html.contains("<caption>首表</caption>"), "{html}");
    assert!(html.contains("<caption>尾表</caption>"), "{html}");
    assert!(html.contains(r#"<ol start="5">"#), "{html}");
    assert!(html.contains(r#"<ol start="3">"#), "{html}");
    assert!(html.contains(r#"<li value="7">"#), "{html}");
    assert!(html.contains(r#"<li value="5">"#), "{html}");
    assert!(!html.contains(r#"<ul start"#), "an unordered list carries no start: {html}");
}

mod asset_materialization;
mod import;
