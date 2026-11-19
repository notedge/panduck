use document_convert::{convert_bytes, convert_bytes_with_body_faces, write_document_docx, write_document_html, write_document_pdf};
use document_formats::import::{markdown::import_markdown_bytes, pdf::import_pdf_bytes};
use document_ir::{
    AttributePair, Attributes, Block, DocumentGraph, DocumentId, Inline, ListItem, MathDisplay, MathFormula, RelationEndpoint, SemanticStatus,
    TableCell, TableRow,
};

use crate::common;
use common::{assert_convert_report, assert_report_contains_loss_codes, docx_zip, docx_zip_with_footnotes, pdf_body_faces, stored_zip};

#[test]
fn html_to_markdown_convert_report_matches_ir_loss() {
    let output = convert_bytes(
        "html",
        "markdown",
        "page.html",
        br#"<html><body><h1>Title</h1><p>Hello.</p><script>alert(1)</script></body></html>"#.to_vec(),
        None,
    )
    .expect("convert html to markdown");

    assert_convert_report(&output, "markdown");
    assert_report_contains_loss_codes(&output.report_json, &["import.html.partial_coverage", "import.html.raw_script"]);
    let reopened = import_markdown_bytes("round.md", &output.markdown).expect("reopen markdown");
    assert!(reopened.validate().is_valid());
}

#[test]
fn docx_field_to_markdown_convert_report_lists_field_loss() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText> PAGE </w:instrText></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r><w:r><w:t>visible fallback</w:t></w:r></w:p></w:body>
</w:document>"#;
    let output = convert_bytes("docx", "markdown", "field.docx", docx_zip(document_xml), None).expect("convert docx with field");

    assert_convert_report(&output, "markdown");
    assert_report_contains_loss_codes(&output.report_json, &["reader.docx.fields", "import.docx.partial_coverage"]);
    assert!(output.markdown.contains("visible fallback"));
}

#[test]
fn docx_unresolved_footnote_to_markdown_convert_report_lists_footnote_loss() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:t>See</w:t></w:r>
      <w:r><w:footnoteReference w:id="1"/></w:r>
      <w:r><w:t> for details.</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
    let output =
        convert_bytes("docx", "markdown", "footnote.docx", docx_zip(document_xml), None).expect("convert docx with unresolved footnote");

    assert_convert_report(&output, "markdown");
    assert_report_contains_loss_codes(&output.report_json, &["reader.docx.footnote_body", "import.docx.partial_coverage"]);
    assert!(output.markdown.contains("See[^1] for details."));
}

#[test]
fn docx_resolved_footnote_to_markdown_convert_report_omits_footnote_body_loss() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:t>See</w:t></w:r>
      <w:r><w:footnoteReference w:id="1"/></w:r>
      <w:r><w:t> for details.</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
    let footnotes_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:footnote w:id="1">
    <w:p><w:r><w:t>Footnote body.</w:t></w:r></w:p>
  </w:footnote>
</w:footnotes>"#;
    let output = convert_bytes("docx", "markdown", "footnote-resolved.docx", docx_zip_with_footnotes(document_xml, footnotes_xml), None)
        .expect("convert docx with resolved footnote");

    assert_convert_report(&output, "markdown");
    assert_report_contains_loss_codes(&output.report_json, &["import.docx.partial_coverage"]);
    let report = serde_json::from_str::<serde_json::Value>(&output.report_json).expect("report");
    let loss_codes = report["losses"].as_array().expect("losses").iter().filter_map(|loss| loss["code"].as_str()).collect::<Vec<_>>();
    assert!(
        !loss_codes.iter().any(|code| *code == "reader.docx.footnote_body"),
        "resolved footnote must not report reader.docx.footnote_body, got {loss_codes:?}"
    );
    assert!(output.markdown.contains("[^1]: Footnote body."));
}

#[test]
fn epub_to_markdown_convert_report_lists_epub_partial_coverage() {
    let container = br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;
    let opf = br#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Sample Book</dc:title>
    <dc:language>en</dc:language>
  </metadata>
  <manifest>
    <item id="ch1" href="chapter.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="ch1"/>
  </spine>
</package>"#;
    let zip = stored_zip(&[
        ("mimetype", b"application/epub+zip"),
        ("META-INF/container.xml", container),
        ("OEBPS/content.opf", opf),
        (
            "OEBPS/chapter.xhtml",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <body><h1>Chapter</h1><p>Body.</p></body>
</html>"#,
        ),
    ]);

    let output = convert_bytes("epub", "markdown", "book.epub", zip, None).expect("convert epub");
    assert_convert_report(&output, "markdown");
    assert_report_contains_loss_codes(&output.report_json, &["import.epub.partial_coverage"]);
}

#[test]
fn epub_missing_nav_member_convert_report_lists_navigation_missing() {
    let container = br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;
    let opf = br#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Broken Nav Book</dc:title>
    <dc:language>en</dc:language>
  </metadata>
  <manifest>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
    <item id="ch1" href="chapter.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="ch1"/>
  </spine>
</package>"#;
    let zip = stored_zip(&[
        ("mimetype", b"application/epub+zip"),
        ("META-INF/container.xml", container),
        ("OEBPS/content.opf", opf),
        (
            "OEBPS/chapter.xhtml",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <body><h1>Chapter</h1><p>Body.</p></body>
</html>"#,
        ),
    ]);

    let output = convert_bytes("epub", "markdown", "broken-nav.epub", zip, None).expect("convert epub");
    assert_convert_report(&output, "markdown");
    assert_report_contains_loss_codes(&output.report_json, &["import.epub.partial_coverage", "import.epub.navigation_missing"]);
}

#[test]
fn epub_with_nav_to_markdown_convert_report_omits_navigation_missing() {
    let container = br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;
    let opf = br#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Nav Book</dc:title>
    <dc:language>en</dc:language>
  </metadata>
  <manifest>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
    <item id="ch1" href="chapter.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="ch1"/>
  </spine>
</package>"#;
    let nav = br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
  <body>
    <nav epub:type="toc" id="toc">
      <ol><li><a href="chapter.xhtml">Chapter One</a></li></ol>
    </nav>
  </body>
</html>"#;
    let zip = stored_zip(&[
        ("mimetype", b"application/epub+zip"),
        ("META-INF/container.xml", container),
        ("OEBPS/content.opf", opf),
        ("OEBPS/nav.xhtml", nav),
        (
            "OEBPS/chapter.xhtml",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <body><h1>Chapter One</h1><p>Body.</p></body>
</html>"#,
        ),
    ]);

    let output = convert_bytes("epub", "markdown", "nav.epub", zip, None).expect("convert epub with nav");
    assert_convert_report(&output, "markdown");
    let report = serde_json::from_str::<serde_json::Value>(&output.report_json).expect("report");
    let loss_codes = report["losses"].as_array().expect("losses").iter().filter_map(|loss| loss["code"].as_str()).collect::<Vec<_>>();
    assert!(
        !loss_codes.iter().any(|code| *code == "import.epub.navigation_missing"),
        "nav manifest must not report navigation_missing, got {loss_codes:?}"
    );
    assert!(output.markdown.contains("[Chapter One](chapter.xhtml)"));
}

#[test]
fn epub_styled_to_markdown_convert_report_lists_css_and_inline_style_losses() {
    let container = br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;
    let opf = br#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Styled Book</dc:title>
    <dc:language>en</dc:language>
  </metadata>
  <manifest>
    <item id="css" href="styles/main.css" media-type="text/css"/>
    <item id="ch1" href="chapter.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="ch1"/>
  </spine>
</package>"#;
    let zip = stored_zip(&[
        ("mimetype", b"application/epub+zip"),
        ("META-INF/container.xml", container),
        ("OEBPS/content.opf", opf),
        ("OEBPS/styles/main.css", b"body { color: red; }"),
        (
            "OEBPS/chapter.xhtml",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <head><link rel="stylesheet" href="styles/main.css"/></head>
  <body><p style="color: blue;">Styled text</p></body>
</html>"#,
        ),
    ]);

    let output = convert_bytes("epub", "markdown", "styled.epub", zip, None).expect("convert styled epub");
    assert_convert_report(&output, "markdown");
    assert_report_contains_loss_codes(
        &output.report_json,
        &["import.epub.partial_coverage", "import.epub.css_link_unsupported", "import.epub.inline_style_unsupported"],
    );
}

#[test]
#[ignore = "legacy DOC fixture lives in the notedown downstream"]
fn doc_to_markdown_convert_report_lists_partial_coverage() {
    // Fixture bytes are owned by `@notedge/notedown` / notedown-fixtures.
}

#[test]
fn pdf_to_markdown_convert_report_lists_partial_coverage() {
    let faces = pdf_body_faces();
    let pdf = convert_bytes_with_body_faces("markdown", "pdf", "sample.md", b"# Heading\n\nBody text.\n".to_vec(), None, &faces)
        .expect("write PDF")
        .binary
        .expect("PDF bytes");
    let output = convert_bytes("pdf", "markdown", "sample.pdf", pdf, None).expect("convert pdf to markdown");

    assert_convert_report(&output, "markdown");
    assert_report_contains_loss_codes(&output.report_json, &["import.pdf.partial_coverage"]);
}

#[test]
fn pdf_convert_reports_dropped_citation_key_and_image_title() {
    let faces = pdf_body_faces();
    let mut graph = DocumentGraph::new(DocumentId(7));
    graph.push_block(Block::Paragraph {
        content: vec![
            Inline::Citation { key: "smith2024".into(), display: Some(vec![Inline::Text { text: "(Smith 2024)".into() }]) },
            Inline::Text { text: " ".into() },
            Inline::Image {
                source: "media/logo.png".into(),
                alt: vec![Inline::Text { text: "alt".into() }],
                title: Some("Logo title".into()),
                asset: None,
            },
        ],
    });

    write_document_pdf(&mut graph, &faces).expect("write PDF");
    let codes: Vec<&str> = graph.coverage.loss.iter().map(|loss| loss.code.as_str()).collect();
    assert_eq!(
        codes,
        ["export.pdf.image_unresolved", "export.pdf.citation_degraded", "export.pdf.image_title_dropped", "export.pdf.image_alt_dropped"],
        "{:?}",
        graph.coverage.loss
    );
    assert!(
        graph.coverage.loss.iter().all(|loss| loss.status == SemanticStatus::Partial),
        "a dropped key or title is partial, not unsupported: {:?}",
        graph.coverage.loss
    );
}

#[test]
fn pdf_export_names_image_alt_when_the_page_cannot_carry_it() {
    let faces = pdf_body_faces();
    let mut graph = DocumentGraph::new(DocumentId(8));
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Image {
            source: "media/logo.png".into(),
            alt: vec![Inline::Text { text: "ZZ".into() }],
            title: None,
            asset: None,
        }],
    });

    let pdf = write_document_pdf(&mut graph, &faces).expect("write PDF");
    let codes: Vec<&str> = graph.coverage.loss.iter().map(|loss| loss.code.as_str()).collect();
    assert!(codes.contains(&"export.pdf.image_alt_dropped"), "the report must name alt itself: {codes:?}");
    assert!(!String::from_utf8_lossy(&pdf).contains("ZZ"), "if the page ever carries alt, drop the loss code");
}

#[test]
fn pdf_export_keeps_list_and_nested_table_cells_inside_the_table() {
    let faces = pdf_body_faces();
    let mut graph = DocumentGraph::new(DocumentId(11));
    let head_text = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "ZZ_HEAD".into() }] });
    let head_text_two = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "ZZ_HEAD_TWO".into() }] });
    let paragraph_cell = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "ZZ_PARA".into() }] });
    let list_cell = graph.push_block(Block::List {
        ordered: true,
        items: vec![
            ListItem { content: vec![Inline::Text { text: "ZZ_LIST_A".into() }], children: Vec::new(), number: Some(1), ..Default::default() },
            ListItem { content: vec![Inline::Text { text: "ZZ_LIST_B".into() }], children: Vec::new(), number: Some(2), ..Default::default() },
        ],
    });
    let inner_head = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "ZZ_INNER_HEAD".into() }] });
    let inner_cell = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "ZZ_INNER".into() }] });
    let inner_table = graph.push_block(Block::Table {
        caption: Some("ZZ_CAPTION".into()),
        head: vec![TableRow { cells: vec![TableCell { children: vec![inner_head], colspan: 1, rowspan: 1 }] }],
        body: vec![TableRow { cells: vec![TableCell { children: vec![inner_cell], colspan: 1, rowspan: 1 }] }],
        foot: Vec::new(),
    });
    graph.push_block(Block::Table {
        caption: None,
        head: vec![TableRow {
            cells: vec![
                TableCell { children: vec![head_text], colspan: 1, rowspan: 1 },
                TableCell { children: vec![head_text_two], colspan: 1, rowspan: 1 },
            ],
        }],
        body: vec![
            TableRow {
                cells: vec![
                    TableCell { children: vec![paragraph_cell], colspan: 1, rowspan: 1 },
                    TableCell { children: vec![list_cell], colspan: 1, rowspan: 1 },
                ],
            },
            TableRow { cells: vec![TableCell { children: vec![inner_table], colspan: 2, rowspan: 1 }] },
        ],
        foot: Vec::new(),
    });

    let pdf = write_document_pdf(&mut graph, &faces).expect("write PDF");
    let codes: Vec<&str> = graph.coverage.loss.iter().map(|loss| loss.code.as_str()).collect();
    assert_eq!(
        codes.iter().filter(|code| **code == "export.pdf.table_cell_flattened").count(),
        1,
        "one table with flattened sub-blocks must report the loss once: {codes:?}"
    );
    let flattened =
        graph.coverage.loss.iter().find(|loss| loss.code == "export.pdf.table_cell_flattened").expect("the flattening loss must be present");
    assert!(flattened.message.contains("`list`"), "the loss must name the list: {}", flattened.message);
    assert!(flattened.message.contains("`table`"), "the loss must name the nested table: {}", flattened.message);

    let reopened = import_pdf_bytes("table-cells.pdf", &pdf).expect("reopen PDF");
    let text = all_text(&reopened);
    for expected in ["ZZ_HEAD", "ZZ_HEAD_TWO", "ZZ_PARA", "ZZ_LIST_A", "ZZ_LIST_B", "ZZ_CAPTION", "ZZ_INNER_HEAD", "ZZ_INNER"] {
        assert!(text.contains(expected), "{expected} must reach the page: {text:?}");
    }
}

#[test]
fn pdf_export_reports_no_cell_flattening_for_paragraph_cells() {
    let faces = pdf_body_faces();
    let mut graph = DocumentGraph::new(DocumentId(12));
    let head_text = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "ZZ_ONLY_HEAD".into() }] });
    let body_text = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "ZZ_ONLY_BODY".into() }] });
    graph.push_block(Block::Table {
        caption: None,
        head: vec![TableRow { cells: vec![TableCell { children: vec![head_text], colspan: 1, rowspan: 1 }] }],
        body: vec![TableRow { cells: vec![TableCell { children: vec![body_text], colspan: 1, rowspan: 1 }] }],
        foot: Vec::new(),
    });

    let pdf = write_document_pdf(&mut graph, &faces).expect("write PDF");
    let codes: Vec<&str> = graph.coverage.loss.iter().map(|loss| loss.code.as_str()).collect();
    assert!(
        !codes.contains(&"export.pdf.table_cell_flattened"),
        "paragraph cells keep their structure and must not raise the flattening loss: {codes:?}"
    );
    let reopened = import_pdf_bytes("paragraph-cells.pdf", &pdf).expect("reopen PDF");
    let text = all_text(&reopened);
    for expected in ["ZZ_ONLY_HEAD", "ZZ_ONLY_BODY"] {
        assert!(text.contains(expected), "{expected} must reach the page: {text:?}");
    }
}

#[test]
fn pdf_export_names_image_alt_only_in_the_body_paragraph_path() {
    let faces = pdf_body_faces();
    let mut graph = DocumentGraph::new(DocumentId(10));
    graph.push_block(Block::Section { level: 1, title: vec![image_with_alt("ZZH", "media/h.png")], children: vec![] });
    graph.push_block(Block::List {
        ordered: false,
        items: vec![ListItem {
            content: vec![image_with_alt("ZZL", "media/l.png")],
            children: vec![],
            number: None,
            is_task: false,
            is_checked: Some(false),
        }],
    });
    let cell_text = graph.push_block(Block::Paragraph { content: vec![image_with_alt("ZZT", "media/t.png")] });
    graph.push_block(Block::Table {
        caption: None,
        head: vec![TableRow { cells: vec![TableCell { children: vec![cell_text], colspan: 1, rowspan: 1 }] }],
        body: vec![],
        foot: vec![],
    });
    graph.push_block(Block::Paragraph { content: vec![image_with_alt("ZZP", "media/p.png")] });

    let pdf = write_document_pdf(&mut graph, &faces).expect("write PDF");
    let codes: Vec<&str> = graph.coverage.loss.iter().map(|loss| loss.code.as_str()).collect();
    assert_eq!(
        codes.iter().filter(|code| **code == "export.pdf.image_alt_dropped").count(),
        1,
        "only the body paragraph cannot carry alt: {codes:?}"
    );

    let reopened = import_pdf_bytes("alt-contexts.pdf", &pdf).expect("reopen PDF");
    let text = all_text(&reopened);
    for expected in ["ZZH", "ZZL", "ZZT"] {
        assert!(text.contains(expected), "{expected} must reach the text flow: {text:?}");
    }
    assert!(!text.contains("ZZP"), "body paragraph alt has no slot, so the code must stand in: {text:?}");
}

fn image_with_alt(alt: &str, source: &str) -> Inline {
    Inline::Image { source: source.into(), alt: vec![Inline::Text { text: alt.into() }], title: None, asset: None }
}

/// Loss codes the PDF report carries after the write.
fn pdf_loss_codes(graph: &DocumentGraph) -> Vec<String> {
    graph.coverage.loss.iter().map(|loss| loss.code.clone()).collect()
}

#[test]
fn pdf_export_reports_an_unsupported_math_language() {
    let faces = pdf_body_faces();
    let mut graph = DocumentGraph::new(DocumentId(98));
    // `typst` has a lowering, so the unsupported-language branch needs a tag
    // that really has none: `asciimath` is kept as preserved source.
    graph.push_block(Block::Math { formula: MathFormula::unsupported("x", Some("asciimath".into()), MathDisplay::Display, "asciimath") });
    write_document_pdf(&mut graph, &faces).expect("PDF write keeps the authored source");
    let codes = pdf_loss_codes(&graph);
    assert!(codes.iter().any(|code| code == "vutex.layout.math.unsupported_language"), "{codes:?}");
}

#[test]
fn pdf_export_reports_an_unparsed_footnote_definition() {
    let faces = pdf_body_faces();
    let mut graph = DocumentGraph::new(DocumentId(98));
    graph.push_block(Block::Opaque {
        kind: "footnote_definition".into(),
        payload_hint: "a footnote without the [^label]: separator".into(),
        status: SemanticStatus::Resolved,
    });
    write_document_pdf(&mut graph, &faces).expect("PDF write");
    let codes = pdf_loss_codes(&graph);
    assert!(codes.iter().any(|code| code == "vutex.layout.footnote_definition_unparsed"), "{codes:?}");
}

#[test]
fn pdf_export_keeps_layout_environment_warnings_out_of_the_report() {
    use document_fonts::{BodyFace, BodyFaceSet, BodyFamily, BodyScript};
    let base = pdf_body_faces();
    let mut faces = base.faces().to_vec();
    // The damaged face takes the `Mono/Latin/Bold` triple, which the pack does
    // not carry: `BodyFaceSet::new` rejects a repeated registration, so the
    // damaged entry must be a face of its own.
    faces.push(BodyFace::new(BodyFamily::Mono, BodyScript::Latin, document_fonts::BodyFaceStyle::Bold, vec![0u8, 1, 2, 3]));
    let set = std::sync::Arc::new(BodyFaceSet::new(faces).expect("distinct face triples"));
    let mut graph = DocumentGraph::new(DocumentId(98));
    graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "Hello".into() }] });
    write_document_pdf(&mut graph, &set).expect("the readable faces still serve the page");
    assert!(pdf_loss_codes(&graph).is_empty(), "font metrics are logger-only: {:?}", pdf_loss_codes(&graph));
}

#[test]
fn pdf_export_keeps_diagram_parser_errors_out_of_the_report() {
    let faces = pdf_body_faces();
    let mut graph = DocumentGraph::new(DocumentId(98));
    graph.push_block(Block::Diagram { language: "mermaid".into(), source: "flowchart TD\n  A[".into() });
    write_document_pdf(&mut graph, &faces).expect("PDF write keeps the authored source text");
    let codes = pdf_loss_codes(&graph);
    assert!(codes.is_empty(), "diagram parser errors are logger-only, the source text stays visible: {codes:?}");
}

#[test]
fn pdf_export_stays_clean_for_a_plain_paragraph() {
    let faces = pdf_body_faces();
    let mut graph = DocumentGraph::new(DocumentId(98));
    graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "Hello".into() }] });
    write_document_pdf(&mut graph, &faces).expect("PDF write");
    assert!(pdf_loss_codes(&graph).is_empty(), "{:?}", pdf_loss_codes(&graph));
}

#[test]
fn html_export_reports_every_projection_loss() {
    let mut graph = DocumentGraph::new(DocumentId(97));
    graph.push_block(Block::Paragraph {
        content: vec![
            Inline::Citation { key: "ref1".into(), display: Some(vec![Inline::Text { text: "[1]".into() }]) },
            Inline::Link {
                target: RelationEndpoint::Unresolved { label: "missing".into() },
                display: vec![Inline::Text { text: "here".into() }],
                title: None,
            },
            image_with_alt("alt", "missing.png"),
        ],
    });
    graph.push_block(Block::Opaque { kind: "custom_block".into(), payload_hint: "payload".into(), status: SemanticStatus::Resolved });
    write_document_html(&mut graph, &std::collections::HashMap::new()).expect("HTML write");
    let codes: Vec<String> = graph.coverage.loss.iter().map(|loss| loss.code.clone()).collect();
    for expected in
        ["export.html.citation_degraded", "export.html.unresolved_link", "export.html.image_asset_unresolved", "export.html.opaque_projection"]
    {
        assert!(codes.iter().any(|code| code == expected), "expected {expected}: {codes:?}");
    }
}

#[test]
fn html_export_stays_clean_for_a_plain_paragraph() {
    let mut graph = DocumentGraph::new(DocumentId(97));
    graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "Hello".into() }] });
    write_document_html(&mut graph, &std::collections::HashMap::new()).expect("HTML write");
    assert!(graph.coverage.loss.is_empty(), "{:?}", graph.coverage.loss);
}

fn all_text(graph: &DocumentGraph) -> String {
    let mut text = String::new();
    for node in &graph.blocks {
        match &node.block {
            Block::Section { title, .. } => text.push_str(&title.iter().map(Inline::plain_text).collect::<String>()),
            Block::Paragraph { content } => text.push_str(&content.iter().map(Inline::plain_text).collect::<String>()),
            Block::List { items, .. } => {
                for item in items {
                    text.push_str(&item.content.iter().map(Inline::plain_text).collect::<String>());
                }
            }
            Block::Table { caption, .. } => {
                if let Some(caption) = caption {
                    text.push_str(caption);
                }
            }
            _ => {}
        }
        text.push('\n');
    }
    text
}

#[test]
fn pdf_export_names_reference_target_when_the_page_cannot_carry_it() {
    let faces = pdf_body_faces();
    let mut graph = DocumentGraph::new(DocumentId(9));
    let target = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "target".into() }] });
    graph.push_block(Block::Paragraph { content: vec![Inline::Reference { display: "go".into(), target }] });

    let _pdf = write_document_pdf(&mut graph, &faces).expect("write PDF");
    let codes: Vec<&str> = graph.coverage.loss.iter().map(|loss| loss.code.as_str()).collect();
    assert!(codes.contains(&"export.pdf.reference_target_dropped"), "the report must name the dropped target: {codes:?}");
}

#[test]
fn markdown_reference_link_convert_report_matches_loss_count() {
    let source = b"Read [guide][docs].\n\n[docs]: https://example.com/guide \"Docs\"\n".to_vec();
    let output = convert_bytes("markdown", "markdown", "reference.md", source, None).expect("convert");

    assert_convert_report(&output, "markdown");
    assert_eq!(output.loss_count, 0, "resolved reference definition title is preserved in IR");
    assert!(output.report_json.contains("\"complete\":true"));
}

#[test]
fn markdown_missing_image_to_docx_reports_import_and_export_loss() {
    let output = convert_bytes("markdown", "docx", "images.md", b"A ![alt](pic.png)\n".to_vec(), None).expect("convert markdown to docx");

    assert_convert_report(&output, "docx");
    assert_report_contains_loss_codes(&output.report_json, &["import.markdown.asset_unresolved", "export.docx.image_unresolved"]);
    let report = serde_json::from_str::<serde_json::Value>(&output.report_json).expect("report");
    assert_eq!(report["status"].as_str(), Some("success_with_loss"), "{report}");
}

#[test]
fn markdown_missing_image_to_pdf_reports_import_and_export_loss() {
    let faces = pdf_body_faces();
    let output = convert_bytes_with_body_faces("markdown", "pdf", "images.md", b"A ![alt](pic.png)\n".to_vec(), None, &faces)
        .expect("convert markdown to pdf");

    assert_convert_report(&output, "pdf");
    assert_report_contains_loss_codes(&output.report_json, &["import.markdown.asset_unresolved", "export.pdf.image_unresolved"]);
    let report = serde_json::from_str::<serde_json::Value>(&output.report_json).expect("report");
    assert_eq!(report["status"].as_str(), Some("success_with_loss"), "{report}");
}

#[test]
fn pdf_export_paints_raw_html_blocks_and_inline_markup() {
    let faces = pdf_body_faces();
    let mut graph = DocumentGraph::new(DocumentId(12));
    graph.push_block(Block::RawHtml { html: "<div>ZZ_RAWBLOCK_ZZ <b>BOLD_RAWBLOCK_ZZ</b></div>".into() });
    graph.push_block(Block::Paragraph {
        content: vec![
            Inline::Text { text: "Before ".into() },
            Inline::RawHtml { html: "<span>ZZ_RAWINLINE_ZZ</span>".into() },
            Inline::Text { text: " after.".into() },
        ],
    });

    let pdf = write_document_pdf(&mut graph, &faces).expect("write PDF");
    let losses: Vec<&document_ir::LossMarker> =
        graph.coverage.loss.iter().filter(|loss| loss.code == "export.pdf.raw_html_flattened").collect();
    assert_eq!(losses.len(), 2, "the block and the inline each name the missing HTML slot: {:?}", graph.coverage.loss);
    assert!(
        losses[0].message.contains("`<div>`") && losses[0].message.contains("`<b>`"),
        "the block loss must name the dropped tags: {}",
        losses[0].message
    );
    assert!(losses[1].message.contains("`<span>`"), "the inline loss must name the dropped tag: {}", losses[1].message);

    let reopened = convert_bytes("pdf", "markdown", "raw.pdf", pdf, None).expect("reopen PDF");
    assert!(reopened.markdown.contains("ZZ_RAWBLOCK_ZZ BOLD_RAWBLOCK_ZZ"), "{}", reopened.markdown);
    assert!(reopened.markdown.contains("ZZ_RAWINLINE_ZZ"), "{}", reopened.markdown);
    assert!(!reopened.markdown.contains("<div>"), "the element markup must not be painted as text: {}", reopened.markdown);
    assert!(!reopened.markdown.contains("<b>"), "the element markup must not be painted as text: {}", reopened.markdown);
    assert!(!reopened.markdown.contains("<span>"), "the element markup must not be painted as text: {}", reopened.markdown);
}

#[test]
fn pdf_export_of_plain_text_does_not_report_raw_html() {
    let faces = pdf_body_faces();
    let mut graph = DocumentGraph::new(DocumentId(13));
    graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "Plain body.".into() }] });

    write_document_pdf(&mut graph, &faces).expect("write PDF");
    let codes: Vec<&str> = graph.coverage.loss.iter().map(|loss| loss.code.as_str()).collect();
    assert!(!codes.contains(&"export.pdf.raw_html_flattened"), "{codes:?}");
}

#[test]
fn docx_export_keeps_raw_html_text_and_reports_flattening() {
    let mut graph = DocumentGraph::new(DocumentId(14));
    graph.push_block(Block::RawHtml { html: "<div>ZZ_DOCXRAWBLOCK_ZZ <b>BOLD_DOCXRAWBLOCK_ZZ</b></div>".into() });
    graph.push_block(Block::Paragraph {
        content: vec![
            Inline::Text { text: "Before ".into() },
            Inline::RawHtml { html: "<span>ZZ_DOCXRAWINLINE_ZZ</span>".into() },
            Inline::Text { text: " after.".into() },
        ],
    });

    let docx = write_document_docx(&mut graph).expect("write DOCX");
    let loss = graph
        .coverage
        .loss
        .iter()
        .find(|loss| loss.code == "export.docx.raw_html_flattened")
        .unwrap_or_else(|| panic!("missing raw HTML loss: {:?}", graph.coverage.loss));
    assert!(loss.message.contains("`<div>`"), "the message must name the block tag: {}", loss.message);
    assert!(loss.message.contains("`<b>`"), "the message must name the nested tag: {}", loss.message);
    assert!(loss.message.contains("`<span>`"), "the message must name the inline tag: {}", loss.message);

    let reopened = convert_bytes("docx", "markdown", "raw.docx", docx, None).expect("reopen DOCX");
    assert!(reopened.markdown.contains("ZZ_DOCXRAWBLOCK_ZZ BOLD_DOCXRAWBLOCK_ZZ"), "{}", reopened.markdown);
    assert!(reopened.markdown.contains("ZZ_DOCXRAWINLINE_ZZ"), "{}", reopened.markdown);
    assert!(!reopened.markdown.contains("<div>"), "the element markup must not survive as text: {}", reopened.markdown);
}

#[test]
fn pdf_export_turns_internal_node_links_into_destination_annotations() {
    let faces = pdf_body_faces();
    let mut graph = DocumentGraph::new(DocumentId(16));
    let body = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "Target body.".into() }] });
    let section = graph.push_block(Block::Section { level: 1, title: vec![Inline::Text { text: "Intro".into() }], children: vec![body] });
    graph.push_block(Block::Paragraph {
        content: vec![
            Inline::Text { text: "Go ".into() },
            Inline::Link { target: RelationEndpoint::Node(section), display: vec![Inline::Text { text: "back".into() }], title: None },
        ],
    });

    let pdf = write_document_pdf(&mut graph, &faces).expect("write PDF");
    let text = String::from_utf8_lossy(&pdf);
    let links = text.matches("/Subtype /Link").count();
    let destinations = text.matches("/Dest [").count();
    assert!(links >= 1, "an internal link needs a /Subtype /Link annotation, saw {links}");
    assert!(destinations >= 1, "the annotation needs a /Dest jump, saw {destinations}");
    let codes: Vec<&str> = graph.coverage.loss.iter().map(|loss| loss.code.as_str()).collect();
    assert!(!codes.contains(&"export.pdf.reference_target_dropped"), "{codes:?}");
}

#[test]
fn pdf_export_resolves_authored_fragment_links_to_heading_destinations() {
    let faces = pdf_body_faces();
    let mut graph = DocumentGraph::new(DocumentId(16));
    let body = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "Target body.".into() }] });
    graph.push_block(Block::Section { level: 1, title: vec![Inline::Text { text: "Intro".into() }], children: vec![body] });
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Link {
            target: RelationEndpoint::ExternalUri("#intro".to_string()),
            display: vec![Inline::Text { text: "jump".into() }],
            title: None,
        }],
    });

    let pdf = write_document_pdf(&mut graph, &faces).expect("write PDF");
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.contains("/Subtype /Link"), "a fragment link needs an annotation");
    assert!(text.contains("/Dest ["), "a fragment link needs a page jump");
    assert!(!text.contains("/URI (#intro)"), "a fragment is not an external URI");
}

#[test]
fn pdf_export_drops_a_fragment_destination_no_anchor_satisfies() {
    let faces = pdf_body_faces();
    let mut graph = DocumentGraph::new(DocumentId(16));
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Link {
            target: RelationEndpoint::ExternalUri("#missing".to_string()),
            display: vec![Inline::Text { text: "jump".into() }],
            title: None,
        }],
    });

    let pdf = write_document_pdf(&mut graph, &faces).expect("write PDF");
    let text = String::from_utf8_lossy(&pdf);
    assert!(!text.contains("/Subtype /Link"), "an unresolved fragment must not fabricate a destination");
    assert!(!text.contains("/Dest ["), "an unresolved fragment has nothing to jump to");
}

#[test]
fn pdf_export_keeps_external_links_as_uri_actions() {
    let faces = pdf_body_faces();
    let mut graph = DocumentGraph::new(DocumentId(16));
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Link {
            target: RelationEndpoint::ExternalUri("https://example.com/guide".to_string()),
            display: vec![Inline::Text { text: "guide".into() }],
            title: Some("Read the guide".to_string()),
        }],
    });

    let pdf = write_document_pdf(&mut graph, &faces).expect("write PDF");
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.contains("/Subtype /Link"), "an external link needs an annotation");
    assert!(text.contains("/S /URI /URI (https://example.com/guide)"), "the external destination survives");
    assert!(!text.contains("/Dest ["), "an external link is not an in-document jump");
}

#[test]
fn pdf_export_names_an_unknown_inline_style_and_keeps_the_text() {
    let faces = pdf_body_faces();
    let mut graph = DocumentGraph::new(DocumentId(33));
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Styled { style: "zzstyle".into(), children: vec![Inline::Text { text: "ZZR53ZZ".into() }] }],
    });

    let pdf = write_document_pdf(&mut graph, &faces).expect("write PDF");
    let reopened = convert_bytes("pdf", "markdown", "style.pdf", pdf, None).expect("reopen PDF");
    assert!(reopened.markdown.contains("ZZR53ZZ"), "the wrapped text stays on the page: {}", reopened.markdown);
    let loss = graph
        .coverage
        .loss
        .iter()
        .find(|loss| loss.code == "export.pdf.inline_style_dropped")
        .unwrap_or_else(|| panic!("missing inline style loss: {:?}", graph.coverage.loss));
    assert_eq!(loss.status, SemanticStatus::Partial);
    assert!(loss.message.contains("zzstyle"), "the message must name the style: {}", loss.message);
}

#[test]
fn pdf_export_of_a_known_style_reports_no_style_loss() {
    let faces = pdf_body_faces();
    let mut graph = DocumentGraph::new(DocumentId(34));
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Styled { style: "bold".into(), children: vec![Inline::Text { text: "ZZ_BOLD_ZZ".into() }] }],
    });

    write_document_pdf(&mut graph, &faces).expect("write PDF");
    let codes: Vec<&str> = graph.coverage.loss.iter().map(|loss| loss.code.as_str()).collect();
    assert!(!codes.contains(&"export.pdf.inline_style_dropped"), "{codes:?}");
}

#[test]
fn pdf_export_names_the_opaque_kind_and_keeps_the_payload_text() {
    let faces = pdf_body_faces();
    let mut graph = DocumentGraph::new(DocumentId(31));
    graph.push_block(Block::Opaque { kind: "zz-kind".into(), payload_hint: "ZZ_OPAQUEHINT_ZZ".into(), status: SemanticStatus::Resolved });

    let pdf = write_document_pdf(&mut graph, &faces).expect("write PDF");
    let reopened = convert_bytes("pdf", "markdown", "opaque.pdf", pdf, None).expect("reopen PDF");
    assert!(reopened.markdown.contains("ZZ_OPAQUEHINT_ZZ"), "the payload text stays on the page: {}", reopened.markdown);
    let loss = graph
        .coverage
        .loss
        .iter()
        .find(|loss| loss.code == "export.pdf.opaque_metadata_dropped")
        .unwrap_or_else(|| panic!("missing opaque kind loss: {:?}", graph.coverage.loss));
    assert_eq!(loss.status, SemanticStatus::Partial);
    assert!(loss.message.contains("zz-kind"), "the message must name the kind: {}", loss.message);
}

#[test]
fn pdf_export_of_a_plain_paragraph_reports_no_opaque_loss() {
    let faces = pdf_body_faces();
    let mut graph = DocumentGraph::new(DocumentId(32));
    graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "ZZ_PLAIN_ZZ".into() }] });

    write_document_pdf(&mut graph, &faces).expect("write PDF");
    let codes: Vec<&str> = graph.coverage.loss.iter().map(|loss| loss.code.as_str()).collect();
    assert!(!codes.contains(&"export.pdf.opaque_metadata_dropped"), "{codes:?}");
}

#[test]
fn pdf_export_names_the_code_language_it_cannot_carry() {
    let faces = pdf_body_faces();
    let mut graph = DocumentGraph::new(DocumentId(29));
    graph.push_block(Block::Code { language: Some("zzlang".into()), content: "ZZ_CODE_ZZ".into() });

    let pdf = write_document_pdf(&mut graph, &faces).expect("write PDF");
    let reopened = convert_bytes("pdf", "markdown", "code.pdf", pdf, None).expect("reopen PDF");
    assert!(reopened.markdown.contains("ZZ_CODE_ZZ"), "the code text must stay on the page: {}", reopened.markdown);
    let loss = graph
        .coverage
        .loss
        .iter()
        .find(|loss| loss.code == "export.pdf.code_language_dropped")
        .unwrap_or_else(|| panic!("missing code language loss: {:?}", graph.coverage.loss));
    assert_eq!(loss.status, SemanticStatus::Partial);
    assert!(loss.message.contains("zzlang"), "the message must name the language: {}", loss.message);
}

#[test]
fn pdf_export_of_a_language_less_code_block_reports_no_language_loss() {
    let faces = pdf_body_faces();
    let mut graph = DocumentGraph::new(DocumentId(30));
    graph.push_block(Block::Code { language: None, content: "ZZ_CODE_ZZ".into() });

    write_document_pdf(&mut graph, &faces).expect("write PDF");
    let codes: Vec<&str> = graph.coverage.loss.iter().map(|loss| loss.code.as_str()).collect();
    assert!(!codes.contains(&"export.pdf.code_language_dropped"), "{codes:?}");
}

#[test]
fn pdf_export_keeps_attributed_text_and_names_the_dropped_attributes() {
    let faces = pdf_body_faces();
    let mut graph = DocumentGraph::new(DocumentId(25));
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Attributed {
            attributes: Attributes {
                id: Some("zzr49-id".into()),
                classes: vec!["zzr49-class".into()],
                pairs: vec![AttributePair { key: "data-zz".into(), value: "zzr49-pair".into() }],
            },
            children: vec![Inline::Text { text: "ZZR49ZZ".into() }],
        }],
    });

    let pdf = write_document_pdf(&mut graph, &faces).expect("write PDF");
    let reopened = convert_bytes("pdf", "markdown", "attributes.pdf", pdf, None).expect("reopen PDF");
    assert!(reopened.markdown.contains("ZZR49ZZ"), "the wrapped text must stay on the page: {}", reopened.markdown);
    let loss = graph
        .coverage
        .loss
        .iter()
        .find(|loss| loss.code == "export.pdf.inline_attributes_dropped")
        .unwrap_or_else(|| panic!("missing attribute loss: {:?}", graph.coverage.loss));
    assert_eq!(loss.status, SemanticStatus::Partial);
    for name in ["id", "classes", "pairs"] {
        assert!(loss.message.contains(name), "the message must name `{name}`: {}", loss.message);
    }
}

#[test]
fn pdf_export_of_a_plain_paragraph_reports_no_attribute_loss() {
    let faces = pdf_body_faces();
    let mut graph = DocumentGraph::new(DocumentId(26));
    graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "ZZ_PLAIN_ZZ".into() }] });

    write_document_pdf(&mut graph, &faces).expect("write PDF");
    let codes: Vec<&str> = graph.coverage.loss.iter().map(|loss| loss.code.as_str()).collect();
    assert!(!codes.contains(&"export.pdf.inline_attributes_dropped"), "{codes:?}");

    let mut empty = DocumentGraph::new(DocumentId(27));
    empty.push_block(Block::Paragraph {
        content: vec![Inline::Attributed { attributes: Attributes::default(), children: vec![Inline::Text { text: "ZZ_EMPTY_ZZ".into() }] }],
    });
    write_document_pdf(&mut empty, &faces).expect("write PDF");
    let codes: Vec<&str> = empty.coverage.loss.iter().map(|loss| loss.code.as_str()).collect();
    assert!(!codes.contains(&"export.pdf.inline_attributes_dropped"), "an empty wrapper carries no attribute: {codes:?}");
}

#[test]
fn docx_export_keeps_the_inline_id_and_first_class_and_names_the_rest() {
    let mut graph = DocumentGraph::new(DocumentId(28));
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Attributed {
            attributes: Attributes {
                id: Some("zzr49-id".into()),
                classes: vec!["zzr49-class".into(), "zzr49-extra".into()],
                pairs: vec![AttributePair { key: "data-zz".into(), value: "zzr49-pair".into() }],
            },
            children: vec![Inline::Text { text: "ZZR49ZZ".into() }],
        }],
    });

    let docx = write_document_docx(&mut graph).expect("write DOCX");
    let xml = String::from_utf8_lossy(&docx);
    assert!(xml.contains("zzr49-id"), "the id must survive as a bookmark: {xml}");
    assert!(xml.contains("zzr49-class"), "the first class must survive as a run style: {xml}");
    let loss = graph
        .coverage
        .loss
        .iter()
        .find(|loss| loss.code == "export.docx.inline_attributes_dropped")
        .unwrap_or_else(|| panic!("missing attribute loss: {:?}", graph.coverage.loss));
    assert_eq!(loss.status, SemanticStatus::Partial);
    assert!(loss.message.contains("data-zz"), "the message must name the dropped pair: {}", loss.message);
    assert!(loss.message.contains("zzr49-extra"), "the message must name the dropped class: {}", loss.message);
}

#[test]
fn docx_export_of_plain_text_does_not_report_raw_html() {
    let mut graph = DocumentGraph::new(DocumentId(15));
    graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "Plain body.".into() }] });

    write_document_docx(&mut graph).expect("write DOCX");
    let codes: Vec<&str> = graph.coverage.loss.iter().map(|loss| loss.code.as_str()).collect();
    assert!(!codes.contains(&"export.docx.raw_html_flattened"), "{codes:?}");
    assert!(!codes.contains(&"export.docx.inline_attributes_dropped"), "{codes:?}");
}
