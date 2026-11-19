//! Container import regressions: HTML footnote endnotes, inline style/image
//! sizes, structured tables, and EPUB ordered-list numbering with fail-closed
//! CSS diagnostics.

#![cfg(all(feature = "html", feature = "epub", feature = "markdown"))]

use std::path::PathBuf;

use document_formats::{
    export::{html::export_html, markdown::export_markdown},
    import::{
        epub::import_epub_bytes,
        html::{import_html_bytes, import_html_text},
        markdown::import_markdown_bytes,
    },
};
use document_ir::{Block, DocumentGraph, Inline, SemanticStatus};

fn fixture(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(path)
}

fn read_fixture(path: &str) -> Vec<u8> {
    std::fs::read(fixture(path)).unwrap_or_else(|error| panic!("read fixture `{path}`: {error}"))
}

fn import_fixture_html(path: &str) -> DocumentGraph {
    import_html_bytes(path, &read_fixture(&format!("html/{path}"))).expect("html import")
}

fn loss_codes(graph: &DocumentGraph) -> Vec<String> {
    graph.coverage.loss.iter().map(|loss| loss.code.clone()).collect()
}

fn loss(graph: &DocumentGraph, code: &str) -> SemanticStatus {
    graph
        .coverage
        .loss
        .iter()
        .find(|loss| loss.code == code)
        .unwrap_or_else(|| panic!("missing loss `{code}` in {:?}", loss_codes(graph)))
        .status
}

fn footnote_definitions(graph: &DocumentGraph) -> Vec<&str> {
    graph
        .blocks
        .iter()
        .filter_map(|node| match &node.block {
            Block::Opaque { kind, payload_hint, .. } if kind == "footnote_definition" => Some(payload_hint.as_str()),
            _ => None,
        })
        .collect()
}

/// Paragraph count of each authored definition body. The payload keeps later
/// paragraphs as GFM continuation lines, so a blank line plus four spaces is a
/// paragraph boundary.
fn footnote_paragraph_counts(graph: &DocumentGraph) -> Vec<usize> {
    footnote_definitions(graph)
        .into_iter()
        .map(|payload| {
            let body = payload.split_once("]:").map(|(_, body)| body.trim()).unwrap_or_default();
            if body.is_empty() { 0 } else { body.matches("\n\n    ").count() + 1 }
        })
        .collect()
}

fn note_labels(graph: &DocumentGraph) -> Vec<String> {
    fn scan(inlines: &[Inline], labels: &mut Vec<String>) {
        for inline in inlines {
            match inline {
                Inline::Note { label, .. } => labels.push(label.clone()),
                Inline::Styled { children, .. } | Inline::Attributed { children, .. } => scan(children, labels),
                Inline::Link { display, .. } => scan(display, labels),
                _ => {}
            }
        }
    }
    let mut labels = Vec::new();
    for node in &graph.blocks {
        match &node.block {
            Block::Paragraph { content } => scan(content, &mut labels),
            Block::Section { title, .. } => scan(title, &mut labels),
            Block::List { items, .. } => {
                for item in items {
                    scan(&item.content, &mut labels);
                }
            }
            // Table cell bodies are separate blocks in the same graph, so their
            // paragraphs are scanned by their own arm.
            _ => {}
        }
    }
    labels
}

fn has_attributed_class(graph: &DocumentGraph, class: &str) -> bool {
    fn scan(inlines: &[Inline], class: &str) -> bool {
        inlines.iter().any(|inline| match inline {
            Inline::Attributed { attributes, children } => attributes.classes.iter().any(|token| token == class) || scan(children, class),
            Inline::Styled { children, .. } => scan(children, class),
            Inline::Link { display, .. } => scan(display, class),
            _ => false,
        })
    }
    graph.blocks.iter().any(|node| match &node.block {
        Block::Paragraph { content } => scan(content, class),
        Block::Section { title, .. } => scan(title, class),
        Block::List { items, .. } => items.iter().any(|item| scan(&item.content, class)),
        // Cell paragraphs are their own blocks, so they are scanned separately.
        _ => false,
    })
}

fn collect_blocks<'a>(graph: &'a DocumentGraph, filter: &dyn Fn(&Block) -> bool) -> Vec<&'a Block> {
    graph.blocks.iter().map(|node| &node.block).filter(|block| filter(*block)).collect()
}

fn ordered_list_starting_with<'a>(graph: &'a DocumentGraph, marker: &str) -> &'a Block {
    graph
        .blocks
        .iter()
        .map(|node| &node.block)
        .find(|block| {
            matches!(block, Block::List { ordered: true, items }
                if matches!(items.first().map(|item| item.content.as_slice()), Some([Inline::Text { text }]) if text == marker))
        })
        .unwrap_or_else(|| panic!("ordered list starting with `{marker}`: {:?}", graph.blocks))
}

fn item_numbers(list: &Block) -> Vec<Option<u32>> {
    let Block::List { items, .. } = list
    else {
        panic!("not a list: {list:?}")
    };
    items.iter().map(|item| item.number).collect()
}

/// A semantic `<table>` lowers to `Block::Table`; the raw markup never survives
/// as `Block::RawHtml`.
#[test]
fn html_table_lowers_to_a_structured_table() {
    let graph = import_fixture_html("table.html");
    let tables = collect_blocks(&graph, &|block| matches!(block, Block::Table { .. }));
    assert_eq!(tables.len(), 1, "{:?}", collect_blocks(&graph, &|_| true));
    let Block::Table { caption, head, body, .. } = tables[0]
    else {
        unreachable!()
    };
    assert_eq!(caption.as_deref(), Some("Table 1. Coverage by region."), "{caption:?}");
    assert_eq!(head.len(), 2, "{head:?}");
    assert_eq!(body.len(), 2, "{body:?}");
    assert!(head.iter().chain(body).all(|row| row.cells.iter().all(|cell| cell.colspan == 1 && cell.rowspan == 1)));
    assert_eq!(loss(&graph, "import.html.table_spans"), SemanticStatus::Partial, "{:?}", loss_codes(&graph));
    let raw_tables: Vec<&Block> =
        collect_blocks(&graph, &|block| matches!(block, Block::RawHtml { html } if html.to_ascii_lowercase().contains("<table")));
    assert!(raw_tables.is_empty(), "table stayed raw: {raw_tables:?}");
    let markdown = export_markdown(&graph).expect("markdown export");
    // A caption has no GFM pipe slot, so the table takes the inline HTML path
    // instead of dropping the caption as before. The fallback still carries
    // `<caption>` and lowers back to a structured table.
    assert!(markdown.contains("<caption>Table 1. Coverage by region.</caption>"), "{markdown}");
    let reopened = import_markdown_bytes("table-roundtrip.md", &markdown).expect("markdown reimport");
    let reopened_tables = collect_blocks(&reopened, &|block| matches!(block, Block::Table { .. }));
    assert_eq!(reopened_tables.len(), 1, "{:?}", collect_blocks(&reopened, &|_| true));
    let Block::Table { caption: reopened_caption, head: reopened_head, body: reopened_body, .. } = reopened_tables[0]
    else {
        unreachable!()
    };
    assert_eq!(reopened_caption.as_deref(), Some("Table 1. Coverage by region."), "{reopened_caption:?}");
    assert_eq!(reopened_head.len(), 2, "{reopened_head:?}");
    assert_eq!(reopened_body.len(), 2, "{reopened_body:?}");
}

/// Endnotes trailing the last section, including inside `<footer>`/`<aside>`,
/// are definitions rather than inline content that leaks `id`/`class`.
#[test]
fn html_endnotes_after_the_last_section_stay_definitions() {
    for fixture_name in ["div-footnotes.html", "early-footnotes.html"] {
        let graph = import_fixture_html(fixture_name);
        assert_eq!(footnote_definitions(&graph).len(), 2, "{fixture_name}: {:?}", graph.blocks);
        assert!(!has_attributed_class(&graph, "footnotes"), "{fixture_name}: endnotes class leaked into inlines");
        let markdown = export_markdown(&graph).expect("markdown export");
        assert!(markdown.contains("[^1]"), "{fixture_name}: {markdown}");
        assert!(markdown.contains("[^2]"), "{fixture_name}: {markdown}");
        assert!(markdown.contains("[^1]: Note one."), "{fixture_name}: {markdown}");
        assert!(markdown.contains("[^2]: Note two."), "{fixture_name}: {markdown}");
    }
}

/// Both authored reference shapes and the `[^label]` text collapse to a note
/// with no custom display, so the Markdown writer emits a real reference.
#[test]
fn html_footnote_reference_shapes_collapse_to_notes() {
    let cases = [
        (
            "exporter-sup",
            r##"<html><body><p>Body<sup class="footnote-reference" data-footnote-label="1">1</sup>.</p><section class="footnotes" role="doc-endnotes"><ol><li id="fn-1"><p>one</p></li></ol></section></body></html>"##,
        ),
        (
            "gfm-anchor",
            r##"<html><body><p>Body<a class="footnote-ref" href="#fn-1">[^1]</a>.</p><div class="footnotes"><ol><li id="fn-1">one</li></ol></div></body></html>"##,
        ),
        (
            "sup-anchor",
            r##"<html><body><p>Body<sup class="footnote-reference" data-footnote-label="1"><a href="#fn-1">[^1]</a></sup>.</p><div class="footnotes"><ol><li id="fn-1">one</li></ol></div></body></html>"##,
        ),
    ];
    for (name, source) in cases {
        let graph = import_html_text(name, source).expect("html import");
        let marked = graph.blocks.iter().any(|node| match &node.block {
            Block::Paragraph { content } => content.iter().any(|inline| matches!(inline, Inline::Note { display: None, .. })),
            _ => false,
        });
        assert!(marked, "{name}: {:?}", graph.blocks);
        let markdown = export_markdown(&graph).expect("markdown export");
        assert!(markdown.contains("Body[^1]."), "{name}: {markdown}");
    }
}

/// Inline `style` attributes keep the element text and record the downgrade.
#[test]
fn html_inline_style_is_a_recorded_loss() {
    let graph = import_fixture_html("inline-style.html");
    assert_eq!(loss(&graph, "import.html.unsupported_style"), SemanticStatus::Partial, "{:?}", loss_codes(&graph));
    let markdown = export_markdown(&graph).expect("markdown export");
    assert!(markdown.contains("Red text"), "{markdown}");
}

/// `<img width>`/`<height>` stay on the image element through the IR and reach
/// the Markdown projection as valid `<img>` attributes, never as a `<span>`.
#[test]
fn html_image_size_survives_as_img_attributes() {
    let graph = import_fixture_html("img-size.html");
    assert!(
        !loss_codes(&graph).iter().any(|code| code == "import.html.image_dimensions"),
        "a display size now has a slot: {:?}",
        loss_codes(&graph)
    );
    let markdown = export_markdown(&graph).expect("markdown export");
    assert!(markdown.contains(r#"<img src="chart.png" alt="chart" width="320" height="180">"#), "{markdown}");
    assert!(!markdown.contains("<span"), "the size belongs on the image element: {markdown}");

    let html = export_html(&graph).expect("html export");
    assert!(
        html.contains(r#"<img alt="chart" src="chart.png" width="320" height="180">"#),
        "the HTML projection keeps the size on the image: {html}"
    );
    assert!(!html.contains("<span width"), "the HTML projection must not invent a sized span: {html}");
}

/// Ordered EPUB list items carry strictly increasing numbers, including
/// `<ol start>` and nested lists, so the graph validates.
#[test]
fn epub_ordered_list_items_carry_numbers() {
    let graph = import_epub_bytes("ordered-list.epub", &epub_fixture("ordered-list")).expect("epub import");
    assert!(graph.validate().is_valid(), "{:?}", graph.validate().issues);
    let lists: Vec<&Block> = collect_blocks(&graph, &|block| matches!(block, Block::List { ordered: true, .. }));
    let main = lists
        .iter()
        .find(|block| {
            matches!(block, Block::List { items, .. }
                if matches!(items.first().map(|item| item.content.as_slice()), Some([Inline::Text { text }]) if text == "one"))
        })
        .copied()
        .unwrap_or_else(|| panic!("ordered list starting with `one`: {:?}", graph.blocks));
    let Block::List { items, .. } = main
    else {
        unreachable!()
    };
    assert_eq!(items.iter().map(|item| item.number).collect::<Vec<_>>(), vec![Some(1), Some(2)]);
    let started = lists
        .iter()
        .find(|block| {
            matches!(block, Block::List { items, .. }
                if matches!(items.first().map(|item| item.content.as_slice()), Some([Inline::Text { text }]) if text == "three"))
        })
        .copied()
        .unwrap_or_else(|| panic!("ordered list starting with `three`: {:?}", graph.blocks));
    let Block::List { items, .. } = started
    else {
        unreachable!()
    };
    assert_eq!(items[0].number, Some(3), "`start=\"3\"` must shift the sequence");
    assert_eq!(items[1].number, Some(4));
    let nested = items[1]
        .children
        .iter()
        .filter_map(|id| graph.block(*id))
        .find(|node| matches!(node.block, Block::List { .. }))
        .expect("nested list block");
    let Block::List { ordered: nested_ordered, items: nested_items } = &nested.block
    else {
        unreachable!()
    };
    assert!(!nested_ordered, "the nested unordered list stays unordered: {:?}", nested.block);
    assert!(nested_items.iter().all(|item| item.number.is_none()), "{nested_items:?}");
}

/// Linked and embedded CSS stay fail-closed with an explicit diagnostic and
/// still keep the chapter text.
#[test]
fn epub_css_stylesheets_stay_fail_closed_with_diagnostics() {
    let linked = import_epub_bytes("css-link.epub", &epub_fixture("css-link")).expect("epub import");
    assert_eq!(loss(&linked, "import.epub.css_link_unsupported"), SemanticStatus::Unsupported);
    let markdown = export_markdown(&linked).expect("markdown export");
    assert!(markdown.contains("Styled paragraph."), "{markdown}");

    let embedded = import_epub_bytes("inline-style.epub", &epub_fixture("inline-style")).expect("epub import");
    assert_eq!(loss(&embedded, "import.epub.css_inline_block_unsupported"), SemanticStatus::Unsupported);
    let markdown = export_markdown(&embedded).expect("markdown export");
    assert!(markdown.contains("Styled paragraph."), "{markdown}");
}

/// `<li value>` shifts the ordinal while it stays increasing. A backward reset
/// and `<ol reversed>` cannot fit the IR invariant, so both keep the list valid
/// and record an explicit loss.
#[test]
fn epub_list_value_and_reversed_are_explicit() {
    let graph = import_epub_bytes("list-value-reversed.epub", &epub_fixture("list-value-reversed")).expect("epub import");
    assert!(graph.validate().is_valid(), "{:?}", graph.validate().issues);

    let shifted = ordered_list_starting_with(&graph, "two");
    assert_eq!(item_numbers(shifted), vec![Some(2), Some(5), Some(6)], "`value=\"5\"` must move the run");

    let reset = ordered_list_starting_with(&graph, "one");
    assert_eq!(item_numbers(reset), vec![Some(1), Some(2)], "a backward `value` keeps the running sequence");
    assert_eq!(loss(&graph, "import.epub.list_item_value_unsupported"), SemanticStatus::Partial);

    let reversed = ordered_list_starting_with(&graph, "a");
    assert_eq!(item_numbers(reversed), vec![Some(1), Some(2)], "reversed lists fall back to ascending ordinals");
    assert_eq!(loss(&graph, "import.epub.list_reversed_unsupported"), SemanticStatus::Partial);
}

/// `<a epub:type="noteref">` and `<aside epub:type="footnote">` lower to the
/// same semantic shapes as the HTML endnotes path, so the Markdown writer can
/// emit real `[^label]` references and definitions. Text node edge spaces stay
/// visible across the reference boundaries and return links stay out of the
/// definition bodies.
#[test]
fn epub_structural_footnotes_become_notes_and_definitions() {
    let graph = import_epub_bytes("footnotes.epub", &epub_fixture("footnotes")).expect("epub import");
    assert!(graph.validate().is_valid(), "{:?}", graph.validate().issues);
    assert_eq!(note_labels(&graph), vec!["1".to_string(), "2".to_string(), "3".to_string()], "{:?}", graph.blocks);
    assert_eq!(
        footnote_definitions(&graph),
        vec!["[^1]: Single paragraph note.", "[^2]: First paragraph.\n\n    Second paragraph.", "[^3]: Inline bold, code, and link.",]
    );
    assert_eq!(footnote_paragraph_counts(&graph), vec![1, 2, 1], "{:?}", footnote_definitions(&graph));
    assert_eq!(loss(&graph, "import.epub.footnote_inline_markup_flattened"), SemanticStatus::Partial, "{:?}", loss_codes(&graph));
    assert!(!loss_codes(&graph).iter().any(|code| code == "import.epub.unsupported_xhtml_block"), "{:?}", loss_codes(&graph));
    let markdown = export_markdown(&graph).expect("markdown export");
    assert!(markdown.contains("Body with a single note[^1], a note with two paragraphs[^2], and a note with inline markup[^3]."), "{markdown}");
    assert!(!markdown.contains('↩'), "return glyph leaked into the definitions: {markdown}");
    assert!(!markdown.contains("link.return"), "return copy leaked into the definitions: {markdown}");
    assert!(markdown.contains("[^1]: Single paragraph note."), "{markdown}");
    assert!(markdown.contains("[^2]: First paragraph.\n\n    Second paragraph."), "{markdown}");
    assert!(markdown.contains("[^3]: Inline bold, code, and link."), "{markdown}");
}

/// A reference without a definition keeps its note label. Duplicate asides
/// stay explicit definition blocks instead of being merged or dropped.
#[test]
fn epub_footnote_reference_without_definition_stays_a_note() {
    let graph = import_epub_bytes("footnotes-edge.epub", &epub_fixture("footnotes-edge")).expect("epub import");
    assert!(graph.validate().is_valid(), "{:?}", graph.validate().issues);
    let notes = note_labels(&graph);
    assert!(notes.iter().any(|label| label == "9"), "{notes:?}");
    assert!(notes.iter().any(|label| label == "3"), "{notes:?}");
    let definitions = footnote_definitions(&graph);
    assert!(!definitions.iter().any(|definition| definition.starts_with("[^9]:")), "{definitions:?}");
    assert_eq!(
        definitions.iter().filter(|definition| definition.starts_with("[^3]:")).count(),
        2,
        "both duplicate definitions stay visible: {definitions:?}"
    );
    let markdown = export_markdown(&graph).expect("markdown export");
    assert!(markdown.contains("[^9]"), "{markdown}");
    assert!(!markdown.contains("[^9]:"), "{markdown}");
    assert!(markdown.contains("[^3]: First copy."), "{markdown}");
    assert!(markdown.contains("[^3]: Second copy."), "{markdown}");
}

// --- in-memory EPUB packages ------------------------------------------------

fn epub_fixture(name: &str) -> Vec<u8> {
    match name {
        "ordered-list" => build_epub("ordered-list", "Fixture ordered-list.epub", &[("chapter1.xhtml", ORDERED_LIST_CHAPTER)], false),
        "css-link" => build_epub("css-link", "Fixture css-link.epub", &[("chapter1.xhtml", CSS_LINK_CHAPTER)], true),
        "inline-style" => build_epub("inline-style", "Fixture inline-style.epub", &[("chapter1.xhtml", INLINE_STYLE_CHAPTER)], false),
        "list-value-reversed" => {
            build_epub("list-value-reversed", "Fixture list value reversed", &[("chapter1.xhtml", LIST_VALUE_REVERSED_CHAPTER)], false)
        }
        "footnotes" => build_epub("footnotes", "Fixture footnotes.epub", &[("chapter1.xhtml", FOOTNOTES_CHAPTER)], false),
        "footnotes-edge" => build_epub(
            "footnotes-edge",
            "Fixture footnotes-edge.epub",
            &[("chapter1.xhtml", FOOTNOTES_EDGE_MISSING), ("chapter2.xhtml", FOOTNOTES_EDGE_DUPLICATE)],
            false,
        ),
        other => panic!("unknown epub fixture `{other}`"),
    }
}

fn build_epub(identifier: &str, title: &str, chapters: &[(&str, &str)], linked_css: bool) -> Vec<u8> {
    let mut manifest = String::from(r#"<item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>"#);
    let mut spine = String::new();
    let mut nav_items = String::new();
    for (index, (href, _)) in chapters.iter().enumerate() {
        let id = format!("c{}", index + 1);
        manifest.push_str(&format!(r#"<item id="{id}" href="{href}" media-type="application/xhtml+xml"/>"#));
        spine.push_str(&format!(r#"<itemref idref="{id}"/>"#));
        nav_items.push_str(&format!(r#"<li><a href="{href}">Chapter {}</a></li>"#, index + 1));
    }
    if linked_css {
        manifest.push_str(r#"<item id="css" href="style.css" media-type="text/css"/>"#);
    }
    let opf = format!(
        r##"<?xml version="1.0" encoding="utf-8"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="bookid"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:identifier id="bookid">fixture-{identifier}.epub</dc:identifier><dc:title>{title}</dc:title><dc:language>en</dc:language></metadata><manifest>{manifest}</manifest><spine>{spine}</spine></package>"##
    );
    let nav = format!(
        r##"<?xml version="1.0" encoding="utf-8"?><html xmlns="http://www.w3.org/1999/xhtml"><head><title>nav</title></head><body><nav epub:type="toc" xmlns:epub="http://www.idpf.org/2007/ops"><ol>{nav_items}</ol></nav></body></html>"##
    );
    let mut parts: Vec<(String, Vec<u8>)> =
        vec![("mimetype".to_string(), b"application/epub+zip".to_vec()), ("META-INF/container.xml".to_string(), CONTAINER.to_vec())];
    for (href, body) in chapters {
        parts.push((format!("OEBPS/{href}"), body.as_bytes().to_vec()));
    }
    parts.push(("OEBPS/content.opf".to_string(), opf.into_bytes()));
    parts.push(("OEBPS/nav.xhtml".to_string(), nav.into_bytes()));
    zip_stored(&parts)
}

fn zip_stored(parts: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut output = Vec::new();
    let mut directory = Vec::new();
    for (name, bytes) in parts {
        let crc = crc32(bytes);
        let offset = output.len() as u32;
        output.extend_from_slice(b"PK\x03\x04");
        output.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        output.extend_from_slice(&crc.to_le_bytes());
        output.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        output.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        output.extend_from_slice(&(name.len() as u16).to_le_bytes());
        output.extend_from_slice(&0u16.to_le_bytes());
        output.extend_from_slice(name.as_bytes());
        output.extend_from_slice(bytes);
        directory.extend_from_slice(b"PK\x01\x02");
        directory.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        directory.extend_from_slice(&crc.to_le_bytes());
        directory.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        directory.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        directory.extend_from_slice(&(name.len() as u16).to_le_bytes());
        directory.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        directory.extend_from_slice(&offset.to_le_bytes());
        directory.extend_from_slice(name.as_bytes());
    }
    let directory_offset = output.len() as u32;
    output.extend_from_slice(&directory);
    output.extend_from_slice(b"PK\x05\x06");
    output.extend_from_slice(&[0, 0, 0, 0]);
    output.extend_from_slice(&(parts.len() as u16).to_le_bytes());
    output.extend_from_slice(&(parts.len() as u16).to_le_bytes());
    output.extend_from_slice(&(directory.len() as u32).to_le_bytes());
    output.extend_from_slice(&directory_offset.to_le_bytes());
    output.extend_from_slice(&[0, 0]);
    output
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in bytes {
        crc ^= *byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

const CONTAINER: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>"#;

const ORDERED_LIST_CHAPTER: &str = r##"<?xml version="1.0" encoding="utf-8"?><html xmlns="http://www.w3.org/1999/xhtml"><head><title>ordered</title></head><body><h1>Ordered</h1><ol><li><p>one</p></li><li><p>two</p></li></ol><ol start="3"><li><p>three</p></li><li><ul><li>nested a</li><li>nested b</li></ul></li></ol></body></html>"##;

const CSS_LINK_CHAPTER: &str = r##"<?xml version="1.0" encoding="utf-8"?><html xmlns="http://www.w3.org/1999/xhtml"><head><title>css link</title><link rel="stylesheet" type="text/css" href="style.css"/></head><body><p>Styled paragraph.</p></body></html>"##;

const INLINE_STYLE_CHAPTER: &str = r##"<?xml version="1.0" encoding="utf-8"?><html xmlns="http://www.w3.org/1999/xhtml"><head><title>css block</title><style>p { color: red; }</style></head><body><p>Styled paragraph.</p></body></html>"##;

const LIST_VALUE_REVERSED_CHAPTER: &str = r##"<?xml version="1.0" encoding="utf-8"?><html xmlns="http://www.w3.org/1999/xhtml"><head><title>lists</title></head><body><h1>Lists</h1><ol start="2"><li><p>two</p></li><li value="5"><p>five</p></li><li><p>six</p></li></ol><ol><li value="1"><p>one</p></li><li value="1"><p>duplicate</p></li></ol><ol reversed><li><p>a</p></li><li><p>b</p></li></ol></body></html>"##;

const FOOTNOTES_CHAPTER: &str = r##"<?xml version="1.0" encoding="utf-8"?><html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops"><head><title>footnotes</title></head><body><h1>Footnotes</h1><p>Body with a single note<a epub:type="noteref" href="#fn1" id="fnref1">1</a>, a note with two paragraphs<a epub:type="noteref" href="#fn2" id="fnref2">2</a>, and a note with inline markup<a epub:type="noteref" href="#fn3" id="fnref3">3</a>.</p><aside epub:type="footnote" id="fn1"><p>Single paragraph note.<a href="#fnref1">↩</a></p></aside><aside epub:type="footnote" id="fn2"><p>First paragraph.<a href="#fnref2">↩︎</a></p><p>Second paragraph.</p></aside><aside epub:type="footnote" id="fn3"><p>Inline <strong>bold</strong>, <code>code</code>, and <a href="https://example.com">link</a>.<a href="#fnref3">return</a></p></aside></body></html>"##;

const FOOTNOTES_EDGE_MISSING: &str = r##"<?xml version="1.0" encoding="utf-8"?><html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops"><head><title>missing</title></head><body><h1>Missing definition</h1><p>Lonely reference<a epub:type="noteref" href="#fn9" id="fnref9">9</a>.</p></body></html>"##;

const FOOTNOTES_EDGE_DUPLICATE: &str = r##"<?xml version="1.0" encoding="utf-8"?><html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops"><head><title>duplicate</title></head><body><h1>Duplicate definition</h1><p>Repeated reference<a epub:type="noteref" href="#fn3" id="fnref3">3</a>.</p><aside epub:type="footnote" id="fn3"><p>First copy.</p></aside><aside epub:type="footnote" id="fn3"><p>Second copy.</p></aside></body></html>"##;
