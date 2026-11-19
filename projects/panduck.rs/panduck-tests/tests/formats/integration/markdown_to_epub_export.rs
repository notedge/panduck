//! Markdown → EPUB 3 export round-trip integrity tests.
//!
//! Exercises `export::epub::export_epub_bytes` and verifies the resulting ZIP
//! is readable by `acorn-epub::OcfPackage` with the expected OPF, nav, and
//! chapter members.

use acorn_core::{Budget, BudgetTracker};
use acorn_epub::OcfPackage;
use document_formats::{
    export::{epub::export_epub_bytes, markdown::export_markdown},
    import::{epub::import_epub_bytes, markdown::import_markdown_bytes, notedown::import_notedown_bytes},
};

fn make_simple_markdown() -> &'static str {
    r#"# A Small Book

Some opening prose.

## Chapter One

Hello world. `let x = 1;`

## Chapter Two

A table:

| a | b |
|---|---|
| 1 | 2 |

And a list:

- one
- two
- three
"#
}

#[test]
fn epub_export_keeps_the_title_at_the_start_of_the_body_chapter() {
    // `notedown` (and `markdown`) lower a heading to a flat `Section` whose body
    // lives in following sibling blocks. The chapter planner must therefore
    // keep authored order and let a heading open the chapter that owns its
    // body, instead of hoisting every non-heading block into a lead chapter.
    let graph = import_notedown_bytes("book.nd", "# Sample Book\n\nHello EPUB\n").expect("import notedown");
    let bytes = export_epub_bytes(&graph).expect("export epub");

    let package = OcfPackage::open("book.epub", bytes.clone()).expect("open as OCF");
    let mut ledger = BudgetTracker::new(Budget::default());
    let first = String::from_utf8(package.read_member("OEBPS/chapter-000.xhtml", &mut ledger).expect("read chapter-000"))
        .expect("chapter-000 must be UTF-8");
    assert!(first.contains("<h1>Sample Book</h1>"), "the title must open the first chapter: {first}");
    assert!(first.contains("Hello EPUB"), "the body must stay in the chapter opened by its title: {first}");
    assert!(package.read_member("OEBPS/chapter-001.xhtml", &mut ledger).is_err(), "one title plus one body must not split into two chapters");

    let reopened = import_epub_bytes("book.epub", &bytes).expect("reopen epub");
    let markdown = export_markdown(&reopened).expect("export markdown");
    assert!(markdown.trim_start().starts_with("# Sample Book"), "reopen must yield title first: {markdown}");
    assert!(markdown.contains("Hello EPUB"), "{markdown}");
}

#[test]
fn epub_export_writes_an_unknown_inline_style_as_a_class() {
    let mut graph = document_ir::DocumentGraph::new(document_ir::DocumentId(80));
    graph.push_block(document_ir::Block::Paragraph {
        content: vec![document_ir::Inline::Styled {
            style: "zzstyle".into(),
            children: vec![document_ir::Inline::Text { text: "ZZR53ZZ".into() }],
        }],
    });

    let bytes = export_epub_bytes(&graph).expect("an unknown style must not reject the EPUB export");
    let package = OcfPackage::open("style.epub", bytes).expect("open as OCF");
    let mut ledger = BudgetTracker::new(Budget::default());
    let chapter =
        String::from_utf8(package.read_member("OEBPS/chapter-000.xhtml", &mut ledger).expect("read chapter")).expect("chapter must be UTF-8");
    assert!(chapter.contains(r#"<span class="zzstyle">ZZR53ZZ</span>"#), "{chapter}");
}

#[test]
fn epub_export_produces_valid_ocf_package() {
    let graph = import_markdown_bytes("book.md", make_simple_markdown()).expect("import markdown");
    let bytes = export_epub_bytes(&graph).expect("export epub");

    // Must start with the ZIP local file header followed by "mimetype" + payload.
    assert!(bytes.starts_with(b"PK\x03\x04"), "EPUB must start with ZIP local header");

    // OcfPackage must open without error.
    let package = OcfPackage::open("test.epub", bytes).expect("open as OCF");
    let mut ledger = BudgetTracker::new(Budget::default());

    let root_opf = package.root_opf_path();
    assert_eq!(root_opf, "OEBPS/content.opf", "container.xml must point at content.opf");

    let opf = package.read_opf(&mut ledger).expect("read OPF");
    assert!(opf.metadata.title.as_deref().map_or(false, |s| s.contains("Small Book")));
    assert_eq!(opf.metadata.language.as_deref(), Some("en"));
    assert!(!opf.manifest.is_empty(), "manifest must not be empty");
    assert!(!opf.spine.is_empty(), "spine must not be empty");

    // Nav manifest item must be flagged with properties="nav".
    let nav_item = opf.manifest.get("nav").expect("manifest must contain nav");
    assert_eq!(nav_item.media_type, "application/xhtml+xml");
    assert!(
        nav_item.properties.as_deref().map_or(false, |s| s.contains("nav")),
        "nav manifest item must carry properties=nav, got {:?}",
        nav_item.properties
    );

    // All spine items must exist in the manifest and have an XHTML href.
    for spine in &opf.spine {
        let item = opf.manifest.get(&spine.idref).expect("spine idref must resolve in manifest");
        assert!(item.media_type.contains("xhtml"), "spine item must be XHTML, got {}", item.media_type);
        assert!(item.href.ends_with(".xhtml"), "spine href must end with .xhtml");
    }
}

#[test]
fn epub_export_mimetype_is_stored_and_first() {
    let graph = import_markdown_bytes("book.md", "# Title\n\nBody.").expect("import markdown");
    let bytes = export_epub_bytes(&graph).expect("export epub");

    eprintln!("total bytes: {}", bytes.len());
    eprintln!("first 60 bytes hex: {:02x?}", &bytes[..60.min(bytes.len())]);
    eprintln!("bytes[26..30]: {:02x?}", &bytes[26..30]);

    // First local file header must point at "mimetype".
    // Local file header layout (APPNOTE §4.3.7):
    //  0..4  signature "PK\x03\x04"
    //  4..6  version needed
    //  6..8  flags
    //  8..10 compression method (0 = stored)
    let flags = u16::from_le_bytes([bytes[6], bytes[7]]);
    let compression = u16::from_le_bytes([bytes[8], bytes[9]]);
    assert_eq!(compression, 0, "mimetype must be STORED (method 0)");
    assert!(flags & 0x08 == 0, "mimetype must not use data descriptor (bit 3)");

    let name_len = u16::from_le_bytes([bytes[26], bytes[27]]) as usize;
    let name = &bytes[30..30 + name_len];
    assert_eq!(name, b"mimetype");

    let extra_len = u16::from_le_bytes([bytes[28], bytes[29]]) as usize;
    let payload_offset = 30 + name_len + extra_len;
    // Compressed size lives at offset 18..22 in the local file header (APPNOTE
    // §4.3.7).
    let payload_len = u32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]) as usize;
    let payload = &bytes[payload_offset..payload_offset + payload_len];
    assert_eq!(payload, b"application/epub+zip");
}

/// EPUB 3 §4.2 规定 OPF 必须声明 dc:identifier / dc:title / dc:language；
/// §3.4 规定 nav 文档的 <html> 根元素必须带 epub:type="nav"。
///
/// 这些断言走原始 XML 字符串匹配，不依赖 epubcheck。
#[test]
fn epub_export_opf_has_required_dc_elements_and_nav_root_is_html_with_epub_type() {
    let graph = import_markdown_bytes("book.md", make_simple_markdown()).expect("import markdown");
    let bytes = export_epub_bytes(&graph).expect("export epub");

    let package = OcfPackage::open("test.epub", bytes).expect("open as OCF");
    let mut ledger = BudgetTracker::new(Budget::default());

    // --- 1. OPF 原始 XML 必须包含三个必填 dc 元素 -----------------------------
    let opf_bytes = package.read_member(package.root_opf_path(), &mut ledger).expect("read OPF raw bytes");
    let opf_xml = String::from_utf8(opf_bytes).expect("OPF must be UTF-8");

    assert!(opf_xml.contains("<dc:identifier"), "OPF must declare dc:identifier (EPUB 3 §4.2.1)");
    assert!(opf_xml.contains("<dc:title>"), "OPF must declare dc:title (EPUB 3 §4.2.2)");
    assert!(opf_xml.contains("<dc:language>"), "OPF must declare dc:language (EPUB 3 §4.2.3)");
    // dc:identifier 内容必须是 urn:uuid:<32 hex with dashes> 格式。
    let uuid_start = opf_xml.find("urn:uuid:").expect("dc:identifier should contain urn:uuid: prefix");
    let uuid_body = &opf_xml[uuid_start + 9..uuid_start + 9 + 36];
    assert_eq!(uuid_body.chars().filter(|&c| c == '-').count(), 4, "uuid must have 4 dashes");
    let without_dashes: String = uuid_body.chars().filter(|&c| c != '-').collect();
    assert_eq!(without_dashes.len(), 32, "uuid body must be 32 hex chars without dashes");
    assert!(without_dashes.chars().all(|c| c.is_ascii_hexdigit()), "uuid body must be all hex digits, got {uuid_body}");

    // --- 2. nav 文档根元素是带 epub:type="nav" 的 <html> --------------------
    let nav_bytes = package.read_member("OEBPS/nav.xhtml", &mut ledger).expect("read nav.xhtml raw bytes");
    let nav_xml = String::from_utf8(nav_bytes).expect("nav.xhtml must be UTF-8");

    assert!(nav_xml.starts_with("<?xml"), "nav.xhtml must begin with an XML declaration");
    assert!(nav_xml.contains(r#"<html xmlns="http://www.w3.org/1999/xhtml""#), "nav.xhtml root must be <html> with the XHTML namespace");
    assert!(nav_xml.contains("xmlns:epub=\"http://www.idpf.org/2007/ops\""), "nav.xhtml must declare the epub namespace on root <html>");
    assert!(nav_xml.contains(r#"epub:type="nav""#), "nav.xhtml root <html> must carry epub:type=\"nav\" (EPUB 3 §3.4)");
    // xml:lang 和 lang 属性都必须落在根 <html> 上，且值一致。
    let xml_lang = extract_attr(&nav_xml, "xml:lang");
    let lang = extract_attr(&nav_xml, "lang");
    assert!(
        xml_lang.is_some() && lang.is_some() && xml_lang == lang,
        "nav.xhtml root <html> must have matching xml:lang and lang, got xml:lang={xml_lang:?}, lang={lang:?}"
    );
}

/// Extract the double-quoted value of an attribute from the nav string.
fn extract_attr(xml: &str, name: &str) -> Option<String> {
    let needle = format!("{name}=\"");
    let idx = xml.find(&needle)?;
    let start = idx + needle.len();
    let rest = &xml[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

#[test]
fn epub_export_renders_footnotes_as_endnotes_instead_of_rejecting_the_block() {
    let graph = import_markdown_bytes("footnote.md", "See[^1] after.\n\n[^1]: Body.\n").expect("import markdown");
    let bytes = export_epub_bytes(&graph).expect("export epub with footnotes");

    let package = OcfPackage::open("footnote.epub", bytes).expect("open as OCF");
    let opf = package.read_opf(&mut BudgetTracker::new(Budget::default())).expect("read OPF");
    let mut chapter_xml = None;
    for item in opf.manifest.values().filter(|item| item.media_type == "application/xhtml+xml") {
        let xml = String::from_utf8(
            package.read_member(&format!("OEBPS/{}", item.href), &mut BudgetTracker::new(Budget::default())).unwrap_or_default(),
        )
        .unwrap_or_default();
        if xml.contains("doc-endnotes") {
            chapter_xml = Some(xml);
            break;
        }
    }
    let chapter_xml = chapter_xml.expect("one chapter must carry endnotes");

    assert!(chapter_xml.contains(r#"<section class="footnotes" role="doc-endnotes">"#), "chapter must contain endnotes: {chapter_xml}");
    assert!(chapter_xml.contains(r#"data-footnote-label="1""#), "footnote reference must carry its label marker: {chapter_xml}");
    assert!(chapter_xml.contains("data-footnote-source-hex="), "definition must keep the authored payload: {chapter_xml}");
    assert!(!chapter_xml.contains("[^1]:"), "GFM payload must not leak as prose: {chapter_xml}");
}

#[test]
fn epub_reference_across_chapters_points_at_the_owning_chapter() {
    use document_ir::{Block, DocumentGraph, DocumentId, Inline};

    let mut graph = DocumentGraph::new(DocumentId(73));
    let first = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "first".into() }] });
    let _section_one = graph.push_block(Block::Section { level: 1, title: vec![Inline::Text { text: "One".into() }], children: vec![first] });
    let second = graph.push_block(Block::Paragraph { content: vec![Inline::Reference { display: "go".into(), target: first }] });
    graph.push_block(Block::Section { level: 1, title: vec![Inline::Text { text: "Two".into() }], children: vec![second] });

    let bytes = export_epub_bytes(&graph).expect("export two chapters");
    let package = OcfPackage::open("cross-reference.epub", bytes).expect("open as OCF");
    let mut ledger = BudgetTracker::new(Budget::default());
    let mut read =
        |path: &str| String::from_utf8(package.read_member(path, &mut ledger).expect("read chapter")).expect("chapter must be UTF-8");
    let chapter_one = read("OEBPS/chapter-000.xhtml");
    let chapter_two = read("OEBPS/chapter-001.xhtml");
    assert!(chapter_one.contains(&format!(r#"id="node-{}""#, first.0)), "{chapter_one}");
    assert!(
        chapter_two.contains(&format!(r#""chapter-000.xhtml#node-{}""#, first.0)),
        "a reference must name the chapter that owns the anchor: {chapter_two}"
    );
}

#[test]
fn epub_export_keeps_table_caption_list_start_and_reference_anchor() {
    use document_ir::{Block, DocumentGraph, DocumentId, Inline, ListItem, TableCell, TableRow};

    let mut graph = DocumentGraph::new(DocumentId(71));
    let first_cell = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "first".into() }] });
    graph.push_block(Block::Table {
        caption: Some("首表".into()),
        head: vec![TableRow { cells: vec![TableCell { children: vec![first_cell], colspan: 1, rowspan: 1 }] }],
        body: Vec::new(),
        foot: Vec::new(),
    });
    let target = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "target".into() }] });
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
    graph.push_block(Block::Paragraph { content: vec![Inline::Reference { display: "go".into(), target }] });

    let bytes = export_epub_bytes(&graph).expect("export epub with structural blocks");
    let package = OcfPackage::open("structural.epub", bytes).expect("open as OCF");
    let mut ledger = BudgetTracker::new(Budget::default());
    let chapter =
        String::from_utf8(package.read_member("OEBPS/chapter-000.xhtml", &mut ledger).expect("read chapter")).expect("chapter must be UTF-8");

    assert!(chapter.contains("<caption>首表</caption>"), "{chapter}");
    assert!(chapter.contains("<caption>尾表</caption>"), "{chapter}");
    assert!(chapter.contains(r#"<ol start="5">"#), "{chapter}");
    assert!(chapter.contains(r#"<ol start="3">"#), "{chapter}");
    assert!(chapter.contains(r#"<li value="7">"#), "{chapter}");
    assert!(chapter.contains(r#"<li value="5">"#), "{chapter}");
    assert!(chapter.contains(&format!(r##"href="#node-{}""##, target.0)), "the reference must point at its target: {chapter}");
    assert!(chapter.contains(&format!(r#"id="node-{}""#, target.0)), "the reference target must exist in the same chapter: {chapter}");
}
