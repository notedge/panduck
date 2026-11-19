use document_formats::{
    export::{
        html::export_html,
        markdown::{export_markdown, export_markdown_report},
    },
    import::{
        html::{HtmlAssetSource, import_html_text},
        markdown::{import_markdown_bytes, import_markdown_bytes_with_assets},
    },
};
use document_ir::{
    AssetKind, Block, DocumentGraph, DocumentId, Inline, MathExpression, RelationEndpoint, RelationKind, SemanticStatus, TableCell, TableRow,
};

struct Assets(std::collections::HashMap<String, Vec<u8>>);

impl HtmlAssetSource for Assets {
    fn read_asset(&self, target: &str) -> Option<Vec<u8>> {
        self.0.get(target).cloned()
    }
}

fn png_bytes() -> Vec<u8> {
    vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]
}

fn table<'a>(graph: &'a DocumentGraph) -> (&'a [TableRow], &'a [TableRow]) {
    graph
        .blocks
        .iter()
        .find_map(|node| match &node.block {
            Block::Table { head, body, .. } => Some((head.as_slice(), body.as_slice())),
            _ => None,
        })
        .unwrap_or_else(|| panic!("table expected: {:#?}", graph.blocks))
}

fn cell_text(graph: &DocumentGraph, cell: &TableCell) -> String {
    cell.children
        .iter()
        .filter_map(|id| graph.block(*id))
        .flat_map(|node| match &node.block {
            Block::Paragraph { content } => content.iter().map(Inline::plain_text).collect::<Vec<_>>(),
            _ => Vec::new(),
        })
        .collect()
}

#[test]
fn markdown_export_keeps_html_image_width_and_height() {
    let graph = import_html_text("img-size.html", r#"<body><img src="chart.png" alt="chart" width="320" height="180"></body>"#)
        .expect("import html image size");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(
        markdown.contains(r#"<img src="chart.png" alt="chart" width="320" height="180">"#),
        "an image size needs the `<img>` element attributes: {markdown}"
    );
    assert!(!markdown.contains("<span width"), "a span width is not an image size: {markdown}");
}

#[test]
fn markdown_export_of_an_unsized_image_stays_markdown() {
    let graph = import_html_text("img-plain.html", r#"<body><img src="chart.png" alt="chart"></body>"#).expect("import html image");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("![chart](chart.png)"), "a plain image stays markdown: {markdown}");
    assert!(!markdown.contains("<img"), "no size means no inline HTML: {markdown}");
}

#[test]
fn markdown_intraword_underscores_stay_literal_in_every_position() {
    let source = "# snake_case_name\n\nsnake_case_name\n\n> snake_case_name\n\n- snake_case_name\n\n\
                  | head |\n| --- |\n| snake_case_name |\n\n**snake_case_name**\n";
    let graph = import_markdown_bytes("intraword.md", source).expect("import intraword underscores");
    assert_eq!(count_intraword(&graph), 6, "all six positions must keep the word: {:#?}", graph.blocks);

    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(!markdown.contains("snake*case*name"), "the export must not rewrite the word as emphasis: {markdown}");
    assert!(!markdown.contains("\\_"), "an intraword underscore needs no escape: {markdown}");
    assert_eq!(markdown.matches("snake_case_name").count(), 6, "{markdown}");

    let reopened = import_markdown_bytes("intraword-roundtrip.md", &markdown).expect("reopen markdown");
    assert_eq!(count_intraword(&reopened), 6, "the round trip must keep the word: {markdown}");
}

fn count_intraword(graph: &DocumentGraph) -> usize {
    fn push_inlines(inlines: &[Inline], out: &mut String) {
        for inline in inlines {
            match inline {
                Inline::Text { text } | Inline::InlineCode { text } => out.push_str(text),
                Inline::Styled { children, .. } | Inline::Attributed { children, .. } => push_inlines(children, out),
                Inline::Link { display, .. } => push_inlines(display, out),
                Inline::Image { alt, .. } => push_inlines(alt, out),
                Inline::Note { display: Some(display), .. } | Inline::Citation { display: Some(display), .. } => {
                    push_inlines(display, out);
                }
                _ => {}
            }
        }
    }

    let mut text = String::new();
    for node in &graph.blocks {
        match &node.block {
            Block::Section { title, .. } => push_inlines(title, &mut text),
            Block::Paragraph { content } => push_inlines(content, &mut text),
            Block::List { items, .. } => {
                for item in items {
                    push_inlines(&item.content, &mut text);
                }
            }
            _ => {}
        }
    }
    text.matches("snake_case_name").count()
}

#[test]
fn markdown_stray_angle_bracket_stays_text_and_is_escaped_on_export() {
    let graph = import_markdown_bytes("angle.md", "Para with 1 < 2.\n").expect("import markdown");
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("paragraph expected: {:#?}", graph.blocks)
    };
    assert!(matches!(content.first(), Some(Inline::Text { text }) if text == "Para with 1 < 2."), "{content:?}");
    assert_eq!(export_markdown(&graph).expect("export markdown"), "Para with 1 \\< 2.\n\n");
    assert!(export_html(&graph).expect("export html").contains("1 &lt; 2"));
    let reopened = import_markdown_bytes("angle-roundtrip.md", "Para with 1 \\< 2.\n").expect("reopen markdown");
    let Block::Paragraph { content } = &reopened.blocks[0].block
    else {
        panic!("paragraph expected: {:#?}", reopened.blocks)
    };
    assert!(matches!(content.first(), Some(Inline::Text { text }) if text == "Para with 1 < 2."), "{content:?}");
}

#[test]
fn markdown_lists_lower_to_list_blocks() {
    let source = "- alpha\n- beta\n";
    let graph = import_markdown_bytes("list.md", source).expect("import markdown");
    let Block::List { ordered: false, items } = &graph.blocks[0].block
    else {
        panic!("unordered list expected: {:#?}", graph.blocks)
    };
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].content, vec![Inline::Text { text: "alpha".into() }]);
    assert_eq!(items[1].content, vec![Inline::Text { text: "beta".into() }]);
    assert!(items.iter().all(|item| item.children.is_empty()));
    assert!(graph.validate().is_valid());

    let ordered_source = "1. one\n2. two\n";
    let ordered_graph = import_markdown_bytes("ordered-list.md", ordered_source).expect("import ordered list");
    let Block::List { ordered: true, items } = &ordered_graph.blocks[0].block
    else {
        panic!("ordered list expected: {:#?}", ordered_graph.blocks)
    };
    assert_eq!(items.iter().map(|item| item.number).collect::<Vec<_>>(), vec![Some(1), Some(2)]);
    assert_eq!(items[0].content, vec![Inline::Text { text: "one".into() }]);
    assert!(ordered_graph.validate().is_valid());

    let nested_source = "- parent\n  - child\n";
    let nested_graph = import_markdown_bytes("nested-list.md", nested_source).expect("import nested list");
    let outer_items = nested_graph
        .blocks
        .iter()
        .find_map(|node| match &node.block {
            Block::List { items, .. } if items.iter().any(|item| !item.children.is_empty()) => Some(items),
            _ => None,
        })
        .expect("outer list with a nested child list");
    assert_eq!(outer_items.len(), 1);
    assert_eq!(outer_items[0].content, vec![Inline::Text { text: "parent".into() }]);
    assert_eq!(outer_items[0].children.len(), 1);
    let nested = nested_graph.block(outer_items[0].children[0]).expect("nested list block").block.clone();
    assert_eq!(
        nested,
        Block::List {
            ordered: false,
            items: vec![document_ir::ListItem {
                content: vec![Inline::Text { text: "child".into() }],
                children: Vec::new(),
                number: None,
                ..Default::default()
            }]
        }
    );
    assert!(nested_graph.validate().is_valid());

    let contained = nested_graph
        .blocks
        .iter()
        .flat_map(|node| match &node.block {
            Block::Section { children, .. } => children.clone(),
            Block::List { items, .. } => items.iter().flat_map(|item| item.children.clone()).collect::<Vec<_>>(),
            _ => Vec::new(),
        })
        .collect::<Vec<_>>();
    let root_blocks = nested_graph.blocks.iter().filter(|node| !contained.contains(&node.id)).collect::<Vec<_>>();
    assert_eq!(root_blocks.len(), 1, "only the outer list is a root block: {:#?}", nested_graph.blocks);
    assert!(matches!(&root_blocks[0].block, Block::List { ordered: false, .. }), "{:#?}", root_blocks[0].block);
    assert!(
        nested_graph.blocks.iter().all(|node| !matches!(&node.block, Block::Paragraph { content } if content.is_empty())),
        "empty paragraphs must never reach the graph: {:#?}",
        nested_graph.blocks
    );
}

#[test]
fn markdown_quote_nested_blocks_round_trip() {
    let source = "> - a\n> - b\n";
    let graph = import_markdown_bytes("quote-list.md", source).expect("import quoted list");
    let Block::Quote { children } = &graph.blocks.iter().find(|node| matches!(node.block, Block::Quote { .. })).expect("quote block").block
    else {
        unreachable!()
    };
    assert_eq!(children.len(), 1, "{children:#?}");
    assert!(
        matches!(graph.block(children[0]).map(|node| &node.block), Some(Block::List { ordered: false, .. })),
        "the quoted list is the only child: {children:#?}"
    );
    let markdown = export_markdown(&graph).expect("export quoted list");
    assert_eq!(markdown, "> - a\n> - b\n\n");

    let heading_source = "> # Title\n> - a\n";
    let heading_graph = import_markdown_bytes("quote-heading.md", heading_source).expect("import quoted heading");
    let heading_markdown = export_markdown(&heading_graph).expect("export quoted heading");
    assert_eq!(heading_markdown, "> # Title\n>\n> - a\n\n");

    let reopened = import_markdown_bytes("quote-heading-roundtrip.md", &heading_markdown).expect("reopen quoted heading");
    assert_eq!(export_markdown(&reopened).expect("re-export quoted heading"), heading_markdown);
}

#[test]
fn markdown_blockquote_lowers_inline_content() {
    let source = "> **bold** text\n";
    let graph = import_markdown_bytes("quote.md", source).expect("import markdown");
    let quote_node = graph.blocks.iter().find(|node| matches!(node.block, Block::Quote { .. })).expect("quote expected");
    let Block::Quote { children } = &quote_node.block
    else {
        unreachable!()
    };
    let content = children
        .iter()
        .find_map(|child_id| match graph.block(*child_id).map(|node| &node.block) {
            Some(Block::Paragraph { content }) => Some(content),
            _ => None,
        })
        .expect("quote body must be a paragraph child");
    assert!(
        content.iter().any(|inline| matches!(
            inline,
            Inline::Styled { style, children } if style == "bold" && children == &[Inline::Text { text: "bold".into() }]
        )),
        "{content:#?}"
    );
    assert_eq!(content.iter().map(Inline::plain_text).collect::<String>().trim(), "bold text");
    assert!(graph.validate().is_valid());

    let markdown = export_markdown(&graph).expect("export markdown");
    let reopened = import_markdown_bytes("quote-roundtrip.md", &markdown).expect("reopen markdown");
    assert_eq!(reopened.blocks, graph.blocks);

    let multiline = "> first line\n> second line\n";
    let multiline_graph = import_markdown_bytes("multiline-quote.md", multiline).expect("import multiline quote");
    let quote_node = multiline_graph.blocks.iter().find(|node| matches!(node.block, Block::Quote { .. })).expect("multiline quote expected");
    let Block::Quote { children } = &quote_node.block
    else {
        unreachable!()
    };
    let content = children
        .iter()
        .find_map(|child_id| match multiline_graph.block(*child_id).map(|node| &node.block) {
            Some(Block::Paragraph { content }) => Some(content),
            _ => None,
        })
        .expect("multiline quote body must be a paragraph child");
    assert!(content.iter().any(|inline| matches!(inline, Inline::Break { kind: document_ir::BreakKind::Soft })), "{content:#?}");
    assert_eq!(content.iter().map(Inline::plain_text).collect::<String>().trim(), "first line second line");
    let multiline_markdown = export_markdown(&multiline_graph).expect("export multiline quote");
    let multiline_reopened = import_markdown_bytes("multiline-quote-roundtrip.md", &multiline_markdown).expect("reopen multiline quote");
    assert_eq!(multiline_reopened.blocks, multiline_graph.blocks);
}

#[test]
fn markdown_fenced_code_keeps_content_and_language() {
    let source = "```rust\nfn main() {\n    let (x, y) = (1, 2);\n}\n```\n";
    let graph = import_markdown_bytes("code.md", source).expect("import markdown");
    let Block::Code { language, content } = &graph.blocks[0].block
    else {
        panic!("code block expected: {:#?}", graph.blocks)
    };
    assert_eq!(language.as_deref(), Some("rust"));
    assert_eq!(content, "fn main() {\n    let (x, y) = (1, 2);\n}");
    assert!(graph.validate().is_valid());

    let markdown = export_markdown(&graph).expect("export markdown");
    let reopened = import_markdown_bytes("code-roundtrip.md", &markdown).expect("reopen markdown");
    assert_eq!(reopened.blocks[0].block, graph.blocks[0].block);
}

#[test]
fn markdown_tables_lower_header_and_rows() {
    let source = "| A | B |\n| --- | --- |\n| 1 | 2 |\n";
    let graph = import_markdown_bytes("table.md", source).expect("import markdown");
    let (head, body) = table(&graph);
    assert_eq!(head.len(), 1);
    assert_eq!(body.len(), 1);
    assert_eq!(head[0].cells.len(), 2);
    assert_eq!(head[0].cells.iter().map(|cell| cell_text(&graph, cell)).collect::<Vec<_>>(), vec!["A".to_string(), "B".to_string()]);
    assert_eq!(body[0].cells.iter().map(|cell| cell_text(&graph, cell)).collect::<Vec<_>>(), vec!["1".to_string(), "2".to_string()]);
    assert!(head.iter().chain(body).flat_map(|row| row.cells.iter()).all(|cell| cell.colspan == 1 && cell.rowspan == 1));
    assert!(graph.validate().is_valid());

    let markdown = export_markdown(&graph).expect("export markdown");
    let reopened = import_markdown_bytes("table-roundtrip.md", &markdown).expect("reopen markdown");
    let (reopened_head, reopened_body) = table(&reopened);
    assert_eq!(reopened_head.len(), 1);
    assert_eq!(
        reopened_head
            .iter()
            .chain(reopened_body)
            .map(|row| row.cells.iter().map(|cell| cell_text(&reopened, cell)).collect::<Vec<_>>())
            .collect::<Vec<_>>(),
        vec![vec!["A".to_string(), "B".to_string()], vec!["1".to_string(), "2".to_string()]],
    );
    assert!(reopened.validate().is_valid());
}

#[test]
fn markdown_table_export_uses_three_hyphen_delimiters() {
    let source = "| A | B |\n| --- | --- |\n| 1 | 2 |\n";
    let graph = import_markdown_bytes("table-delimiter.md", source).expect("import markdown");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.starts_with("| A | B |\n| --- | --- |\n"), "{markdown:?}");
    let reopened = import_markdown_bytes("table-delimiter-roundtrip.md", &markdown).expect("reopen markdown");
    let (head, body) = table(&reopened);
    assert_eq!(head.len(), 1);
    assert_eq!(body.len(), 1);
    assert_eq!(body[0].cells.iter().map(|cell| cell_text(&reopened, cell)).collect::<Vec<_>>(), vec!["1".to_string(), "2".to_string()]);
}

#[test]
fn markdown_spanning_table_reopens_as_a_flattened_pipe_table() {
    let mut graph = DocumentGraph::new(DocumentId(7));
    let head_text = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "Head".into() }] });
    let a = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "a".into() }] });
    let b = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "b".into() }] });
    let c = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "c".into() }] });
    graph.push_block(Block::Table {
        caption: None,
        head: vec![TableRow { cells: vec![TableCell { children: vec![head_text], colspan: 2, rowspan: 1 }] }],
        body: vec![
            TableRow {
                cells: vec![TableCell { children: vec![a], colspan: 1, rowspan: 2 }, TableCell { children: vec![b], colspan: 1, rowspan: 1 }],
            },
            TableRow { cells: vec![TableCell { children: vec![c], colspan: 1, rowspan: 1 }] },
        ],
        foot: Vec::new(),
    });

    let markdown = export_markdown(&graph).expect("export spanning table");
    assert!(markdown.contains("<table>"), "{markdown:?}");
    assert!(markdown.contains("colspan=\"2\""), "{markdown:?}");
    assert!(markdown.contains("rowspan=\"2\""), "{markdown:?}");

    let reopened = import_markdown_bytes("span-table-roundtrip.md", &markdown).expect("reopen spanning table");
    let (head, body) = table(&reopened);
    let rows: Vec<&TableRow> = head.iter().chain(body).collect();
    // Inline HTML tables are not rendered downstream, so every span collapses
    // into a rectangular grid of single cells and the downgrade is reported.
    assert!(rows.iter().flat_map(|row| &row.cells).all(|cell| cell.colspan == 1 && cell.rowspan == 1));
    assert_eq!(head.len(), 1);
    assert_eq!(rows[0].cells.len(), 2);
    assert_eq!(cell_text(&reopened, &rows[0].cells[0]), "Head");
    assert!(body[0].cells.iter().any(|cell| cell_text(&reopened, cell) == "a"), "{:?}", body[0].cells);
    assert!(
        reopened.coverage.loss.iter().any(|loss| loss.code == "import.html.table_spans" && loss.status == SemanticStatus::Partial),
        "{:?}",
        reopened.coverage.loss
    );
    assert!(reopened.validate().is_valid());
    let flattened = export_markdown(&reopened).expect("re-export flattened table");
    assert!(!flattened.contains("<table>"), "{flattened:?}");
    assert!(flattened.contains("| Head |"), "{flattened:?}");
    assert!(flattened.contains("| a | b |"), "{flattened:?}");
}

fn cell_block_kinds(graph: &DocumentGraph, cell: &TableCell) -> Vec<&'static str> {
    cell.children
        .iter()
        .filter_map(|id| graph.block(*id))
        .map(|node| match &node.block {
            Block::Paragraph { .. } => "paragraph",
            Block::Table { .. } => "table",
            _ => "other",
        })
        .collect()
}

fn top_level_table<'a>(graph: &'a DocumentGraph) -> &'a Block {
    let referenced: std::collections::HashSet<document_ir::NodeId> = graph
        .blocks
        .iter()
        .flat_map(|node| match &node.block {
            Block::Section { children, .. } | Block::Quote { children } => children.clone(),
            Block::Table { head, body, foot, .. } => {
                head.iter().chain(body).chain(foot).flat_map(|row| row.cells.iter()).flat_map(|cell| cell.children.iter().copied()).collect()
            }
            _ => Vec::new(),
        })
        .collect();
    &graph
        .blocks
        .iter()
        .find(|node| matches!(node.block, Block::Table { .. }) && !referenced.contains(&node.id))
        .unwrap_or_else(|| panic!("top-level table expected: {:#?}", graph.blocks))
        .block
}

#[test]
fn markdown_html_table_with_a_nested_table_round_trips_as_blocks() {
    const SOURCE: &str = "<table><tr><td><p>outer</p><table><tr><td>inner</td></tr></table></td></tr></table>";
    let graph = import_markdown_bytes("nested-cell-table.md", SOURCE).expect("import markdown");
    assert!(
        !graph.blocks.iter().any(|node| matches!(node.block, Block::RawHtml { .. })),
        "a nested table inside a cell must lower to blocks: {:#?}",
        graph.blocks
    );
    assert!(!graph.coverage.loss.iter().any(|loss| loss.code == "import.markdown.raw_html_unrendered"), "{:?}", graph.coverage.loss);

    let Block::Table { body, .. } = top_level_table(&graph)
    else {
        panic!("top-level table expected: {:#?}", graph.blocks)
    };
    let cell = &body[0].cells[0];
    assert_eq!(cell_block_kinds(&graph, cell), ["paragraph", "table"], "cell blocks stay separate: {cell:?}");

    let markdown = export_markdown(&graph).expect("export markdown");
    let reopened = import_markdown_bytes("nested-cell-table-roundtrip.md", &markdown).expect("reopen markdown");
    let Block::Table { body: reopened_body, .. } = top_level_table(&reopened)
    else {
        panic!("top-level table expected after round trip: {:#?}", reopened.blocks)
    };
    let reopened_cell = &reopened_body[0].cells[0];
    assert_eq!(cell_block_kinds(&reopened, reopened_cell), ["paragraph", "table"], "round trip keeps the nested table: {markdown}");
}

#[test]
fn markdown_definition_list_lowers_to_paragraphs_with_a_loss_code() {
    let source = "Term\n: definition\n: second\n";
    let graph = import_markdown_bytes("definition-list.md", source).expect("import markdown");
    let texts = graph
        .blocks
        .iter()
        .map(|node| match &node.block {
            Block::Paragraph { content } => content.iter().map(Inline::plain_text).collect::<String>(),
            other => panic!("paragraph expected, got {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(texts, vec!["Term".to_string(), "definition".to_string(), "second".to_string()]);
    assert!(
        graph.coverage.loss.iter().any(|loss| loss.code == "import.markdown.definition_list" && loss.status == SemanticStatus::Partial),
        "{:?}",
        graph.coverage.loss
    );
    assert!(graph.validate().is_valid());

    let markdown = export_markdown(&graph).expect("export definition list");
    assert_eq!(markdown, "Term\n\ndefinition\n\nsecond\n\n");
    let reopened = import_markdown_bytes("definition-list-roundtrip.md", &markdown).expect("reopen definition list");
    assert!(!reopened.coverage.loss.iter().any(|loss| loss.code == "import.markdown.definition_list"), "{:?}", reopened.coverage.loss);
}

#[test]
fn markdown_inline_code_strips_backticks() {
    let source = "Use `code` here.\n";
    let graph = import_markdown_bytes("inline-code.md", source).expect("import markdown");
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("paragraph expected: {:#?}", graph.blocks)
    };
    assert!(content.iter().any(|inline| matches!(inline, Inline::InlineCode { text } if text == "code")), "{content:#?}");
    assert!(graph.validate().is_valid());

    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("`code`"), "{markdown:?}");
    let reopened = import_markdown_bytes("inline-code-roundtrip.md", &markdown).expect("reopen markdown");
    let Block::Paragraph { content } = &reopened.blocks[0].block
    else {
        panic!("reopened paragraph expected: {:#?}", reopened.blocks)
    };
    assert!(content.iter().any(|inline| matches!(inline, Inline::InlineCode { text } if text == "code")), "{content:#?}");
}

#[test]
fn markdown_import_lowers_heading_and_paragraph() {
    let source = "# Title\n\nHello world.\n";
    let graph = import_markdown_bytes("sample.md", source).expect("import markdown");
    assert_eq!(graph.blocks.len(), 2);
    assert!(matches!(
        &graph.blocks[0].block,
        Block::Section { title, level: 1, .. }
            if title == &[Inline::Text { text: "Title".into() }]
    ));
    assert!(matches!(&graph.blocks[1].block, Block::Paragraph { .. }));
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("# Title"));
    assert!(markdown.contains("Hello world."));
    let reopened = import_markdown_bytes("reopened.md", &markdown).expect("reopen markdown");
    assert!(matches!(
        &reopened.blocks[0].block,
        Block::Section { title, .. }
            if title.iter().any(|inline| matches!(inline, Inline::Text { text } if text == "Title"))
    ));
}

#[test]
fn markdown_import_lowers_inline_styles_and_links() {
    let source = "Hello **bold** and *italic* with [link](https://example.com/a(b)).\n";
    let graph = import_markdown_bytes("inline.md", source).expect("import markdown");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("**bold**"));
    assert!(markdown.contains("*italic*"));
    assert!(markdown.contains(r"[link](https://example.com/a\(b\))"), "markdown: {markdown:?}");
    assert!(markdown.contains(r" with [link](https://example.com/a\(b\))."));
}

#[test]
fn markdown_import_lowers_strikethrough_to_a_strike_style() {
    let source = "This is ~~obsolete~~ text.\n";
    let graph = import_markdown_bytes("strike.md", source).expect("import markdown");
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("paragraph expected")
    };
    assert!(
        matches!(&content[1], Inline::Styled { style, children } if style == "strike" && children == &[Inline::Text { text: "obsolete".into() }]),
        "{content:?}"
    );

    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("~~obsolete~~"), "{markdown:?}");
    let reopened = import_markdown_bytes("reopened.md", &markdown).expect("reopen markdown");
    let Block::Paragraph { content } = &reopened.blocks[0].block
    else {
        panic!("paragraph expected")
    };
    assert!(
        matches!(&content[1], Inline::Styled { style, children } if style == "strike" && children == &[Inline::Text { text: "obsolete".into() }]),
        "{content:?}"
    );
}

#[test]
fn markdown_reference_links_resolve_definitions_and_reopen_as_inline_links() {
    let source = "Read [the guide][Guide] and [guide][].\n\n[guide]: <https://example.com/guide> \"Guide\"\n";
    let graph = import_markdown_bytes("reference.md", source).expect("import markdown");
    let markdown = export_markdown(&graph).expect("export markdown");
    let reopened = import_markdown_bytes("reopened.md", &markdown).expect("re-import markdown");
    let Block::Paragraph { content } = &reopened.blocks[0].block
    else {
        panic!("paragraph")
    };
    let links = content
        .iter()
        .filter_map(|inline| match inline {
            Inline::Link { target, display, title } => Some((target, display, title)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(links.len(), 2, "{content:#?}; output={markdown:?}");
    assert!(links.iter().all(|(target, _, _)| **target == RelationEndpoint::ExternalUri("https://example.com/guide".into())));
    assert_eq!(links.iter().map(|(_, display, _)| display[0].plain_text()).collect::<Vec<_>>(), vec!["the guide", "guide"]);
    assert_eq!(links.iter().map(|(_, _, title)| title.as_deref()).collect::<Vec<_>>(), vec![Some("Guide"), Some("Guide")]);
    assert!(graph.validate().is_valid());
}

#[test]
fn markdown_shortcut_reference_resolves_and_reopens() {
    let source = "Read [Guide] now.\n\n[guide]: https://example.com/guide\n";
    let graph = import_markdown_bytes("shortcut.md", source).expect("import markdown");
    let markdown = export_markdown(&graph).expect("export markdown");
    let reopened = import_markdown_bytes("shortcut-output.md", &markdown).expect("reopen markdown");
    let Block::Paragraph { content } = &reopened.blocks[0].block
    else {
        panic!("paragraph")
    };
    assert!(content.iter().any(|inline| matches!(
        inline,
        Inline::Link { target, display, .. }
            if target == &RelationEndpoint::ExternalUri("https://example.com/guide".into())
                && display == &vec![Inline::Text { text: "Guide".into() }]
    )));
    assert!(graph.coverage.loss.is_empty(), "{:#?}", graph.coverage.loss);
}

#[test]
fn markdown_undefined_shortcut_is_literal_without_link_loss() {
    let source = "Keep [literal] and ![literal image] unchanged.";
    let graph = import_markdown_bytes("literal-brackets.md", source).expect("import markdown");
    assert!(graph.coverage.loss.is_empty(), "{:#?}", graph.coverage.loss);
    assert!(graph.assets.is_empty());
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("paragraph")
    };
    assert!(content.iter().all(|inline| matches!(inline, Inline::Text { .. })));
    let markdown = export_markdown(&graph).expect("export");
    let reopened = import_markdown_bytes("literal-output.md", &markdown).expect("reopen");
    assert_eq!(reopened.blocks[0].block, graph.blocks[0].block);
}

fn collect_links<'a>(inlines: &'a [Inline], links: &mut Vec<&'a Vec<Inline>>) {
    for inline in inlines {
        match inline {
            Inline::Link { display, .. } => {
                links.push(display);
                collect_links(display, links);
            }
            Inline::Styled { children, .. } | Inline::Attributed { children, .. } => collect_links(children, links),
            Inline::Image { alt, .. } => collect_links(alt, links),
            Inline::Note { display: Some(display), .. } => collect_links(display, links),
            _ => {}
        }
    }
}

fn link_displays(graph: &DocumentGraph) -> Vec<&Vec<Inline>> {
    let mut links = Vec::new();
    for node in &graph.blocks {
        match &node.block {
            Block::Paragraph { content } => collect_links(content, &mut links),
            Block::Section { title, .. } => collect_links(title, &mut links),
            Block::List { items, .. } => {
                for item in items {
                    collect_links(&item.content, &mut links);
                }
            }
            _ => {}
        }
    }
    links
}

fn only_link_display(graph: &DocumentGraph) -> &Vec<Inline> {
    let links = link_displays(graph);
    assert_eq!(links.len(), 1, "one link expected: {:#?}", graph.blocks);
    links[0]
}

#[test]
fn markdown_image_inside_a_link_keeps_the_image() {
    let cases = [
        ("[![alt](a.png)](https://example.com)\n", vec![("alt", "a.png")]),
        ("[cover ![alt](a.png)](https://example.com)\n", vec![("alt", "a.png")]),
        ("# [![alt](a.png)](https://example.com)\n", vec![("alt", "a.png")]),
        ("- [![alt](a.png)](https://example.com)\n", vec![("alt", "a.png")]),
        ("| [![alt](a.png)](https://example.com) |\n| --- |\n", vec![("alt", "a.png")]),
        ("[![alt][img]][link]\n\n[img]: a.png\n[link]: https://example.com\n", vec![("alt", "a.png")]),
    ];
    for (source, expected) in cases {
        let graph = import_markdown_bytes("image-in-link.md", source).expect("import markdown");
        let display = only_link_display(&graph);
        let images: Vec<(&str, &str)> = display
            .iter()
            .filter_map(|inline| match inline {
                Inline::Image { source, alt, .. } => {
                    let alt = alt.iter().map(Inline::plain_text).collect::<String>();
                    Some((Box::leak(alt.into_boxed_str()) as &str, source.as_str()))
                }
                _ => None,
            })
            .collect();
        assert_eq!(images, expected, "image in link label must survive: {source:?} -> {display:#?}");
        assert!(
            !graph.coverage.loss.iter().any(|loss| loss.code == "import.markdown.unresolved_reference_link"),
            "a resolvable image in a link must not report an unresolved label: {source:?} -> {:#?}",
            graph.coverage.loss
        );
    }
}

#[test]
fn markdown_image_in_a_mixed_link_label_round_trips() {
    let source = "[cover ![alt](a.png)](https://example.com)\n";
    let graph = import_markdown_bytes("mixed-label.md", source).expect("import markdown");
    let display = only_link_display(&graph);
    assert_eq!(display.len(), 2, "the label keeps its text run plus the image: {display:#?}");
    assert!(matches!(&display[0], Inline::Text { text } if text == "cover "), "{display:#?}");

    let codes: Vec<&str> = graph.coverage.loss.iter().map(|loss| loss.code.as_str()).collect();
    assert_eq!(codes, ["import.markdown.asset_unresolved"], "the only loss is the unresolved asset: {:#?}", graph.coverage.loss);

    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("![alt](a.png)"), "{markdown}");
    let reopened = import_markdown_bytes("mixed-label-roundtrip.md", &markdown).expect("reopen markdown");
    let reopened_display = only_link_display(&reopened);
    assert!(
        reopened_display.iter().any(|inline| matches!(inline, Inline::Image { source, .. } if source == "a.png")),
        "the image survives the round trip: {reopened_display:#?}"
    );
}

#[test]
fn markdown_unresolved_reference_links_are_preserved_and_reported() {
    let graph = import_markdown_bytes("unresolved.md", "before [label][missing] after\n\n").expect("import markdown");
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("paragraph")
    };
    assert!(content.iter().any(|inline| matches!(inline, Inline::Text { text } if text.contains("[label][missing]"))), "{content:#?}");
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "import.markdown.unresolved_reference_link"));
}

#[test]
fn markdown_reference_images_resolve_and_register_assets() {
    let source = "![diagram][figure]\n\n[figure]: assets/diagram.png\n";
    let graph = import_markdown_bytes("reference-image.md", source).expect("import markdown");
    assert_eq!(graph.assets.len(), 1);
    assert_eq!(graph.assets[0].source.as_deref(), Some("assets/diagram.png"));
    assert!(matches!(
        &graph.blocks[0].block,
        Block::Paragraph { content }
            if content.iter().any(|inline| matches!(
                inline,
                Inline::Image { source, alt, asset: Some(_), .. }
                    if source == "assets/diagram.png"
                        && alt == &vec![Inline::Text { text: "diagram".into() }]
            ))
    ));
}

#[test]
fn markdown_round_trip_validates() {
    let source = "# Route\n\nBody.\n";
    let graph = import_markdown_bytes("route.md", source).expect("import");
    assert!(graph.validate().is_valid());
    let markdown = export_markdown(&graph).expect("export");
    assert!(markdown.contains("# Route"));
}

#[test]
fn markdown_math_reopen_preserves_semantics_and_following_text() {
    let source = "before $x^2 + y^2$ after\n\n$$E = mc^2$$\n";
    let graph = import_markdown_bytes("math.md", source).expect("import");
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("paragraph")
    };
    assert_eq!(content.len(), 3);
    let Inline::InlineMath { formula } = &content[1]
    else {
        panic!("expected inline math, got {content:?}")
    };
    assert_eq!(formula.source, "x^2 + y^2");
    assert_eq!(formula.language.as_deref(), Some("latex"));
    let math_block = graph
        .blocks
        .iter()
        .find_map(|node| match &node.block {
            Block::Math { formula } => Some(formula),
            _ => None,
        })
        .expect("display math block");
    assert_eq!(math_block.source, "E = mc^2");
    assert_eq!(math_block.language.as_deref(), Some("latex"));
    let markdown = export_markdown(&graph).expect("export");
    let reopened = import_markdown_bytes("math-round.md", &markdown).expect("reopen");
    assert_eq!(reopened.blocks[0].block, graph.blocks[0].block);
    assert!(reopened.blocks.iter().any(|node| match &node.block {
        Block::Math { formula } => formula.source == "E = mc^2" && formula.language.as_deref() == Some("latex"),
        _ => false,
    }));
}

/// `\text` keeps every space it was authored with. The HTML and Markdown
/// writers must pass the phrase through untouched, so the round trip cannot
/// silently trim either side of the content.
#[cfg(feature = "html")]
#[test]
fn math_text_spaces_survive_the_html_and_markdown_roundtrip() {
    use document_formats::import::html::import_html_text;

    let source = "$$\nx + \\text{ text } + y\n$$\n";
    let graph = import_markdown_bytes("math-text.md", source).expect("import");
    let formula = graph
        .blocks
        .iter()
        .find_map(|node| match &node.block {
            Block::Math { formula } => Some(formula),
            _ => None,
        })
        .expect("display math block");
    let MathExpression::Sequence(items) = &formula.expression
    else {
        panic!("expected a sequence, got {:#?}", formula.expression)
    };
    let text = items
        .iter()
        .find_map(|item| match item {
            MathExpression::Text { text, .. } => Some(text),
            _ => None,
        })
        .expect("the `\\text` node");
    assert_eq!(text.chars().collect::<Vec<_>>(), vec![' ', 't', 'e', 'x', 't', ' '], "the parser must keep both spaces");

    let html = export_html(&graph).expect("export html");
    assert!(html.contains("x + \\text{ text } + y"), "the HTML math node must keep the authored spaces: {html}");

    let reopened = import_html_text("math-text.html", &html).expect("import html");
    let markdown = export_markdown(&reopened).expect("export markdown");
    assert!(markdown.contains(" text "), "the text must not be trimmed on the way back: {markdown}");
}

#[test]
fn markdown_footnote_reopen_preserves_reference_body_and_following_text() {
    let source = "See[^1] after\n\n[^1]: Body.\n";
    let graph = import_markdown_bytes("footnote.md", source).expect("import");
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("paragraph")
    };
    assert_eq!(
        content,
        &vec![Inline::Text { text: "See".into() }, Inline::Note { label: "1".into(), display: None }, Inline::Text { text: " after".into() },]
    );
    let markdown = export_markdown(&graph).expect("export");
    let reopened = import_markdown_bytes("footnote-round.md", &markdown).expect("reopen");
    assert_eq!(reopened.blocks[0].block, graph.blocks[0].block);
    assert!(reopened.blocks.iter().any(|node| matches!(&node.block,
        Block::Opaque { kind, payload_hint, .. } if kind == "footnote_definition" && payload_hint.trim() == "[^1]: Body."
    )));
}

#[test]
fn markdown_footnote_definition_covers_inline_display_content() {
    let mut graph = DocumentGraph::new(DocumentId(43));
    graph.push_block(Block::Paragraph {
        content: vec![
            Inline::Text { text: "See".into() },
            Inline::Note { label: "pandoc-note-1".into(), display: Some(vec![Inline::Text { text: "Body.".into() }]) },
            Inline::Text { text: " after".into() },
        ],
    });
    let markdown = export_markdown(&graph).expect("inline note display must have a footnote spelling");
    assert!(markdown.contains("See[^pandoc-note-1] after"), "{markdown:?}");
    assert!(markdown.contains("[^pandoc-note-1]: Body."), "{markdown:?}");

    let reopened = import_markdown_bytes("inline-note-display.md", &markdown).expect("reopen");
    let Block::Paragraph { content } = &reopened.blocks[0].block
    else {
        panic!("paragraph expected: {:#?}", reopened.blocks)
    };
    assert_eq!(content[1], Inline::Note { label: "pandoc-note-1".into(), display: None });
    assert!(reopened.blocks.iter().any(|node| matches!(&node.block,
        Block::Opaque { kind, payload_hint, .. }
            if kind == "footnote_definition" && payload_hint.trim() == "[^pandoc-note-1]: Body."
    )));
}

#[test]
fn markdown_authored_footnote_definition_wins_over_synthesized_display() {
    let mut graph = DocumentGraph::new(DocumentId(44));
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Note { label: "1".into(), display: Some(vec![Inline::Text { text: "Reference".into() }]) }],
    });
    graph.push_block(Block::Opaque {
        kind: "footnote_definition".into(),
        payload_hint: "[^1]: Authored.".into(),
        status: SemanticStatus::Resolved,
    });
    let markdown = export_markdown(&graph).expect("export");
    assert_eq!(markdown.matches("[^1]:").count(), 1, "the label must be defined once: {markdown:?}");
    assert!(markdown.contains("[^1]: Authored."), "{markdown:?}");
}

#[test]
fn export_from_programmatic_ir() {
    let mut graph = DocumentGraph::new(DocumentId(42));
    graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "plain ir".into() }] });
    let markdown = export_markdown(&graph).expect("export markdown from programmatic ir");
    assert!(markdown.contains("plain ir"));
    let reopened = import_markdown_bytes("plain.md", &markdown).expect("reopen");
    assert!(reopened.validate().is_valid());
}

#[test]
fn markdown_export_does_not_silently_drop_reference_identity() {
    let mut graph = DocumentGraph::new(DocumentId(11));
    let target = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "target".into() }] });
    graph.push_block(Block::Paragraph { content: vec![Inline::Reference { target, display: "see target".into() }] });
    assert!(export_markdown(&graph).expect_err("reference identity cannot be discarded").to_string().contains("anchor mapping"));
}

#[test]
fn markdown_export_preserves_nested_section_content_on_reopen() {
    let mut graph = DocumentGraph::new(DocumentId(9));
    let lead = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "lead paragraph".into() }] });
    let nested_body = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "nested body".into() }] });
    let nested_section =
        graph.push_block(Block::Section { level: 2, title: vec![Inline::Text { text: "Nested".into() }], children: vec![nested_body] });
    graph.push_block(Block::Section { level: 1, title: vec![Inline::Text { text: "Root".into() }], children: vec![lead, nested_section] });

    let markdown = export_markdown(&graph).expect("export");
    let reopened = import_markdown_bytes("nested.md", &markdown).expect("reopen");
    let reopened_markdown = reopened
        .blocks
        .iter()
        .filter_map(|node| match &node.block {
            Block::Section { title, .. } => Some(
                title
                    .iter()
                    .filter_map(|inline| match inline {
                        Inline::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<String>(),
            ),
            Block::Paragraph { content } => {
                let text = content
                    .iter()
                    .filter_map(|inline| match inline {
                        Inline::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<String>();
                (!text.is_empty()).then_some(text)
            }
            other => panic!("unexpected reopened block: {other:?}"),
        })
        .collect::<Vec<_>>();

    assert_eq!(reopened_markdown, ["Root", "lead paragraph", "Nested", "nested body"]);
}

#[test]
fn markdown_image_reopen_preserves_asset_relation_and_surrounding_text() {
    let source = "before ![diagram](images/diagram.png) after\n";
    let graph = import_markdown_bytes("image.md", source).expect("import");
    let markdown = export_markdown(&graph).expect("export");
    let reopened = import_markdown_bytes("image-roundtrip.md", &markdown).expect("reopen");

    assert_eq!(reopened.assets.len(), 1);
    let asset = &reopened.assets[0];
    assert_eq!(asset.source.as_deref(), Some("images/diagram.png"));
    assert_eq!(asset.status, SemanticStatus::Unresolved);
    assert!(
        reopened
            .relations
            .iter()
            .any(|relation| { relation.kind == RelationKind::Embeds && relation.target == RelationEndpoint::Asset(asset.id) })
    );
    let Block::Paragraph { content } = &reopened.blocks[0].block
    else {
        panic!("expected reopened paragraph")
    };
    assert!(matches!(&content[0], Inline::Text { text } if text == "before "));
    assert!(matches!(&content[1], Inline::Image { source, asset: Some(bound), .. } if source == "images/diagram.png" && *bound == asset.id));
    assert!(matches!(&content[2], Inline::Text { text } if text == " after"));
}

#[test]
fn markdown_export_rejects_containment_cycles() {
    let mut graph = DocumentGraph::new(DocumentId(10));
    let section = graph.push_block(Block::Section { level: 1, title: vec![Inline::Text { text: "Cycle".into() }], children: Vec::new() });
    if let Block::Section { children, .. } = &mut graph.blocks[0].block {
        children.push(section);
    }

    let error = export_markdown(&graph).expect_err("invalid graph must not recurse");
    assert!(error.to_string().contains("ContainmentCycle"));
}

#[test]
fn markdown_export_escapes_literal_syntax_characters() {
    let mut graph = DocumentGraph::new(DocumentId(1));
    graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "literal *stars* and [brackets] | marker".into() }] });

    let markdown = export_markdown(&graph).expect("export");
    assert!(markdown.contains(r"literal \*stars\* and \[brackets\] \| marker"));

    let reopened = import_markdown_bytes("escaped.md", &markdown).expect("re-import");
    assert_eq!(
        reopened.blocks[0].block,
        Block::Paragraph { content: vec![Inline::Text { text: "literal *stars* and [brackets] | marker".into() }] }
    );
}

#[test]
fn markdown_export_rejects_unrepresentable_inline_styles() {
    let mut graph = DocumentGraph::new(DocumentId(1));
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Styled { style: "small_caps".into(), children: vec![Inline::Text { text: "meaningful style".into() }] }],
    });

    let error = export_markdown(&graph).expect_err("style must not be silently dropped");
    assert!(error.to_string().contains("small_caps"));
}

#[test]
fn markdown_export_chooses_safe_code_delimiters() {
    let mut graph = DocumentGraph::new(DocumentId(2));
    graph.push_block(Block::Paragraph { content: vec![Inline::InlineCode { text: "`a``b`".into() }] });
    graph.push_block(Block::Code { language: None, content: "line with ``` inside".into() });

    let markdown = export_markdown(&graph).expect("export");
    assert!(markdown.starts_with("``` `a``b` ```\n\n"));
    assert!(markdown.contains("````\nline with ``` inside\n````"));
}

#[test]
fn markdown_inline_code_matches_opening_backtick_run() {
    let source = "Use ``a`b`` here.\n";
    let graph = import_markdown_bytes("inline-code-multi-backtick.md", source).expect("import markdown");
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("paragraph expected: {:#?}", graph.blocks)
    };
    assert_eq!(content.iter().filter(|inline| matches!(inline, Inline::InlineCode { .. })).count(), 1, "{content:#?}");
    assert_eq!(content.len(), 3, "{content:#?}");
    assert_eq!(content.first(), Some(&Inline::Text { text: "Use ".into() }), "{content:#?}");
    assert_eq!(content.get(1), Some(&Inline::InlineCode { text: "a`b".into() }), "{content:#?}");
    assert!(matches!(content.get(2), Some(Inline::Text { text }) if text.starts_with(" here.")), "{content:#?}");
    assert!(graph.validate().is_valid());

    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("``a`b``"), "{markdown:?}");
    let reopened = import_markdown_bytes("inline-code-multi-backtick-roundtrip.md", &markdown).expect("reopen markdown");
    let Block::Paragraph { content } = &reopened.blocks[0].block
    else {
        panic!("reopened paragraph expected: {:#?}", reopened.blocks)
    };
    assert_eq!(content.get(1), Some(&Inline::InlineCode { text: "a`b".into() }), "{content:#?}");
    assert_eq!(content.iter().map(Inline::plain_text).collect::<String>(), "Use a`b here.", "{content:#?}");
    assert!(reopened.validate().is_valid());

    let padded = "Use `` `a` `` here.\n";
    let graph = import_markdown_bytes("inline-code-padded.md", padded).expect("import padded markdown");
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("padded paragraph expected: {:#?}", graph.blocks)
    };
    assert_eq!(content.get(1), Some(&Inline::InlineCode { text: "`a`".into() }), "{content:#?}");
    assert_eq!(content.iter().map(Inline::plain_text).collect::<String>(), "Use `a` here.", "{content:#?}");
    assert!(graph.validate().is_valid());
}

#[test]
fn markdown_thematic_break_lowers_to_the_canonical_opaque_block() {
    let graph = import_markdown_bytes("break.md", "Alpha\n\n---\n\nBeta\n").expect("import markdown");
    assert_eq!(graph.blocks.len(), 3, "{:#?}", graph.blocks);
    assert!(matches!(&graph.blocks[0].block, Block::Paragraph { .. }), "{:#?}", graph.blocks[0].block);
    assert_eq!(
        graph.blocks[1].block,
        Block::Opaque { kind: "thematic_break".into(), payload_hint: "---".into(), status: SemanticStatus::Partial }
    );
    assert!(matches!(&graph.blocks[2].block, Block::Paragraph { .. }), "{:#?}", graph.blocks[2].block);
    assert!(!graph.coverage.loss.iter().any(|loss| loss.code == "import.markdown.horizontal_rule"), "{:#?}", graph.coverage.loss);
    assert!(graph.validate().is_valid());

    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("---"), "{markdown:?}");
    let reopened = import_markdown_bytes("break-roundtrip.md", &markdown).expect("reopen markdown");
    assert_eq!(reopened.blocks[1].block, graph.blocks[1].block);
    assert!(reopened.validate().is_valid());
}

fn code_block_content(markdown: &str) -> (Option<String>, String) {
    let graph = import_markdown_bytes("fenced-code.md", markdown).expect("import markdown");
    let Block::Code { language, content } = &graph.blocks[0].block
    else {
        panic!("code block expected: {:#?}", graph.blocks)
    };
    (language.clone(), content.clone())
}

fn diagram_block(markdown: &str) -> (String, String) {
    let graph = import_markdown_bytes("fenced-diagram.md", markdown).expect("import markdown");
    let Block::Diagram { language, source } = &graph.blocks[0].block
    else {
        panic!("diagram block expected: {:#?}", graph.blocks)
    };
    assert!(graph.validate().is_valid(), "{:#?}", graph.validate().issues);
    (language.clone(), source.clone())
}

#[test]
fn markdown_mermaid_fence_lowers_to_a_diagram_block() {
    let source = "```mermaid\nflowchart TD\n  A[Start] --> B{Done?}\n  B -->|yes| C[Stop]\n```\n";
    let (language, body) = diagram_block(source);
    assert_eq!(language, "mermaid");
    assert_eq!(body, "flowchart TD\n  A[Start] --> B{Done?}\n  B -->|yes| C[Stop]");
}

#[test]
fn markdown_diagram_round_trips_through_a_mermaid_fence() {
    let graph = import_markdown_bytes("diagram-roundtrip.md", "```mermaid\nflowchart TD\n  A[Start] --> B{Done?}\n  B -->|yes| C[Stop]\n```\n")
        .expect("import markdown");

    let markdown = export_markdown(&graph).expect("export diagram markdown");
    assert!(markdown.contains("```mermaid"), "{markdown:?}");
    let reopened = import_markdown_bytes("diagram-reopen.md", &markdown).expect("reopen markdown");
    assert_eq!(reopened.blocks[0].block, graph.blocks[0].block);
    assert!(reopened.validate().is_valid());
}

#[test]
fn markdown_diagram_export_rejects_languages_without_a_reopen_rule() {
    let mut graph = DocumentGraph::new(DocumentId(3));
    graph.push_block(Block::Diagram { language: "custom-diagram".into(), source: "A -> B".into() });

    let error = export_markdown(&graph).expect_err("a diagram without a reopen rule cannot silently reopen as code");
    assert!(error.to_string().contains("custom-diagram"), "{error}");
}

#[test]
fn markdown_diagram_keeps_indented_source_lines() {
    let (language, body) = diagram_block("```mermaid\n  A[Start] --> B{Done?}\n    nested[Node]\n```\n");
    assert_eq!(language, "mermaid");
    assert_eq!(body, "  A[Start] --> B{Done?}\n    nested[Node]");
}

#[test]
fn markdown_fenced_code_supports_tilde_and_long_backtick_fences() {
    let (tilde_language, tilde_content) = diagram_block("~~~mermaid\nA[Start] --> B{Done?}\n~~~\n");
    assert_eq!(tilde_language, "mermaid");
    assert_eq!(tilde_content, "A[Start] --> B{Done?}");

    let (backtick_language, backtick_content) = code_block_content("````js\nconst a = `b`;\n````\n");
    assert_eq!(backtick_language.as_deref(), Some("js"));
    assert_eq!(backtick_content, "const a = `b`;");
}

#[test]
fn markdown_fenced_code_allows_an_indented_opening_fence() {
    let (language, content) = diagram_block("  ```mermaid\nA[Start] --> B\n  ```\n");
    assert_eq!(language, "mermaid");
    assert_eq!(content, "A[Start] --> B");

    let deep = import_markdown_bytes("deep-fence.md", "    ```mermaid\nA[Start] --> B\n    ```\n").expect("import markdown");
    assert!(!matches!(&deep.blocks[0].block, Block::Diagram { .. }), "four spaces must not open a fence: {:#?}", deep.blocks[0].block);
}

#[test]
fn markdown_fenced_code_keeps_inner_blank_lines() {
    let (language, content) = code_block_content("```\nfirst\n\nthird\n```\n");
    assert_eq!(language, None);
    assert_eq!(content, "first\n\nthird");
}

#[test]
fn markdown_setext_heading_is_not_a_thematic_break() {
    let graph = import_markdown_bytes("setext.md", "Title\n---\n").expect("import markdown");
    assert_eq!(graph.blocks.len(), 1, "{:#?}", graph.blocks);
    assert_eq!(graph.blocks[0].block, Block::Section { level: 2, title: vec![Inline::Text { text: "Title".into() }], children: Vec::new() });
    assert!(graph.coverage.loss.is_empty(), "{:#?}", graph.coverage.loss);
    assert!(graph.validate().is_valid());
}

#[test]
fn markdown_ordered_list_numbers_and_task_markers_round_trip() {
    let ordered_source = "3. three\n4. four\n";
    let ordered_graph = import_markdown_bytes("ordered-start.md", ordered_source).expect("import ordered list");
    let ordered_markdown = export_markdown(&ordered_graph).expect("export ordered list");
    assert_eq!(ordered_markdown, "3. three\n4. four\n\n");
    let ordered_reopened = import_markdown_bytes("ordered-start-roundtrip.md", &ordered_markdown).expect("reopen ordered list");
    assert_eq!(ordered_reopened.blocks[0].block, ordered_graph.blocks[0].block);

    let task_source = "- [x] done\n- [ ] pending\n- plain\n";
    let task_graph = import_markdown_bytes("task-roundtrip-source.md", task_source).expect("import task list");
    let task_markdown = export_markdown(&task_graph).expect("export task list");
    assert_eq!(task_markdown, "- [x] done\n- [ ] pending\n- plain\n\n");
    let task_reopened = import_markdown_bytes("task-roundtrip.md", &task_markdown).expect("reopen task list");
    assert_eq!(task_reopened.blocks[0].block, task_graph.blocks[0].block);
}

#[test]
fn markdown_task_list_lowers_checkbox_state() {
    let source = "- [x] done\n- [ ] pending\n- normal item\n";
    let graph = import_markdown_bytes("task-list.md", source).expect("import task list");
    let Block::List { ordered: false, items } = &graph.blocks[0].block
    else {
        panic!("unordered list expected: {:#?}", graph.blocks)
    };
    assert_eq!(items.len(), 3);

    assert!(items[0].is_task);
    assert_eq!(items[0].is_checked, Some(true));
    assert_eq!(items[0].content, vec![Inline::Text { text: "done".into() }]);

    assert!(items[1].is_task);
    assert_eq!(items[1].is_checked, Some(false));
    assert_eq!(items[1].content, vec![Inline::Text { text: "pending".into() }]);

    assert!(!items[2].is_task);
    assert_eq!(items[2].is_checked, None);
    assert_eq!(items[2].content, vec![Inline::Text { text: "normal item".into() }]);

    assert!(graph.validate().is_valid());
}

#[test]
fn markdown_inline_html_stays_verbatim() {
    let source = "a <b>x</b> b\n";
    let graph = import_markdown_bytes("inline-html.md", source).expect("import inline HTML");
    let markdown = export_markdown(&graph).expect("export inline HTML");
    assert_eq!(markdown, "a <b>x</b> b\n\n");
    let reopened = import_markdown_bytes("inline-html-roundtrip.md", &markdown).expect("reopen inline HTML");
    assert_eq!(export_markdown(&reopened).expect("re-export inline HTML"), markdown);
}

#[test]
fn markdown_raw_html_block_keeps_every_line() {
    for source in ["<div>text</div>\n", "<div>\nline one\nline two\n</div>\n"] {
        let graph = import_markdown_bytes("html-block.md", source).expect("import raw HTML block");
        let Block::RawHtml { html } = &graph.blocks[0].block
        else {
            panic!("expected one raw HTML block, got {:#?}", graph.blocks)
        };
        assert_eq!(graph.blocks.len(), 1, "{:#?}", graph.blocks);
        assert_eq!(html.trim_end(), source.trim_end());
        let markdown = export_markdown(&graph).expect("export raw HTML block");
        assert_eq!(markdown, format!("{}\n\n", source.trim_end()));
        let reopened = import_markdown_bytes("html-block-roundtrip.md", &markdown).expect("reopen raw HTML block");
        assert_eq!(export_markdown(&reopened).expect("re-export raw HTML block"), markdown);
    }
}

#[test]
fn markdown_raw_html_block_is_preserved() {
    let source = "# Task + HTML Test\n\n<div class=\"note\">\n\nBody text.\n";
    let graph = import_markdown_bytes("html.md", source).expect("import markdown with HTML");

    // Block 0: heading
    assert!(matches!(&graph.blocks[0].block, Block::Section { level: 1, .. }));

    // Block 1: RawHtml block for <div>
    assert!(
        graph.blocks.iter().any(|node| matches!(&node.block, Block::RawHtml { html } if html.contains("<div class=\"note\">"))),
        "expected RawHtml block, got: {:#?}",
        graph.blocks
    );

    assert!(graph.validate().is_valid());
    // The block is preserved verbatim, but the semantic SVG/PDF reflow does not
    // lay raw HTML out, so the loss stays truthful instead of being cleared.
    assert!(
        graph.coverage.loss.iter().any(|loss| loss.code == "import.markdown.raw_html_unrendered" && loss.status == SemanticStatus::Partial),
        "the unrendered raw HTML block must surface a structured loss: {:#?}",
        graph.coverage.loss
    );
}
#[test]
fn markdown_front_matter_is_reported_as_an_unmapped_loss_with_provenance() {
    let source = "---\ntitle: Notes\n---\n\nBody.\n";
    let graph = import_markdown_bytes("front.md", source).expect("import markdown");
    let index = graph.coverage.loss.iter().position(|loss| loss.code == "import.markdown.front_matter_unmapped").expect("front matter loss");
    assert_eq!(graph.coverage.loss[index].status, SemanticStatus::Unsupported);
    assert!(!graph.coverage.complete);
    let span = graph.loss_span(index).expect("front matter span");
    let authored = &source[span.start as usize..span.end as usize];
    assert!(authored.contains("title: Notes"), "{authored:?}");
    // The block keeps the authored bytes so a metadata-aware consumer can still
    // read them, while the Markdown writer stays fail-closed until the IR grows
    // a metadata field.
    assert!(
        matches!(
            &graph.blocks[0].block,
            Block::Opaque { kind, payload_hint, status }
                if kind == "front_matter" && payload_hint.contains("title: Notes") && *status == SemanticStatus::Unsupported
        ),
        "{:?}",
        graph.blocks
    );
    assert!(export_markdown(&graph).is_err(), "front matter export stays fail-closed");
}

#[test]
fn markdown_images_without_bytes_report_asset_unresolved() {
    let graph =
        import_markdown_bytes("images.md", "A local ![alt](pic.png) and a remote ![r](https://example.com/r.png)\n").expect("import markdown");
    let codes = graph.coverage.loss.iter().map(|loss| loss.code.as_str()).collect::<Vec<_>>();
    assert_eq!(codes, vec!["import.markdown.asset_unresolved", "import.markdown.asset_unresolved"], "{codes:?}");
    assert!(graph.coverage.loss.iter().all(|loss| loss.status == SemanticStatus::Partial), "{:#?}", graph.coverage.loss);
    assert_eq!(graph.assets.iter().filter(|asset| asset.kind == AssetKind::Image).count(), 2);
    assert!(graph.assets.iter().all(|asset| asset.status == SemanticStatus::Unresolved && asset.bytes.is_none()));
}

#[test]
fn markdown_images_with_supplied_bytes_resolve_assets() {
    let mut assets = Assets(std::collections::HashMap::new());
    assets.0.insert("pic.png".into(), png_bytes());
    let graph = import_markdown_bytes_with_assets("images.md", "A ![alt](pic.png)\n", &assets).expect("import with assets");
    assert!(graph.coverage.loss.is_empty(), "{:#?}", graph.coverage.loss);
    let asset = graph.assets.iter().find(|asset| asset.kind == AssetKind::Image).expect("image asset");
    assert_eq!(asset.status, SemanticStatus::Resolved);
    assert_eq!(asset.bytes.as_deref(), Some(png_bytes().as_slice()));
    assert_eq!(asset.media_type.as_deref(), Some("image/png"));
}

#[test]
fn markdown_export_keeps_table_caption_through_the_html_fallback() {
    let mut graph = DocumentGraph::new(DocumentId(72));
    let heading = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "h".into() }] });
    graph.push_block(Block::Table {
        caption: Some("表 1".into()),
        head: vec![TableRow { cells: vec![TableCell { children: vec![heading], colspan: 1, rowspan: 1 }] }],
        body: Vec::new(),
        foot: Vec::new(),
    });

    let markdown = export_markdown(&graph).expect("export table caption");
    assert!(markdown.contains("<caption>表 1</caption>"), "a GFM pipe table has no caption slot: {markdown}");
    let reopened = import_markdown_bytes("caption-roundtrip.md", &markdown).expect("reopen table caption");
    let Block::Table { caption, .. } = top_level_table(&reopened)
    else {
        panic!("top-level table expected after round trip: {:#?}", reopened.blocks)
    };
    assert_eq!(caption.as_deref(), Some("表 1"), "the caption must round trip through the HTML fallback");
}

mod export_project;
mod roundtrip;

#[test]
fn markdown_export_reports_only_declared_losses() {
    let graph = import_markdown_bytes("md-loss-channel", "# Title\n\nBody.\n").expect("markdown import");
    let export = export_markdown_report(&graph).expect("markdown export");
    assert!(export.text.contains("# Title"), "{}", export.text);
    let codes: Vec<&str> = export.losses.iter().map(|loss| loss.code.as_str()).collect();
    assert!(codes.is_empty(), "no Markdown loss code is declared yet, but the channel reported {codes:?}");
}
