//! Pandoc JSON codec and IR adapter tests.

use document_formats::{
    export::markdown::export_markdown,
    import::markdown::import_markdown_bytes,
    pandoc::{
        PandocBudget, PandocImportOptions,
        codec::{decode, encode},
        export_pandoc_json, import_pandoc_json_bytes_with,
        model::{
            API_VERSION, Alignment, Attr, Block, Caption, Cell, Citation, CitationMode, ColSpec, Inline, ListNumberDelim, ListNumberStyle,
            MathType, MetaValue, OrderedList, PandocDocument, QuoteType, Row, Table, TableBody, TableFoot, TableHead, Target,
        },
    },
};
use document_ir::{Block as IrBlock, DocumentGraph, Inline as IrInline, SemanticStatus};

fn encode_text(document: &PandocDocument) -> String {
    encode(document, &PandocBudget::default()).expect("encode")
}

fn decode_text(text: &str) -> PandocDocument {
    decode(text.as_bytes(), &PandocBudget::default()).expect("decode")
}

fn allow_loss() -> PandocImportOptions {
    PandocImportOptions::allow_loss()
}

fn import(text: &str) -> DocumentGraph {
    import_pandoc_json_bytes_with("pandoc-fixture", text.as_bytes(), &allow_loss()).expect("pandoc import")
}

fn attr(id: &str, classes: &[&str], pairs: &[(&str, &str)]) -> Attr {
    Attr {
        id: id.into(),
        classes: classes.iter().map(|class| (*class).into()).collect(),
        pairs: pairs.iter().map(|(key, value)| ((*key).into(), (*value).into())).collect(),
    }
}

fn inline_set() -> Vec<Inline> {
    let citation = Citation {
        id: "doe2024".into(),
        prefix: vec![Inline::Str("see".into())],
        suffix: vec![Inline::Str("p. 3".into())],
        mode: CitationMode::SuppressAuthor,
        note_num: 4,
        hash: 9,
    };
    vec![
        Inline::Str("text".into()),
        Inline::Emph(vec![Inline::Str("emph".into())]),
        Inline::Underline(vec![Inline::Str("under".into())]),
        Inline::Strong(vec![Inline::Str("strong".into())]),
        Inline::Strikeout(vec![Inline::Str("strike".into())]),
        Inline::Superscript(vec![Inline::Str("sup".into())]),
        Inline::Subscript(vec![Inline::Str("sub".into())]),
        Inline::SmallCaps(vec![Inline::Str("caps".into())]),
        Inline::Quoted { kind: QuoteType::Single, inlines: vec![Inline::Str("quoted".into())] },
        Inline::Cite { citations: vec![citation], inlines: vec![Inline::Str("(Doe 2024)".into())] },
        Inline::Code { attr: attr("code-id", &["rust"], &[("data-x", "1")]), text: "fn main() {}".into() },
        Inline::Space,
        Inline::SoftBreak,
        Inline::LineBreak,
        Inline::Math { display: MathType::Inline, text: "x^2".into() },
        Inline::RawInline { format: "html".into(), text: "<b>x</b>".into() },
        Inline::Link {
            attr: attr("link-id", &["c"], &[("k", "v")]),
            inlines: vec![Inline::Str("link".into())],
            target: Target { url: "https://example.com".into(), title: "title".into() },
        },
        Inline::Image {
            attr: attr("image-id", &["c"], &[]),
            inlines: vec![Inline::Str("alt".into())],
            target: Target { url: "img/x.png".into(), title: "img".into() },
        },
        Inline::Note(vec![Block::Para(vec![Inline::Str("note".into())])]),
        Inline::Span { attr: attr("span-id", &["s"], &[("k", "v1"), ("k", "v2")]), inlines: vec![Inline::Str("span".into())] },
    ]
}

fn all_constructors_document() -> PandocDocument {
    let row = Row {
        attr: Attr::default(),
        cells: vec![
            Cell {
                attr: attr("cell", &["c"], &[("k", "v")]),
                alignment: Alignment::Center,
                row_span: 2,
                col_span: 1,
                blocks: vec![Block::Plain(vec![Inline::Str("cell".into())])],
            },
            Cell {
                attr: Attr::default(),
                alignment: Alignment::Default,
                row_span: 1,
                col_span: 2,
                blocks: vec![Block::Para(vec![Inline::Str("wide".into())])],
            },
        ],
    };
    let table = Table {
        attr: attr("table", &["t"], &[("k", "a"), ("k", "b")]),
        caption: Caption { short: Some(vec![Inline::Str("short".into())]), blocks: vec![Block::Plain(vec![Inline::Str("caption".into())])] },
        colspecs: vec![ColSpec { alignment: Alignment::Left, width: 0.25 }, ColSpec { alignment: Alignment::Default, width: 0.0 }],
        head: TableHead { attr: Attr::default(), rows: vec![row.clone()] },
        bodies: vec![TableBody { attr: attr("body", &[], &[]), row_head_columns: 1, head: vec![row.clone()], body: vec![row.clone()] }],
        foot: TableFoot { attr: Attr::default(), rows: vec![row] },
    };
    let blocks = vec![
        Block::Plain(vec![Inline::Str("plain".into())]),
        Block::Para(inline_set()),
        Block::LineBlock(vec![vec![Inline::Str("line1".into())], vec![Inline::Str("line2".into())]]),
        Block::CodeBlock(attr("block", &["rust"], &[("k", "v1"), ("k", "v2")]), "code".into()),
        Block::RawBlock("html".into(), "<div>raw</div>".into()),
        Block::BlockQuote(vec![Block::Para(vec![Inline::Str("quote".into())])]),
        Block::OrderedList(OrderedList {
            start: 3,
            style: ListNumberStyle::UpperRoman,
            delimiter: ListNumberDelim::TwoParens,
            items: vec![vec![Block::Plain(vec![Inline::Str("one".into())])], vec![Block::Para(vec![Inline::Str("two".into())])]],
        }),
        Block::BulletList(vec![vec![Block::Plain(vec![Inline::Str("bullet".into())])]]),
        Block::DefinitionList(vec![(vec![Inline::Str("term".into())], vec![vec![Block::Plain(vec![Inline::Str("definition".into())])]])]),
        Block::Header { level: 2, attr: attr("head", &["h"], &[("k", "v")]), inlines: vec![Inline::Str("Heading".into())] },
        Block::HorizontalRule,
        Block::Table(table),
        Block::Figure {
            attr: attr("figure", &["f"], &[]),
            caption: Caption { short: None, blocks: vec![Block::Plain(vec![Inline::Str("figure caption".into())])] },
            blocks: vec![Block::Para(vec![Inline::Str("figure body".into())])],
        },
        Block::Div { attr: attr("div", &["d"], &[("class", "x")]), blocks: vec![Block::Para(vec![Inline::Str("div body".into())])] },
    ];
    let mut meta: Vec<(String, MetaValue)> = vec![
        ("title".into(), MetaValue::Inlines(vec![Inline::Str("标题".into())])),
        ("map".into(), MetaValue::Map(vec![("nested".into(), MetaValue::Bool(true))])),
        ("list".into(), MetaValue::List(vec![MetaValue::String("a".into()), MetaValue::Bool(false)])),
        ("bool".into(), MetaValue::Bool(true)),
        ("string".into(), MetaValue::String("value".into())),
        ("blocks".into(), MetaValue::Blocks(vec![Block::Para(vec![Inline::Str("meta block".into())])])),
    ];
    // JSON object key ordering is not part of the contract; `serde_json` sorts
    // keys on decode, so the fixture starts sorted to compare structurally.
    meta.sort_by(|left, right| left.0.cmp(&right.0));
    PandocDocument { api_version: API_VERSION, meta, blocks }
}

#[test]
fn codec_round_trips_every_constructor() {
    let document = all_constructors_document();
    let text = encode_text(&document);
    assert_eq!(decode_text(&text), document);
    // A second encode must be byte stable, not just structurally equal.
    assert_eq!(encode_text(&decode_text(&text)), text);
}

#[test]
fn codec_preserves_duplicate_attribute_pairs_and_unicode() {
    let document = PandocDocument {
        api_version: API_VERSION,
        meta: vec![("title".into(), MetaValue::String("中文 😀".into()))],
        blocks: vec![Block::CodeBlock(attr("", &[], &[("k", "a"), ("k", "b"), ("k", "c")]), "表情 😀".into())],
    };
    let decoded = decode_text(&encode_text(&document));
    assert_eq!(decoded, document);
    let Block::CodeBlock(decoded_attr, _) = &decoded.blocks[0]
    else {
        panic!("expected a code block");
    };
    assert_eq!(decoded_attr.pairs, vec![("k".into(), "a".into()), ("k".into(), "b".into()), ("k".into(), "c".into())]);
}

#[test]
fn codec_rejects_unknown_constructors_with_a_pointer() {
    let text = r#"{"pandoc-api-version":[1,23,1],"meta":{},"blocks":[{"t":"Bogus","c":[]}]}"#;
    let error = decode(text.as_bytes(), &PandocBudget::default()).expect_err("unknown block must fail");
    let message = format!("{error}");
    assert!(message.contains("/blocks/0"), "{message}");
    assert!(message.contains("unknown block constructor `Bogus`"), "{message}");

    let text = r#"{"pandoc-api-version":[1,23,1],"meta":{},"blocks":[{"t":"Para","c":[{"t":"Bogus"}]}]}"#;
    let error = decode(text.as_bytes(), &PandocBudget::default()).expect_err("unknown inline must fail");
    let message = format!("{error}");
    assert!(message.contains("/blocks/0/c/0"), "{message}");
    assert!(message.contains("unknown inline constructor `Bogus`"), "{message}");
}

#[test]
fn codec_rejects_unsupported_versions_and_top_level_fields() {
    let text = r#"{"pandoc-api-version":[1,22,2],"meta":{},"blocks":[]}"#;
    let error = decode(text.as_bytes(), &PandocBudget::default()).expect_err("version must fail");
    let message = format!("{error}");
    assert!(message.contains("/pandoc-api-version"), "{message}");
    assert!(message.contains("unsupported"), "{message}");
    assert!(message.contains("[1, 23, 1]"), "{message}");

    let text = r#"{"pandoc-api-version":[1,23,1],"meta":{},"blocks":[],"extra":1}"#;
    let error = decode(text.as_bytes(), &PandocBudget::default()).expect_err("extra field must fail");
    assert!(format!("{error}").contains("unknown top-level field `extra`"), "{error}");
}

#[test]
fn codec_rejects_illegal_arity_and_nullary_payloads() {
    let text = r#"{"pandoc-api-version":[1,23,1],"meta":{},"blocks":[{"t":"Header","c":[1,["",[],[]]]}]}"#;
    let error = decode(text.as_bytes(), &PandocBudget::default()).expect_err("short header must fail");
    let message = format!("{error}");
    assert!(message.contains("/blocks/0/c"), "{message}");
    assert!(message.contains("expected arity 3, found 2"), "{message}");

    let text = r#"{"pandoc-api-version":[1,23,1],"meta":{},"blocks":[{"t":"Para","c":[{"t":"Space","c":[]}]}]}"#;
    let error = decode(text.as_bytes(), &PandocBudget::default()).expect_err("nullary payload must fail");
    assert!(format!("{error}").contains("constructor carries no payload"), "{error}");
}

#[test]
fn codec_enforces_byte_depth_and_node_budgets() {
    let document = all_constructors_document();
    let text = encode_text(&document);
    let bytes_budget = PandocBudget { max_bytes: 16, ..PandocBudget::default() };
    let error = decode(text.as_bytes(), &bytes_budget).expect_err("byte budget must fail");
    assert!(format!("{error}").contains("input is"), "{error}");

    let nested = PandocDocument {
        api_version: API_VERSION,
        meta: Vec::new(),
        blocks: vec![Block::BlockQuote(vec![Block::BlockQuote(vec![Block::Para(vec![Inline::Str("x".into())])])])],
    };
    let depth_budget = PandocBudget { max_depth: 1, ..PandocBudget::default() };
    let text = encode_text(&nested);
    let error = decode(text.as_bytes(), &depth_budget).expect_err("depth budget must fail");
    assert!(format!("{error}").contains("nesting depth"), "{error}");
    // Encode enforces the same limit, so an in-memory document cannot walk
    // past `max_depth` just because it never came through decode.
    let error = encode(&nested, &depth_budget).expect_err("encode depth budget must fail");
    assert!(format!("{error}").contains("nesting depth"), "{error}");

    let node_budget = PandocBudget { max_nodes: 3, ..PandocBudget::default() };
    let error = decode(encode_text(&document).as_bytes(), &node_budget).expect_err("node budget must fail");
    assert!(format!("{error}").contains("node count"), "{error}");
}

#[test]
fn codec_refuses_to_encode_an_unknown_version() {
    let document = PandocDocument { api_version: [1, 22, 2], ..PandocDocument::new() };
    let error = encode(&document, &PandocBudget::default()).expect_err("version must fail");
    assert!(format!("{error}").contains("cannot encode unsupported"), "{error}");
}

fn full_fixture() -> String {
    serde_json::json!({
        "pandoc-api-version": [1, 23, 1],
        "meta": {
            "title": {"t": "MetaInlines", "c": [{"t": "Str", "c": "文档"}]},
            "author": {"t": "MetaList", "c": [{"t": "MetaString", "c": "Alice"}]},
            "lang": {"t": "MetaString", "c": "zh"},
            "tags": {"t": "MetaList", "c": [{"t": "MetaString", "c": "a"}, {"t": "MetaString", "c": "b"}]},
            "custom": {"t": "MetaBool", "c": true}
        },
        "blocks": [
            {"t": "Header", "c": [1, ["intro", ["h"], [["k", "v"]]], [{"t": "Str", "c": "Intro"}]]},
            {"t": "Para", "c": [{"t": "Str", "c": "body"}, {"t": "SoftBreak"}, {"t": "Str", "c": "next"}]},
            {"t": "Header", "c": [3, ["", [], []], [{"t": "Str", "c": "Deep"}]]},
            {"t": "Para", "c": [{"t": "Str", "c": "deep body"}]},
            {"t": "Header", "c": [2, ["", [], []], [{"t": "Str", "c": "Second"}]]},
            {"t": "Para", "c": [{"t": "Str", "c": "second body"}]}
        ]
    })
    .to_string()
}

#[test]
fn import_builds_sections_without_inventing_headings_and_preserves_typed_metadata() {
    let graph = import(&full_fixture());
    assert_eq!(graph.metadata.title.as_deref(), Some("文档"));
    assert_eq!(graph.metadata.language.as_deref(), Some("zh"));
    assert_eq!(graph.metadata.authors, vec!["Alice".to_string()]);
    assert_eq!(graph.metadata.tags, vec!["a".to_string(), "b".to_string()]);
    assert_eq!(graph.metadata.properties.len(), 1);
    assert_eq!(graph.metadata.properties[0].key, "custom");
    assert_eq!(graph.metadata.properties[0].value, document_ir::MetadataValue::Boolean(true));
    assert!(!graph.coverage.loss.iter().any(|loss| loss.code == "import.pandoc-json.meta_unmapped"), "{:?}", graph.coverage.loss);

    let sections: Vec<(u8, String, usize)> = graph
        .blocks
        .iter()
        .filter_map(|node| match &node.block {
            IrBlock::Section { level, title, children } => {
                Some((*level, title.iter().map(document_ir::Inline::plain_text).collect::<String>(), children.len()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(sections.len(), 3, "{:?}", graph.blocks);
    // The level-3 heading nests directly under the level-1 section; no level-2
    // heading is synthesized, and its authored level stays 3.
    assert_eq!(sections.iter().map(|section| section.0).collect::<Vec<_>>(), vec![1, 3, 2]);

    let exported = export_pandoc_json(&graph).expect("export");
    let blocks = &exported.document.blocks;
    assert_eq!(blocks.len(), 6, "{:?}", blocks);
    let levels: Vec<i64> = blocks
        .iter()
        .filter_map(|block| match block {
            Block::Header { level, .. } => Some(*level),
            _ => None,
        })
        .collect();
    assert_eq!(levels, vec![1, 3, 2]);
}

#[test]
fn document_without_headings_round_trips_without_sections() {
    let text = serde_json::json!({
        "pandoc-api-version": [1, 23, 1],
        "meta": {},
        "blocks": [
            {"t": "Para", "c": [{"t": "Str", "c": "one"}]},
            {"t": "Para", "c": [{"t": "Str", "c": "two"}]}
        ]
    })
    .to_string();
    let graph = import(&text);
    assert!(graph.blocks.iter().all(|node| !matches!(node.block, IrBlock::Section { .. })));
    let exported = export_pandoc_json(&graph).expect("export");
    let reimported = import(&exported.to_json_text().expect("json"));
    assert_eq!(graph.blocks, reimported.blocks);
}

#[test]
fn ir_semantic_round_trip_is_stable_after_export() {
    let graph = import(&full_fixture());
    let exported = export_pandoc_json(&graph).expect("export");
    let reimported = import(&exported.to_json_text().expect("json"));
    assert_eq!(graph.metadata, reimported.metadata);
    assert_eq!(graph.blocks, reimported.blocks);
}

#[test]
fn pandoc_blockquote_round_trips_quoted_text() {
    let document = PandocDocument {
        api_version: API_VERSION,
        meta: Vec::new(),
        blocks: vec![Block::BlockQuote(vec![Block::Para(vec![Inline::Str("Quoted line".into())])])],
    };
    let graph = import(&encode_text(&document));
    let children = graph
        .blocks
        .iter()
        .find_map(|node| match &node.block {
            IrBlock::Quote { children } => Some(children.as_slice()),
            _ => None,
        })
        .expect("quote expected");
    assert!(children.iter().any(|child_id| matches!(
        graph.block(*child_id).map(|node| &node.block),
        Some(IrBlock::Paragraph { content })
            if content.iter().any(|inline| matches!(inline, IrInline::Text { text } if text == "Quoted line"))
    )));

    let exported = export_pandoc_json(&graph).expect("export");
    let json = exported.to_json_text().expect("json");
    let decoded = decode_text(&json);
    assert!(matches!(decoded.blocks.as_slice(), [Block::BlockQuote(_)]), "{:?}", decoded.blocks);
    let reimported = import(&json);
    assert_eq!(graph.blocks, reimported.blocks);
}

#[test]
fn adapter_round_trips_the_full_constructor_fixture() {
    let text = encode_text(&all_constructors_document());
    let graph = import(&text);
    assert!(graph.coverage.loss.len() >= 5, "{:?}", graph.coverage.loss);
    let exported = export_pandoc_json(&graph).expect("export");
    let reimported = import(&exported.to_json_text().expect("json"));
    assert_eq!(graph.blocks, reimported.blocks);
    assert_eq!(graph.assets, reimported.assets);
}

#[test]
fn strict_policy_rejects_containers_that_only_have_an_opaque_projection() {
    let text = serde_json::json!({
        "pandoc-api-version": [1, 23, 1],
        "meta": {},
        "blocks": [
            {"t": "Div", "c": [["id", ["c"], [["k", "v"]]], [{"t": "Para", "c": [{"t": "Str", "c": "inside"}]}]]}
        ]
    })
    .to_string();
    let error =
        import_pandoc_json_bytes_with("strict", text.as_bytes(), &PandocImportOptions::default()).expect_err("strict policy must reject a div");
    assert!(format!("{error}").contains("import.pandoc-json.div_payload"), "{error}");

    let graph = import(&text);
    assert!(matches!(graph.blocks[0].block, IrBlock::Opaque { .. }));
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "import.pandoc-json.div_payload"));

    let exported = export_pandoc_json(&graph).expect("export");
    assert!(matches!(exported.document.blocks[0], Block::Div { .. }));
    let reimported = import(&exported.to_json_text().expect("json"));
    assert_eq!(graph.blocks, reimported.blocks);
}

#[test]
fn images_are_registered_as_unresolved_assets_not_fetched() {
    let text = serde_json::json!({
        "pandoc-api-version": [1, 23, 1],
        "meta": {},
        "blocks": [
            {"t": "Para", "c": [
                {"t": "Image", "c": [["", [], []], [{"t": "Str", "c": "alt"}], ["https://example.com/a.png", ""]]}
            ]}
        ]
    })
    .to_string();
    let graph = import(&text);
    assert_eq!(graph.assets.len(), 1, "{:?}", graph.assets);
    assert_eq!(graph.assets[0].source.as_deref(), Some("https://example.com/a.png"));
    assert_eq!(graph.assets[0].status, SemanticStatus::Unresolved);
    assert!(graph.assets[0].bytes.is_none());
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "import.pandoc-json.asset_unresolved"));
    assert_eq!(graph.relations.len(), 1, "{:?}", graph.relations);
}

#[test]
fn table_cell_block_content_is_preserved_and_round_trips() {
    let text = serde_json::json!({
        "pandoc-api-version": [1, 23, 1],
        "meta": {},
        "blocks": [
            {"t": "Table", "c": [
                ["", [], []],
                [null, []],
                [["AlignDefault", 0.0]],
                [["", [], []], []],
                [[["", [], []], 0, [], [
                    [["", [], []], [
                        [["", [], []], "AlignDefault", 1, 1, [
                            {"t": "Para", "c": [{"t": "Str", "c": "a"}]},
                            {"t": "Para", "c": [{"t": "Str", "c": "b"}]}
                        ]]
                    ]]
                ]]],
                [["", [], []], []]
            ]}
        ]
    })
    .to_string();
    let graph = import(&text);
    assert!(!graph.coverage.loss.iter().any(|loss| loss.code == "import.pandoc-json.table_cell_blocks"), "{:?}", graph.coverage.loss);
    let body = graph
        .blocks
        .iter()
        .find_map(|node| match &node.block {
            IrBlock::Table { body, .. } => Some(body.as_slice()),
            _ => None,
        })
        .expect("table");
    let cell = &body[0].cells[0];
    assert_eq!(cell.children.len(), 2, "both authored paragraphs stay cell blocks: {cell:?}");
    let texts: Vec<&str> = cell
        .children
        .iter()
        .filter_map(|id| graph.block(*id))
        .filter_map(|node| match &node.block {
            IrBlock::Paragraph { content } => content.iter().find_map(|inline| match inline {
                document_ir::Inline::Text { text } => Some(text.as_str()),
                _ => None,
            }),
            _ => None,
        })
        .collect();
    assert_eq!(texts, ["a", "b"]);
    let exported = export_pandoc_json(&graph).expect("export");
    let reimported = import(&exported.to_json_text().expect("json"));
    assert_eq!(graph.blocks, reimported.blocks);
}

#[test]
fn table_cell_blocks_round_trip_through_markdown_as_an_html_table() {
    let text = serde_json::json!({
        "pandoc-api-version": [1, 23, 1],
        "meta": {},
        "blocks": [
            {"t": "Table", "c": [
                ["", [], []],
                [null, []],
                [["AlignDefault", 0.0]],
                [["", [], []], []],
                [[["", [], []], 0, [], [
                    [["", [], []], [
                        [["", [], []], "AlignDefault", 1, 1, [
                            {"t": "Para", "c": [{"t": "Str", "c": "first"}]},
                            {"t": "BulletList", "c": [[{"t": "Plain", "c": [{"t": "Str", "c": "item"}]}]]}
                        ]]
                    ]]
                ]]],
                [["", [], []], []]
            ]}
        ]
    })
    .to_string();
    let graph = import(&text);
    let markdown = export_markdown(&graph).expect("markdown export");
    assert!(markdown.contains("<table"), "{markdown:?}");
    assert!(markdown.contains("<ul><li>item</li></ul>"), "{markdown:?}");

    let reopened = import_markdown_bytes("cell-blocks.md", &markdown).expect("reopen markdown");
    let exported = export_pandoc_json(&graph).expect("export pandoc json");
    let round_tripped = export_pandoc_json(&reopened).expect("export reopened pandoc json");
    assert_eq!(round_tripped.document.blocks, exported.document.blocks);
}

#[test]
fn task_list_items_are_reported_as_unrepresentable_in_pandoc_json() {
    let mut graph = DocumentGraph::new(document_ir::IdAllocator::default().document_id());
    graph.push_block(IrBlock::List {
        ordered: false,
        items: vec![document_ir::ListItem {
            content: vec![IrInline::Text { text: "task".into() }],
            children: Vec::new(),
            number: None,
            is_task: true,
            is_checked: Some(true),
        }],
    });
    let exported = export_pandoc_json(&graph).expect("export");
    assert!(
        exported.losses.iter().any(|loss| loss.code == "export.pandoc-json.task_list"),
        "the Pandoc JSON AST has no task field, so the checked state must be reported: {:?}",
        exported.losses
    );
}

#[test]
fn plain_lists_do_not_report_task_marker_loss_in_pandoc_json() {
    let mut graph = DocumentGraph::new(document_ir::IdAllocator::default().document_id());
    graph.push_block(IrBlock::List {
        ordered: false,
        items: vec![document_ir::ListItem {
            content: vec![IrInline::Text { text: "item".into() }],
            children: Vec::new(),
            number: None,
            is_task: false,
            is_checked: None,
        }],
    });
    let exported = export_pandoc_json(&graph).expect("export");
    assert!(!exported.losses.iter().any(|loss| loss.code == "export.pandoc-json.task_list"), "{:?}", exported.losses);
}

#[test]
fn export_writes_only_the_three_contract_fields() {
    let graph = import(&full_fixture());
    let exported = export_pandoc_json(&graph).expect("export");
    let text = exported.to_json_text().expect("json");
    let value: serde_json::Value = serde_json::from_str(&text).expect("json value");
    let mut keys: Vec<String> = value.as_object().expect("top-level object").keys().cloned().collect();
    keys.sort();
    // Losses, provenance and Notedown ids stay in the graph report sidecar, so
    // the exchange document carries exactly the Pandoc contract fields.
    assert_eq!(keys, vec!["blocks", "meta", "pandoc-api-version"]);
}

#[test]
fn non_consecutive_ordered_list_numbers_are_reported() {
    let mut graph = DocumentGraph::new(document_ir::IdAllocator::default().document_id());
    graph.push_block(IrBlock::List {
        ordered: true,
        items: vec![
            document_ir::ListItem {
                content: vec![document_ir::Inline::Text { text: "one".into() }],
                number: Some(1),
                ..document_ir::ListItem::default()
            },
            document_ir::ListItem {
                content: vec![document_ir::Inline::Text { text: "three".into() }],
                number: Some(3),
                ..document_ir::ListItem::default()
            },
        ],
    });
    let exported = export_pandoc_json(&graph).expect("export");
    assert!(exported.losses.iter().any(|loss| loss.code == "export.pandoc-json.list_number"), "{}", exported.to_json_text().expect("json"));
    let Block::OrderedList(list) = &exported.document.blocks[0]
    else {
        panic!("expected an ordered list");
    };
    assert_eq!(list.start, 1);
    assert_eq!(list.items.len(), 2);
}

#[test]
fn images_nested_in_links_and_notes_register_assets() {
    let text = serde_json::json!({
        "pandoc-api-version": [1, 23, 1],
        "meta": {},
        "blocks": [{"t": "Para", "c": [
            {"t": "Link", "c": [
                ["", [], []],
                [{"t": "Image", "c": [["", [], []], [{"t": "Str", "c": "alt"}], ["linked.png", ""]]}],
                ["https://example.com", ""]
            ]},
            {"t": "Note", "c": [{"t": "Para", "c": [
                {"t": "Image", "c": [["", [], []], [{"t": "Str", "c": "alt"}], ["note.png", ""]]}
            ]}]}
        ]}]
    })
    .to_string();
    let graph = import(&text);
    let sources: Vec<&str> = graph.assets.iter().filter_map(|asset| asset.source.as_deref()).collect();
    assert_eq!(sources, ["linked.png", "note.png"], "{:?}", graph.assets);
    assert_eq!(graph.coverage.loss.iter().filter(|loss| loss.code == "import.pandoc-json.asset_unresolved").count(), 2);
}

#[test]
fn pandoc_note_with_an_image_survives_a_markdown_round_trip() {
    let document = PandocDocument {
        api_version: API_VERSION,
        meta: Vec::new(),
        blocks: vec![Block::Para(vec![
            Inline::Str("See".into()),
            Inline::Note(vec![Block::Para(vec![
                Inline::Str("picture ".into()),
                Inline::Image {
                    attr: Attr::default(),
                    inlines: vec![Inline::Str("alt".into())],
                    target: Target { url: "pic.png".into(), title: String::new() },
                },
            ])]),
        ])],
    };
    let graph = import(&encode_text(&document));
    let markdown = export_markdown(&graph).expect("markdown export");
    assert!(markdown.contains("![alt](pic.png)"), "{markdown:?}");

    let reopened = import_markdown_bytes("note-image.md", &markdown).expect("reopen markdown");
    let exported = export_pandoc_json(&reopened).expect("export pandoc json");
    assert!(!exported.losses.iter().any(|loss| loss.code.starts_with("export.pandoc-json.note_definition")), "{:?}", exported.losses);
    assert!(footnote_definition_payloads(&exported.document.blocks).is_empty(), "{:?}", exported.document.blocks);
    let notes = paragraph_notes(&exported.document.blocks);
    assert_eq!(notes.len(), 1, "{:?}", exported.document.blocks);
    let note_body = &notes[0];
    let images: Vec<&str> = note_body
        .iter()
        .filter_map(|block| match block {
            Block::Para(inlines) => Some(inlines),
            _ => None,
        })
        .flat_map(|inlines| inlines.iter())
        .filter_map(|inline| match inline {
            Inline::Image { target, .. } => Some(target.url.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(images, ["pic.png"], "{note_body:?}");
}

#[test]
fn unrelated_custom_inline_style_fails_the_export() {
    let mut graph = DocumentGraph::new(document_ir::IdAllocator::default().document_id());
    graph.push_block(IrBlock::Paragraph { content: vec![document_ir::Inline::Styled { style: "sparkle".into(), children: vec![] }] });
    let error = export_pandoc_json(&graph).expect_err("unknown style must fail");
    assert!(format!("{error}").contains("inline style `sparkle`"), "{error}");
}

#[test]
fn export_reemits_typed_metadata_entries() {
    let graph = import(&full_fixture());
    let exported = export_pandoc_json(&graph).expect("export");
    let custom = exported.document.meta.iter().find(|(key, _)| key == "custom").expect("custom metadata");
    assert!(matches!(custom.1, MetaValue::Bool(true)));

    let restored = import(&exported.to_json_text().expect("json"));
    assert_eq!(restored.metadata.properties[0].key, "custom");
    assert_eq!(restored.metadata.properties[0].value, document_ir::MetadataValue::Boolean(true));
}

#[test]
fn export_writes_numeric_metadata_as_strings_and_reports_loss() {
    let mut graph = DocumentGraph::new(document_ir::IdAllocator::default().document_id());
    graph.metadata.properties = vec![
        document_ir::MetadataEntry { key: "edition".into(), value: document_ir::MetadataValue::Integer(3) },
        document_ir::MetadataEntry { key: "ratio".into(), value: document_ir::MetadataValue::Float(0.25) },
    ];

    let exported = export_pandoc_json(&graph).expect("export");
    assert!(matches!(&exported.document.meta[0].1, MetaValue::String(value) if value == "3"));
    assert!(matches!(&exported.document.meta[1].1, MetaValue::String(value) if value == "0.25"));
    assert!(exported.losses.iter().any(|loss| loss.code == "export.pandoc-json.metadata_number"), "{:?}", exported.losses);
}

fn note_labels(graph: &DocumentGraph) -> Vec<String> {
    graph
        .blocks
        .iter()
        .filter_map(|node| match &node.block {
            IrBlock::Paragraph { content } => Some(
                content
                    .iter()
                    .filter_map(|inline| match inline {
                        IrInline::Note { label, .. } => Some(label.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .flatten()
        .collect()
}

#[test]
fn pandoc_notes_receive_deterministic_distinct_labels() {
    let document = PandocDocument {
        api_version: API_VERSION,
        meta: Vec::new(),
        blocks: vec![Block::Para(vec![
            Inline::Str("first".into()),
            Inline::Note(vec![Block::Para(vec![Inline::Str("body one".into())])]),
            Inline::Str(" second".into()),
            Inline::Note(vec![Block::Para(vec![Inline::Str("body two".into())])]),
        ])],
    };
    let text = encode_text(&document);
    let graph = import(&text);
    assert!(graph.validate().is_valid(), "{:?}", graph.validate().issues);
    assert_eq!(note_labels(&graph), ["pandoc-note-1", "pandoc-note-2"]);
    assert_eq!(note_labels(&import(&text)), ["pandoc-note-1", "pandoc-note-2"], "labels must not depend on import order or clock");
}

#[test]
fn pandoc_citations_without_a_key_are_dropped_with_a_loss() {
    let document = PandocDocument {
        api_version: API_VERSION,
        meta: Vec::new(),
        blocks: vec![Block::Para(vec![
            Inline::Str("before ".into()),
            Inline::Cite {
                citations: vec![Citation {
                    id: "  ".into(),
                    prefix: Vec::new(),
                    suffix: Vec::new(),
                    mode: CitationMode::Normal,
                    note_num: 0,
                    hash: 0,
                }],
                inlines: vec![Inline::Str("[?]".into())],
            },
            Inline::Str("after".into()),
        ])],
    };
    let graph = import(&encode_text(&document));
    let content = graph
        .blocks
        .iter()
        .find_map(|node| match &node.block {
            IrBlock::Paragraph { content } => Some(content.clone()),
            _ => None,
        })
        .expect("paragraph");
    assert_eq!(content.iter().map(document_ir::Inline::plain_text).collect::<String>(), "before after");
    assert!(!content.iter().any(|inline| matches!(inline, IrInline::Citation { .. })));
    let loss =
        graph.coverage.loss.iter().find(|loss| loss.code == "import.pandoc-json.empty_citation").expect("a keyless citation must be recorded");
    assert_eq!(loss.status, SemanticStatus::Unsupported);
    assert!(graph.validate().is_valid());
}

#[test]
fn pandoc_citation_display_distinguishes_absent_from_empty() {
    let citation =
        Citation { id: "smith2024".into(), prefix: Vec::new(), suffix: Vec::new(), mode: CitationMode::Normal, note_num: 0, hash: 0 };
    let document = PandocDocument {
        api_version: API_VERSION,
        meta: Vec::new(),
        blocks: vec![Block::Para(vec![
            Inline::Cite { citations: vec![citation.clone()], inlines: Vec::new() },
            Inline::Str(" ".into()),
            Inline::Cite { citations: vec![citation], inlines: vec![Inline::Str("(Smith 2024)".into())] },
        ])],
    };
    let graph = import(&encode_text(&document));
    let IrBlock::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("paragraph expected: {:?}", graph.blocks)
    };
    assert!(
        matches!(&content[0], IrInline::Citation { key, display: None } if key == "smith2024"),
        "an absent display must stay `None`, not become an empty run: {content:#?}"
    );
    assert!(
        matches!(&content[2], IrInline::Citation { key, display: Some(display) }
            if key == "smith2024" && display == &vec![IrInline::Text { text: "(Smith 2024)".into() }]),
        "an authored display must survive: {content:#?}"
    );
}

fn footnote_definition_payloads(blocks: &[Block]) -> Vec<&str> {
    blocks
        .iter()
        .filter_map(|block| match block {
            Block::RawBlock(format, payload) if format == "notedown.footnote_definition" => Some(payload.as_str()),
            _ => None,
        })
        .collect()
}

fn paragraph_notes(blocks: &[Block]) -> Vec<Vec<Block>> {
    blocks
        .iter()
        .filter_map(|block| match block {
            Block::Para(inlines) => Some(inlines.as_slice()),
            _ => None,
        })
        .flat_map(|inlines| {
            inlines.iter().filter_map(|inline| match inline {
                Inline::Note(blocks) => Some(blocks.clone()),
                _ => None,
            })
        })
        .collect()
}

fn pandoc_inline_text(inlines: &[Inline]) -> String {
    inlines
        .iter()
        .map(|inline| match inline {
            Inline::Str(text) => text.clone(),
            Inline::Space | Inline::SoftBreak => " ".into(),
            Inline::Emph(inner)
            | Inline::Strong(inner)
            | Inline::Strikeout(inner)
            | Inline::Underline(inner)
            | Inline::Superscript(inner)
            | Inline::Subscript(inner)
            | Inline::SmallCaps(inner) => pandoc_inline_text(inner),
            Inline::Span { inlines, .. } => pandoc_inline_text(inlines),
            Inline::Code { text, .. } => text.clone(),
            Inline::Quoted { inlines, .. } | Inline::Link { inlines, .. } | Inline::Image { inlines, .. } => pandoc_inline_text(inlines),
            Inline::Cite { inlines, .. } => pandoc_inline_text(inlines),
            _ => String::new(),
        })
        .collect()
}

#[test]
fn markdown_footnote_definition_becomes_a_native_note() {
    let markdown = "Text[^a].\n\n[^a]: body with *emphasis* and `code`\n";
    let graph = import_markdown_bytes("note.md", markdown).expect("markdown import");
    assert!(graph.validate().is_valid(), "{:?}", graph.validate().issues);

    let exported = export_pandoc_json(&graph).expect("export");
    assert!(!exported.losses.iter().any(|loss| loss.code.starts_with("export.pandoc-json.note_definition")), "{:?}", exported.losses);
    // The consumed definition must not linger as a raw opaque block.
    assert!(footnote_definition_payloads(&exported.document.blocks).is_empty(), "{:?}", exported.document.blocks);

    let notes = paragraph_notes(&exported.document.blocks);
    assert_eq!(notes.len(), 1, "{:?}", exported.document.blocks);
    let [Block::Para(body)] = notes[0].as_slice()
    else {
        panic!("a rebuilt note must hold exactly one paragraph: {:?}", notes[0]);
    };
    assert_eq!(pandoc_inline_text(body), "body with emphasis and code");
    assert!(matches!(body.as_slice(), [Inline::Str(_), Inline::Emph(_), Inline::Str(_), Inline::Code { .. }]), "{body:?}");
}

#[test]
fn multiline_footnote_definition_stays_opaque_with_a_precise_reason() {
    let markdown = "Text[^a].\n\n[^a]: first line\n    second line\n";
    let graph = import_markdown_bytes("note.md", markdown).expect("markdown import");
    let payload = graph
        .blocks
        .iter()
        .find_map(|node| match &node.block {
            IrBlock::Opaque { kind, payload_hint, .. } if kind == "footnote_definition" => Some(payload_hint.clone()),
            _ => None,
        })
        .expect("opaque definition");
    assert!(payload.contains('\n'), "{payload:?}");

    let exported = export_pandoc_json(&graph).expect("export");
    assert!(exported.losses.iter().any(|loss| loss.code == "export.pandoc-json.note_definition_multiblock"), "{:?}", exported.losses);
    assert_eq!(footnote_definition_payloads(&exported.document.blocks), vec![payload.as_str()]);
    assert!(paragraph_notes(&exported.document.blocks).iter().all(|note| note.is_empty()));
}

#[test]
fn nested_footnote_reference_stays_opaque_with_a_precise_reason() {
    let markdown = "Text[^a].\n\n[^a]: body with [^b]\n";
    let graph = import_markdown_bytes("note.md", markdown).expect("markdown import");
    let exported = export_pandoc_json(&graph).expect("export");
    assert!(exported.losses.iter().any(|loss| loss.code == "export.pandoc-json.note_definition_nested"), "{:?}", exported.losses);
    assert_eq!(footnote_definition_payloads(&exported.document.blocks), vec!["[^a]: body with [^b]"]);
    assert!(paragraph_notes(&exported.document.blocks).iter().all(|note| note.is_empty()));
}

#[test]
fn missing_footnote_definition_reports_a_missing_reason() {
    let graph = import_markdown_bytes("note.md", "Text[^a].\n").expect("markdown import");
    let exported = export_pandoc_json(&graph).expect("export");
    assert!(exported.losses.iter().any(|loss| loss.code == "export.pandoc-json.note_definition_missing"), "{:?}", exported.losses);
    assert!(footnote_definition_payloads(&exported.document.blocks).is_empty());
    assert!(paragraph_notes(&exported.document.blocks).iter().all(|note| note.is_empty()));
}

#[test]
fn empty_footnote_definition_reports_an_empty_reason() {
    let graph = import_markdown_bytes("note.md", "Text[^a].\n\n[^a]:\n").expect("markdown import");
    let exported = export_pandoc_json(&graph).expect("export");
    assert!(exported.losses.iter().any(|loss| loss.code == "export.pandoc-json.note_definition_empty"), "{:?}", exported.losses);
}

#[test]
fn block_level_footnote_body_reports_a_block_level_reason() {
    let markdown = "Text[^a].\n\n[^a]: - list item\n";
    let graph = import_markdown_bytes("note.md", markdown).expect("markdown import");
    let exported = export_pandoc_json(&graph).expect("export");
    assert!(exported.losses.iter().any(|loss| loss.code == "export.pandoc-json.note_definition_block_level"), "{:?}", exported.losses);
    assert_eq!(footnote_definition_payloads(&exported.document.blocks), vec!["[^a]: - list item"]);
}

#[test]
fn footnote_body_with_an_image_is_rebuilt_as_a_native_note() {
    let markdown = "Text[^a].\n\n[^a]: see ![alt](pic.png)\n";
    let graph = import_markdown_bytes("note.md", markdown).expect("markdown import");
    let exported = export_pandoc_json(&graph).expect("export");
    assert!(!exported.losses.iter().any(|loss| loss.code.starts_with("export.pandoc-json.note_definition")), "{:?}", exported.losses);
    assert!(footnote_definition_payloads(&exported.document.blocks).is_empty());
    let notes = paragraph_notes(&exported.document.blocks);
    assert_eq!(notes.len(), 1);
    let images: Vec<&str> = notes[0]
        .iter()
        .filter_map(|block| match block {
            Block::Para(inlines) => Some(inlines),
            _ => None,
        })
        .flat_map(|inlines| inlines.iter())
        .filter_map(|inline| match inline {
            Inline::Image { target, .. } => Some(target.url.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(images, ["pic.png"], "{:?}", notes[0]);
}

#[test]
fn lossy_footnote_body_stays_opaque_with_an_unrepresentable_reason() {
    let markdown = "Text[^a].\n\n[^a]: [text][missing]\n";
    let graph = import_markdown_bytes("note.md", markdown).expect("markdown import");
    let exported = export_pandoc_json(&graph).expect("export");
    assert!(exported.losses.iter().any(|loss| loss.code == "export.pandoc-json.note_definition_unrepresentable"), "{:?}", exported.losses);
    assert_eq!(footnote_definition_payloads(&exported.document.blocks), vec!["[^a]: [text][missing]"]);
}

#[test]
fn markdown_footnote_body_round_trips_through_pandoc_json_and_back() {
    let markdown = "Text[^a].\n\n[^a]: body with *emphasis* and `code`\n";
    let graph = import_markdown_bytes("note.md", markdown).expect("markdown import");
    let exported = export_pandoc_json(&graph).expect("export");
    let first = exported.document.clone();
    let json = exported.to_json_text().expect("json");

    let pandoc_graph = import(&json);
    assert_eq!(note_labels(&pandoc_graph), ["pandoc-note-1"]);
    let again = export_pandoc_json(&pandoc_graph).expect("export");
    // Both content and Pandoc structure survive the second pass.
    assert_eq!(again.document.blocks, first.blocks);

    let back = export_markdown(&pandoc_graph).expect("markdown export");
    assert!(back.contains("[^pandoc-note-1]"), "{back:?}");
    assert!(back.contains("[^pandoc-note-1]: body with *emphasis* and `code`"), "{back:?}");
    let reopened = import_markdown_bytes("round.md", &back).expect("reopen markdown");
    assert_eq!(note_labels(&reopened), ["pandoc-note-1"]);
}

#[test]
fn pandoc_note_body_survives_a_markdown_round_trip() {
    let document = PandocDocument {
        api_version: API_VERSION,
        meta: Vec::new(),
        blocks: vec![Block::Para(vec![Inline::Str("See".into()), Inline::Note(vec![Block::Para(vec![Inline::Str("body one".into())])])])],
    };
    let graph = import(&encode_text(&document));
    let markdown = export_markdown(&graph).expect("a pandoc note body must have a Markdown footnote spelling");
    assert!(markdown.contains("[^pandoc-note-1]"), "{markdown:?}");
    assert!(markdown.contains("[^pandoc-note-1]: body one"), "{markdown:?}");

    let reopened = import_markdown_bytes("pandoc-note-round.md", &markdown).expect("reopen markdown");
    let exported = export_pandoc_json(&reopened).expect("export pandoc json");
    let json = encode_text(&exported.document);
    assert!(json.contains("body one"), "the note body must survive pandoc-json -> markdown -> pandoc-json: {json}");
}
