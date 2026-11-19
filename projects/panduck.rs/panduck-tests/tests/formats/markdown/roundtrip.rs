use document_formats::{export::markdown::export_markdown, import::markdown::import_markdown_bytes};
use document_ir::{Block, DocumentGraph, Inline, RelationEndpoint, SemanticStatus};

fn blocks(graph: &DocumentGraph) -> Vec<Block> {
    graph.blocks.iter().map(|node| node.block.clone()).collect()
}

fn cell_inlines<'a>(graph: &'a DocumentGraph, cell: &document_ir::TableCell) -> &'a [Inline] {
    let Some(child) = cell.children.first().and_then(|id| graph.block(*id))
    else {
        return &[];
    };
    match &child.block {
        Block::Paragraph { content } => content,
        _ => &[],
    }
}

fn round_trip(name: &str, source: &str) -> (DocumentGraph, DocumentGraph, String) {
    let graph = import_markdown_bytes(&format!("{name}.md"), source).expect("import markdown");
    let markdown = export_markdown(&graph).expect("export markdown");
    let reopened = import_markdown_bytes(&format!("{name}-reopen.md"), &markdown).expect("reopen markdown");
    (graph, reopened, markdown)
}

/// Fixtures whose full block payload must survive Markdown import, export, and
/// re-import without diagnostics.
///
/// A fixture that references an image still reports
/// `import.markdown.asset_unresolved` because Markdown carries no media bytes
/// and no asset source is supplied here. That loss is reported, not silent, so
/// it is the only one these fixtures may raise.
const LOSSLESS_FIXTURES: &[(&str, &str)] = &[
    ("fence-language", "```rust title=\"sample\"\nfn main() {}\n```\n"),
    ("fence-long-inner-backticks", "````markdown\n```inner\ncode\n```\n````\n"),
    ("fence-tilde", "~~~python\nprint(1)\n~~~\n"),
    ("fence-indented-body", "```mermaid\n  A-->B\n    B-->C\n```\n"),
    ("diagram-empty", "```mermaid\n```\n"),
    ("diagram-long", "```mermaid\nflowchart TD\n  A[Alpha] --> B[Beta]\n  B --> C[Gamma]\n  C --> D[Delta]\n```\n"),
    ("table-inline-cells", "| **A** | [x](https://e.com) |\n| --- | --- |\n| ![i](i.png) | y |\n"),
    ("table-escaped-pipes", "| A | B |\n| --- | --- |\n| a \\| b | `c|d` |\n"),
    ("link-title", "[a](https://example.com \"t\")\n"),
    ("image-title", "![alt](i.png \"t\")\n"),
    ("reference-title", "[a][r]\n\n[r]: https://example.com \"t\"\n"),
    ("footnote-continuation", "See[^1].\n\n[^1]: first\n    second\n"),
    ("quote-multi-paragraph", "> para one\n>\n> para two\n"),
    ("hard-break", "line one  \nline two\n"),
    ("ordered-list", "1. one\n2. two\n"),
];

#[test]
fn markdown_round_trip_fixture_set_keeps_block_payloads_and_reports_no_unexpected_loss() {
    for (name, source) in LOSSLESS_FIXTURES {
        let (graph, reopened, markdown) = round_trip(name, source);
        let unexpected = |graph: &DocumentGraph| {
            graph
                .coverage
                .loss
                .iter()
                .filter(|loss| loss.code != "import.markdown.asset_unresolved")
                .map(|loss| loss.code.clone())
                .collect::<Vec<_>>()
        };
        assert!(unexpected(&graph).is_empty(), "{name} imported lossy: {:#?}\n{markdown}", unexpected(&graph));
        assert!(unexpected(&reopened).is_empty(), "{name} reopened lossy: {:#?}\n{markdown}", unexpected(&reopened));
        assert_eq!(blocks(&graph), blocks(&reopened), "{name} changed shape\n{markdown}");
    }
}

#[test]
fn markdown_fences_keep_language_and_source_across_long_and_indented_fences() {
    let source = "````markdown\n```inner\n  code\n```\n````\n";
    let (graph, reopened, markdown) = round_trip("fence-long", source);
    assert_eq!(graph.blocks.len(), 1, "{markdown}");
    assert_eq!(
        graph.blocks[0].block,
        Block::Code { language: Some("markdown".into()), content: "```inner\n  code\n```".into() },
        "a closing run shorter than the opening run must not split the fence\n{markdown}"
    );
    assert_eq!(blocks(&reopened), blocks(&graph));

    let source = "```mermaid\n  A-->B\n    B-->C\n```\n";
    let (graph, reopened, markdown) = round_trip("fence-indented", source);
    assert_eq!(graph.blocks[0].block, Block::Diagram { language: "mermaid".into(), source: "  A-->B\n    B-->C".into() }, "{markdown}");
    assert_eq!(blocks(&reopened), blocks(&graph));
}

#[test]
fn markdown_table_cells_keep_inline_shapes_and_escaped_pipes() {
    let source = "| **A** | [x](https://e.com) |\n| --- | --- |\n| `c|d` | a \\| b |\n";
    let (graph, reopened, markdown) = round_trip("table-inline", source);
    let (head, body) = graph
        .blocks
        .iter()
        .find_map(|node| match &node.block {
            Block::Table { head, body, .. } => Some((head.as_slice(), body.as_slice())),
            _ => None,
        })
        .unwrap_or_else(|| panic!("table expected: {:#?}", graph.blocks));
    assert!(matches!(&cell_inlines(&graph, &head[0].cells[0])[0], Inline::Styled { style, .. } if style == "bold"), "{markdown}");
    assert!(
        matches!(&cell_inlines(&graph, &head[0].cells[1])[0], Inline::Link { target, .. } if *target == RelationEndpoint::ExternalUri("https://e.com".into()))
    );
    assert_eq!(cell_inlines(&graph, &body[0].cells[0]), [Inline::InlineCode { text: "c|d".into() }], "{markdown}");
    assert_eq!(cell_inlines(&graph, &body[0].cells[1]), [Inline::Text { text: "a | b".into() }], "{markdown}");
    assert_eq!(blocks(&reopened), blocks(&graph), "{markdown}");
}

#[test]
fn markdown_link_and_image_titles_round_trip_instead_of_folding_into_the_url() {
    let (graph, reopened, markdown) = round_trip("link-title", "[a](https://example.com \"t\")\n");
    assert_eq!(
        graph.blocks[0].block,
        Block::Paragraph {
            content: vec![Inline::Link {
                target: RelationEndpoint::ExternalUri("https://example.com".into()),
                display: vec![Inline::Text { text: "a".into() }],
                title: Some("t".into()),
            }]
        },
        "{markdown}"
    );
    assert_eq!(blocks(&reopened), blocks(&graph));

    let (graph, reopened, markdown) = round_trip("image-title", "![alt](i.png \"t\")\n");
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("paragraph expected: {:#?}", graph.blocks)
    };
    let Inline::Image { source, title, .. } = &content[0]
    else {
        panic!("image expected: {content:#?}")
    };
    assert_eq!(source.as_str(), "i.png", "{markdown}");
    assert_eq!(title.as_deref(), Some("t"), "{markdown}");
    assert_eq!(blocks(&reopened), blocks(&graph));
}

#[test]
fn markdown_footnote_continuations_stay_inside_the_definition() {
    let source = "See[^1].\n\n[^1]: first\n    second\n";
    let (graph, reopened, markdown) = round_trip("footnote-continuation", source);
    assert_eq!(
        blocks(&graph),
        vec![
            Block::Paragraph {
                content: vec![
                    Inline::Text { text: "See".into() },
                    Inline::Note { label: "1".into(), display: None },
                    Inline::Text { text: ".".into() },
                ],
            },
            Block::Opaque {
                kind: "footnote_definition".into(),
                payload_hint: "[^1]: first\n    second".into(),
                status: SemanticStatus::Resolved,
            },
        ],
        "{markdown}"
    );
    assert_eq!(blocks(&reopened), blocks(&graph), "{markdown}");
}

#[test]
fn markdown_table_alignment_is_reported_as_partial_loss_without_changing_cells() {
    let source = "| Left | Center | Right |\n| :--- | :---: | ---: |\n| a | b | c |\n";
    let (graph, reopened, markdown) = round_trip("table-alignment", source);
    let loss = graph.coverage.loss.iter().find(|loss| loss.code == "import.markdown.table_alignment");
    let loss = loss.unwrap_or_else(|| panic!("alignment loss expected: {:#?}", graph.coverage.loss));
    assert_eq!(loss.status, SemanticStatus::Partial);
    assert!(markdown.contains("| --- | --- | --- |"), "{markdown}");
    assert_eq!(blocks(&reopened), blocks(&graph), "{markdown}");
}

#[test]
fn markdown_paragraph_trailing_line_break_is_not_inline_content() {
    let source = "Alpha.\n";
    let (graph, reopened, markdown) = round_trip("paragraph-terminator", source);
    assert_eq!(graph.blocks[0].block, Block::Paragraph { content: vec![Inline::Text { text: "Alpha.".into() }] }, "{markdown}");
    assert_eq!(blocks(&reopened), blocks(&graph), "{markdown}");
}
