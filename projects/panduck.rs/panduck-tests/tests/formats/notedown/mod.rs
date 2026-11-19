use document_formats::{
    export::markdown::export_markdown,
    import::{markdown::import_markdown_bytes, notedown::import_notedown_bytes},
};
use document_ir::{Block, Inline, RelationEndpoint, SemanticStatus};

#[test]
fn notedown_import_lowers_heading_and_paragraph() {
    let source = "# Title\n\nHello world.\n";
    let graph = import_notedown_bytes("sample.nd", source).expect("import notedown");
    assert!(graph.blocks.len() >= 2);
    assert!(matches!(
        &graph.blocks[0].block,
        Block::Section { title, .. } if title == &[Inline::Text { text: "Title".into() }]
    ));
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("# Title"));
    assert!(markdown.contains("Hello world"));
}

#[test]
fn notedown_import_lowers_strong_and_emphasis_without_leaking_markers() {
    let graph = import_notedown_bytes("styles.nd", "Body **bold** and *italic*.\n").expect("import notedown");
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("expected paragraph: {:?}", graph.blocks);
    };
    assert!(
        content.iter().any(|inline| matches!(
            inline,
            Inline::Styled { style, children }
                if style == "bold" && children == &[Inline::Text { text: "bold".into() }]
        )),
        "{content:?}"
    );
    assert!(content.iter().any(|inline| matches!(
        inline,
        Inline::Styled { style, children }
            if style == "italic" && children == &[Inline::Text { text: "italic".into() }]
    )));
    assert!(!content.iter().any(|inline| matches!(inline, Inline::Text { text } if text.contains("**") || text.contains("*"))));
}

#[test]
fn notedown_import_lowers_nested_emphasis() {
    let graph = import_notedown_bytes("nested-styles.nd", "**bold *italic***\n").expect("import notedown");
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("expected paragraph: {:?}", graph.blocks);
    };
    assert!(
        matches!(
            content.first(),
            Some(Inline::Styled { style, children })
                if style == "bold" && children.iter().any(|child| matches!(
                    child,
                    Inline::Styled { style, children }
                        if style == "italic" && children == &[Inline::Text { text: "italic".into() }]
                ))
        ),
        "{content:?}"
    );
}

#[test]
fn notedown_import_preserves_separated_nested_emphasis() {
    let graph = import_notedown_bytes("nested-styles.nd", "**bold *italic* end**\n").expect("import notedown");
    let html = document_formats::export::html::export_html(&graph).expect("export HTML");
    assert!(html.contains("<strong>bold <em>italic</em> end</strong>"), "{html}");
}

#[test]
fn notedown_import_lowers_underscore_emphasis() {
    let graph = import_notedown_bytes("underscore.nd", "__bold__ and _italic_.\n").expect("import notedown");
    let html = document_formats::export::html::export_html(&graph).expect("export HTML");
    assert!(html.contains("<strong>bold</strong> and <em>italic</em>."), "{html}");
}

#[test]
fn notedown_table_cells_preserve_emphasis() {
    let graph =
        import_notedown_bytes("styled-table.nd", "| Name | Value |\n|------|-------|\n| **Alpha** | *one* |\n").expect("import notedown");
    let (head, body) = graph
        .blocks
        .iter()
        .find_map(|node| match &node.block {
            Block::Table { head, body, .. } => Some((head.as_slice(), body.as_slice())),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected table: {:?}", graph.blocks));
    assert_eq!(head.len(), 1);
    let cell_inlines = |cell: &document_ir::TableCell| {
        cell.children
            .iter()
            .filter_map(|id| graph.block(*id))
            .flat_map(|node| match &node.block {
                Block::Paragraph { content } => content.clone(),
                _ => Vec::new(),
            })
            .collect::<Vec<_>>()
    };
    assert!(
        cell_inlines(&body[0].cells[0]).iter().any(|inline| matches!(
            inline,
            Inline::Styled { style, children }
                if style == "bold" && children == &[Inline::Text { text: "Alpha".into() }]
        )),
        "{:?}",
        body[0].cells[0]
    );
    assert!(
        cell_inlines(&body[0].cells[1]).iter().any(|inline| matches!(
            inline,
            Inline::Styled { style, children }
                if style == "italic" && children == &[Inline::Text { text: "one".into() }]
        )),
        "{:?}",
        body[0].cells[1]
    );
}

#[test]
fn notedown_import_lowers_list_items() {
    let source = "- Alpha\n- Beta\n";
    let graph = import_notedown_bytes("list.nd", source).expect("import notedown");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("- Alpha"));
    assert!(markdown.contains("- Beta"));
}

#[test]
fn notedown_import_lowers_fenced_code_block() {
    let source = "```rust\nfn main() {}\n```\n";
    let graph = import_notedown_bytes("code.nd", source).expect("import notedown");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("```rust"));
    assert!(markdown.contains("fn main() {}"));
}

#[test]
fn notedown_import_lowers_blockquote() {
    let source = "> Quoted line\n";
    let graph = import_notedown_bytes("quote.nd", source).expect("import notedown");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("> Quoted line"));
}

#[test]
fn notedown_import_lowers_multiline_blockquote() {
    let source = "> Line one\n> Line two\n";
    let graph = import_notedown_bytes("quote.nd", source).expect("import notedown");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("Line one"));
    assert!(markdown.contains("Line two"));
    assert!(markdown.contains('>'));
}

#[test]
fn notedown_import_lowers_horizontal_rule() {
    let source = "Before\n\n---\n\nAfter\n";
    let graph = import_notedown_bytes("hr.nd", source).expect("import notedown");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("Before"));
    assert!(markdown.contains("---"));
    assert!(markdown.contains("After"));
}

#[test]
fn notedown_import_lowers_links() {
    let source = "Visit [home](https://example.com/a(b)) today.\n";
    let graph = import_notedown_bytes("link.nd", source).expect("import notedown");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains(r"[home](https://example.com/a\(b\))"), "{markdown:?}, {graph:?}");
    let reopened = import_markdown_bytes("link.md", &markdown).expect("re-import markdown");
    assert!(markdown.contains(" today."), "trailing paragraph content was lost: {markdown:?}, {graph:?}");
    let Block::Paragraph { content } = &reopened.blocks[0].block
    else {
        panic!("expected paragraph: {:?}", reopened.blocks);
    };
    let link = content
        .iter()
        .find_map(|inline| match inline {
            Inline::Link { target, display, .. } => Some((target, display)),
            _ => None,
        })
        .expect("link must survive markdown reopen as a first-class inline");
    assert_eq!(link.0, &RelationEndpoint::ExternalUri("https://example.com/a(b)".into()));
    assert_eq!(link.1, &vec![Inline::Text { text: "home".into() }]);
    assert!(
        graph.blocks.iter().any(|node| matches!(
            &node.block,
            Block::Paragraph { content } if content.iter().any(|inline| matches!(inline, Inline::Link { .. }))
        )),
        "the notedown author surface must lower links to first-class inlines: {graph:?}"
    );
}

#[test]
fn notedown_import_lowers_images_as_semantic_inline_images() {
    let source = "![diagram](assets/diagram.png)\n";
    let graph = import_notedown_bytes("image.nd", source).expect("import notedown");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("![diagram](assets/diagram.png)"), "{markdown:?}, {graph:?}");
    assert_eq!(graph.assets.len(), 1);
    assert_eq!(graph.assets[0].source.as_deref(), Some("assets/diagram.png"));
    assert_eq!(graph.assets[0].media_type.as_deref(), Some("image/png"));
    assert_eq!(graph.relations.len(), 1);
    let authored = graph.blocks[0].block.clone();
    let Block::Paragraph { content } = &authored
    else {
        panic!("expected paragraph: {authored:?}");
    };
    assert_eq!(
        content.first(),
        Some(&Inline::Image {
            source: "assets/diagram.png".into(),
            alt: vec![Inline::Text { text: "diagram".into() }],
            title: None,
            asset: Some(graph.assets[0].id),
        })
    );
    let reopened = import_markdown_bytes("image.md", &markdown).expect("re-import markdown");
    assert!(matches!(
        &reopened.blocks[0].block,
        Block::Paragraph { content }
            if content.iter().any(|inline| matches!(
                inline,
                Inline::Image { source, alt, asset: Some(_), .. }
                    if source == "assets/diagram.png"
                        && alt == &[Inline::Text { text: "diagram".into() }]
            ))
    ));
    assert_eq!(reopened.assets.len(), 1);
    assert_eq!(reopened.assets[0].source.as_deref(), Some("assets/diagram.png"));
    assert_eq!(reopened.relations.len(), 1);
}

#[test]
fn notedown_inline_forms_never_lower_to_an_unnamed_span_style() {
    let source = "**bold** *italic* ~~old~~ `code` [home](https://example.com) ![diagram](assets/diagram.png)\n";
    let graph = import_notedown_bytes("inline-forms.nd", source).expect("import notedown");
    let styles: Vec<&str> = graph
        .blocks
        .iter()
        .filter_map(|node| match &node.block {
            Block::Paragraph { content } => Some(
                content
                    .iter()
                    .filter_map(|inline| match inline {
                        Inline::Styled { style, .. } => Some(style.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .flatten()
        .collect();
    assert_eq!(styles, ["bold", "italic", "strike"], "syntax names must never leak as semantic styles: {styles:?}");
    assert!(!styles.contains(&"span"), "a grouping node must not become a style: {styles:?}");

    let markdown = export_markdown(&graph).expect("export markdown");
    for expected in ["**bold**", "*italic*", "~~old~~", "`code`", "[home](https://example.com)", "![diagram](assets/diagram.png)"] {
        assert!(markdown.contains(expected), "{expected} missing from {markdown:?}");
    }
}

#[test]
fn notedown_import_lowers_pipe_tables() {
    let source = "| Name | Value |\n|------|-------|\n| Alpha | 1 |\n| Beta | 2 |\n";
    let graph = import_notedown_bytes("table.nd", source).expect("import notedown");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("| Name | Value |"));
    assert!(markdown.contains("| Alpha | 1 |"));
    assert!(markdown.contains("| Beta | 2 |"));
    assert!(!markdown.contains("| ------ |"), "GFM separator row should not round-trip as data: {markdown}");
}

#[test]
fn notedown_import_reports_partial_coverage_loss() {
    let graph = import_notedown_bytes("coverage.nd", "# Title\n\nBody.\n").expect("import notedown");
    assert!(!graph.coverage.complete);
    let codes = loss_codes(&graph);
    assert!(codes.contains(&"import.notedown.partial_coverage"), "{codes:?}");
}

#[test]
fn notedown_import_reports_empty_document_loss() {
    let graph = import_notedown_bytes("empty.nd", "   \n").expect("import notedown");
    assert!(graph.blocks.is_empty(), "{:?}", graph.blocks);
    let codes = loss_codes(&graph);
    assert!(codes.contains(&"import.notedown.empty_document"), "{codes:?}");
}

#[test]
fn notedown_import_reports_task_list_as_partial() {
    let source = "- [ ] todo\n- [x] done\n";
    let graph = import_notedown_bytes("tasks.nd", source).expect("import notedown");
    assert!(
        graph.blocks.iter().any(|node| matches!(
            &node.block,
            Block::List { ordered: false, items } if items.len() == 2
        )),
        "{:?}",
        graph.blocks
    );
    let codes = loss_codes(&graph);
    assert!(codes.contains(&"import.notedown.partial_coverage"), "{codes:?}");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("todo"), "{markdown:?}");
    assert!(markdown.contains("done"), "{markdown:?}");
}

#[test]
fn notedown_round_trip_preserves_supported_semantics() {
    let source = "# Title\n\nBody **bold** and *italic*.\n\n- Alpha\n- Beta\n\n> Quoted\n\n```rust\nfn main() {}\n```\n\n| Name | Value |\n|------|-------|\n| Alpha | 1 |\n\n[home](https://example.com)\n";
    let graph = import_notedown_bytes("roundtrip.nd", source).expect("import notedown");
    let markdown = export_markdown(&graph).expect("export markdown");
    let reopened = import_markdown_bytes("roundtrip.md", &markdown).expect("re-import markdown");

    assert!(
        reopened.blocks.iter().any(|node| matches!(
            &node.block,
            Block::Section { level: 1, title, .. }
                if title == &[Inline::Text { text: "Title".into() }]
        )),
        "{:?}",
        reopened.blocks
    );
    assert!(
        reopened.blocks.iter().any(|node| matches!(
            &node.block,
            Block::Paragraph { content }
                if content.iter().any(|inline| matches!(inline, Inline::Styled { style, .. } if style == "bold"))
        )),
        "{:?}",
        reopened.blocks
    );
    assert!(
        reopened.blocks.iter().any(|node| matches!(
            &node.block,
            Block::List { ordered: false, items } if items.len() == 2
        )),
        "{:?}",
        reopened.blocks
    );
    assert!(
        reopened.blocks.iter().any(|node| matches!(
            &node.block,
            Block::Quote { children }
                if children.iter().any(|child_id| matches!(
                    reopened.block(*child_id).map(|child| &child.block),
                    Some(Block::Paragraph { content })
                        if content.iter().any(|inline| matches!(inline, Inline::Text { text } if text.contains("Quoted")))
                ))
        )),
        "{:?}",
        reopened.blocks
    );
    assert!(
        reopened.blocks.iter().any(|node| matches!(
            &node.block,
            Block::Code { language: Some(language), content } if language == "rust" && content.contains("fn main")
        )),
        "{:?}",
        reopened.blocks
    );
    assert!(
        reopened.blocks.iter().any(|node| matches!(
            &node.block,
            Block::Table { head, body, .. } if head.len() == 1 && body.len() == 1
        )),
        "{:?}",
        reopened.blocks
    );
    assert!(
        reopened.blocks.iter().any(|node| matches!(
            &node.block,
            Block::Paragraph { content }
                if content.iter().any(|inline| matches!(inline, Inline::Link { .. }))
        )),
        "{:?}",
        reopened.blocks
    );
}

#[test]
fn notedown_import_reports_unresolved_target_with_provenance() {
    let source = "See [missing note] for context.\n";
    let graph = import_notedown_bytes("unresolved.nd", source).expect("import notedown");
    let index = graph.coverage.loss.iter().position(|loss| loss.code == "import.notedown.unresolved_target").expect("unresolved target loss");
    assert_eq!(graph.coverage.loss[index].status, SemanticStatus::Unresolved);
    let span = graph.loss_span(index).expect("loss span");
    let authored = &source[span.start as usize..span.end as usize];
    assert!(authored.contains("[missing note]"), "{authored:?}");
    assert!(
        graph.blocks.iter().any(|node| matches!(
            &node.block,
            Block::Paragraph { content }
                if content.iter().any(|inline| matches!(
                    inline,
                    Inline::Link { target: RelationEndpoint::Unresolved { label }, display, .. }
                        if label == "missing note"
                            && display == &[Inline::Text { text: "missing note".into() }]
                ))
        )),
        "{:?}",
        graph.blocks
    );
}

fn loss_codes(graph: &document_ir::DocumentGraph) -> Vec<&str> {
    graph.coverage.loss.iter().map(|loss| loss.code.as_str()).collect()
}
