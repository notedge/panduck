use notedown_formats::import::markdown::import_markdown_bytes;
use panduck_convert::convert_bytes;

mod common;
use common::{assert_convert_report, assert_report_contains_loss_codes, docx_zip, docx_zip_with_footnotes, stored_zip};

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
    assert_report_contains_loss_codes(&output.report_json, &["import.html.partial_coverage", "import.html.unsupported_script"]);
    let reopened = import_markdown_bytes("round.md", &output.markdown).expect("reopen markdown");
    assert!(reopened.validate().is_valid());
}

#[test]
fn docx_field_to_markdown_convert_report_lists_field_loss() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText> PAGE </w:instrText></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r><w:r><w:t>visible fallback</w:t></w:r></w:p></w:body>
</w:document>"#;
    let output =
        convert_bytes("docx", "markdown", "field.docx", docx_zip(document_xml), None).expect("convert docx with field");

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
    let output = convert_bytes("docx", "markdown", "footnote.docx", docx_zip(document_xml), None)
        .expect("convert docx with unresolved footnote");

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
    let output =
        convert_bytes("docx", "markdown", "footnote-resolved.docx", docx_zip_with_footnotes(document_xml, footnotes_xml), None)
            .expect("convert docx with resolved footnote");

    assert_convert_report(&output, "markdown");
    assert_report_contains_loss_codes(&output.report_json, &["import.docx.partial_coverage"]);
    let report = serde_json::from_str::<serde_json::Value>(&output.report_json).expect("report");
    let loss_codes =
        report["losses"].as_array().expect("losses").iter().filter_map(|loss| loss["code"].as_str()).collect::<Vec<_>>();
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
    let loss_codes =
        report["losses"].as_array().expect("losses").iter().filter_map(|loss| loss["code"].as_str()).collect::<Vec<_>>();
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
fn doc_to_markdown_convert_report_lists_partial_coverage() {
    let output = convert_bytes("doc", "markdown", "legacy.doc", notedown_fixtures::acorn::doc::LEGACY_PLAIN_DOC.to_vec(), None)
        .expect("convert doc to markdown");

    assert_convert_report(&output, "markdown");
    assert_report_contains_loss_codes(&output.report_json, &["import.doc.partial_coverage"]);
}

#[test]
fn pdf_to_markdown_convert_report_lists_partial_coverage() {
    let pdf = convert_bytes("markdown", "pdf", "sample.md", b"# Heading\n\nBody text.\n".to_vec(), None)
        .expect("write PDF")
        .binary
        .expect("PDF bytes");
    let output = convert_bytes("pdf", "markdown", "sample.pdf", pdf, None).expect("convert pdf to markdown");

    assert_convert_report(&output, "markdown");
    assert_report_contains_loss_codes(&output.report_json, &["import.pdf.partial_coverage"]);
}

#[test]
fn markdown_reference_link_convert_report_matches_loss_count() {
    let source = b"Read [guide][docs].\n\n[docs]: https://example.com/guide \"Docs\"\n".to_vec();
    let output = convert_bytes("markdown", "markdown", "reference.md", source, None).expect("convert");

    assert_convert_report(&output, "markdown");
    assert_eq!(output.loss_count, 0, "resolved reference definition title is preserved in IR");
    assert!(output.report_json.contains("\"complete\":true"));
}
