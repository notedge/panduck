use std::fs;

use notedown_formats::import::markdown::import_markdown_bytes;
use notedown_ir::{Block, Inline};
use panduck_convert::{convert_to_markdown_project, supports_markdown_project_source};

mod common;
use common::{assert_convert_project_report, assert_report_contains_loss_codes, stored_zip};

/// Assert that an asset path matches the content-addressed pattern
/// `assets/{16-hex-chars}.{ext}`.
fn assert_content_addressed(path: &str, expected_ext: &str) {
    let expected_prefix = "assets/";
    assert!(path.starts_with(expected_prefix), "path {path} must start with assets/");
    let rest = &path[expected_prefix.len()..];
    let (hex, ext) = rest.rsplit_once('.').expect("path must have extension");
    assert_eq!(ext, expected_ext, "extension mismatch for {path}");
    assert_eq!(hex.len(), 16, "fingerprint prefix must be 16 hex chars in {path}");
    assert!(hex.chars().all(|c| c.is_ascii_hexdigit()), "must be hex in {path}");
}

#[test]
fn docx_to_markdown_project_writes_index_assets_and_report() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
            xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
            xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing">
  <w:body>
    <w:p>
      <w:r>
        <w:drawing>
          <wp:inline>
            <wp:docPr descr="Logo"/>
            <a:graphic>
              <a:graphicData>
                <a:blip r:embed="rId2"/>
              </a:graphicData>
            </a:graphic>
          </wp:inline>
        </w:drawing>
      </w:r>
    </w:p>
  </w:body>
</w:document>"#;
    let rels_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2"
    Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
    Target="media/logo.png"/>
</Relationships>"#;
    let zip = stored_zip(&[
        ("word/document.xml", document_xml),
        ("word/_rels/document.xml.rels", rels_xml),
        ("word/media/logo.png", b"\x89PNG\r\n"),
    ]);

    let output_dir = std::env::temp_dir().join(format!("panduck-md-project-{}", std::process::id()));
    let (output, published) = convert_to_markdown_project("docx", "image.docx", zip, &output_dir).expect("convert project");

    assert_eq!(output.published_assets.len(), 1);
    let asset_path = &output.published_assets[0];
    assert_content_addressed(asset_path, "png");
    assert!(output.index_markdown.contains(&format!("![Logo]({asset_path})")));
    assert!(output.unresolved_assets.is_empty());
    assert_convert_project_report(&output, &published);
    assert_report_contains_loss_codes(&output.report_json, &["import.docx.partial_coverage"]);

    let index = fs::read_to_string(published.index_path).expect("read index.md");
    assert!(index.contains(&format!("![Logo]({asset_path})")));
    let asset_bytes = fs::read(output_dir.join(asset_path)).expect("read asset");
    assert_eq!(asset_bytes, b"\x89PNG\r\n");

    let reopened = import_markdown_bytes("index.md", &index).expect("oak markdown reopen");
    assert!(reopened.validate().is_valid());
    assert!(reopened.blocks.iter().any(|node| matches!(
        &node.block,
        Block::Paragraph { content }
            if content.iter().any(|inline| matches!(
                inline,
                Inline::Image { source, alt, .. }
                    if source == asset_path
                        && alt.iter().any(|child| matches!(child, Inline::Text { text } if text == "Logo"))
            ))
    )));

    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn docx_to_markdown_project_materializes_jpeg_embedded_image() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
            xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
            xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing">
  <w:body>
    <w:p>
      <w:r>
        <w:drawing>
          <wp:inline>
            <wp:docPr descr="Photo"/>
            <a:graphic>
              <a:graphicData>
                <a:blip r:embed="rId7"/>
              </a:graphicData>
            </a:graphic>
          </wp:inline>
        </wp:inline>
      </w:drawing>
    </w:r>
  </w:p>
</w:body>
</w:document>"#;
    let rels_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId7"
    Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
    Target="media/photo.jpg"/>
</Relationships>"#;
    let jpeg_bytes = b"\xFF\xD8\xFF\xE0\x00\x10JFIF\x00photo";
    let zip = stored_zip(&[
        ("word/document.xml", document_xml),
        ("word/_rels/document.xml.rels", rels_xml),
        ("word/media/photo.jpg", jpeg_bytes),
    ]);

    let output_dir = std::env::temp_dir().join(format!("panduck-docx-jpeg-{}", std::process::id()));
    let (output, published) = convert_to_markdown_project("docx", "jpeg.docx", zip, &output_dir).expect("convert");

    assert_eq!(output.published_assets.len(), 1);
    let asset_path = &output.published_assets[0];
    assert_content_addressed(asset_path, "jpg");
    assert!(output.index_markdown.contains(&format!("![Photo]({asset_path})")));
    assert!(output.unresolved_assets.is_empty());

    let asset_on_disk = fs::read(output_dir.join(asset_path)).expect("read jpeg asset");
    assert_eq!(asset_on_disk, jpeg_bytes);

    let index = fs::read_to_string(published.index_path).expect("read index");
    let reopened = import_markdown_bytes("index.md", &index).expect("oak markdown reopen");
    assert!(reopened.validate().is_valid());
    assert!(reopened.blocks.iter().any(|node| matches!(
        &node.block,
        Block::Paragraph { content }
            if content.iter().any(|inline| matches!(
                inline,
                Inline::Image { source, alt, .. }
                    if source == asset_path
                        && alt.iter().any(|child| matches!(child, Inline::Text { text } if text == "Photo"))
            ))
    )));

    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn docx_to_markdown_project_is_deterministic_across_repeated_runs() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
            xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
            xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing">
  <w:body>
    <w:p><w:r><w:t>Hello DOCX</w:t></w:r></w:p>
    <w:p>
      <w:r>
        <w:drawing>
          <wp:inline>
            <wp:docPr descr="Banner"/>
            <a:graphic><a:graphicData><a:blip r:embed="rId1"/></a:graphicData></a:graphic>
          </wp:inline>
        </wp:inline>
      </w:drawing>
    </w:r>
  </w:p>
</w:body>
</w:document>"#;
    let rels_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
    Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
    Target="media/banner.png"/>
</Relationships>"#;
    let zip = stored_zip(&[
        ("word/document.xml", document_xml),
        ("word/_rels/document.xml.rels", rels_xml),
        ("word/media/banner.png", b"\x89PNG\x01\x02\x03"),
    ]);

    let base = std::env::temp_dir().join(format!("panduck-docx-deterministic-{}", std::process::id()));
    let dir_a = base.join("a");
    let dir_b = base.join("b");

    let (output_a, published_a) = convert_to_markdown_project("docx", "same.docx", zip.clone(), &dir_a).expect("a");
    let (output_b, published_b) = convert_to_markdown_project("docx", "same.docx", zip, &dir_b).expect("b");

    // Content-addressed asset paths must be identical across runs.
    assert_eq!(output_a.published_assets, output_b.published_assets);
    // Index markdown body must be byte-identical.
    assert_eq!(output_a.index_markdown, output_b.index_markdown);
    // Report JSON must be identical (no timestamps or nondeterministic fields).
    assert_eq!(output_a.report_json, output_b.report_json);
    assert_eq!(published_a.index_path, dir_a.join("index.md"));
    assert_eq!(fs::read_to_string(&published_a.index_path).expect("a"),
               fs::read_to_string(&published_b.index_path).expect("b"));

    let _ = fs::remove_dir_all(&base);
}

fn minimal_epub_zip() -> Vec<u8> {
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
    stored_zip(&[
        ("mimetype", b"application/epub+zip"),
        ("META-INF/container.xml", container),
        ("OEBPS/content.opf", opf),
        (
            "OEBPS/chapter.xhtml",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <body>
    <h1>Chapter One</h1>
    <p>Hello EPUB</p>
  </body>
</html>"#,
        ),
    ])
}

#[test]
fn epub_to_markdown_project_writes_index_and_reopens_semantics() {
    assert!(supports_markdown_project_source("epub"));
    let output_dir = std::env::temp_dir().join(format!("panduck-epub-project-{}", std::process::id()));
    let (output, published) =
        convert_to_markdown_project("epub", "book.epub", minimal_epub_zip(), &output_dir).expect("convert epub project");

    assert!(output.index_markdown.contains("# Chapter One"));
    assert!(output.index_markdown.contains("Hello EPUB"));
    assert!(output.published_assets.is_empty());
    assert!(published.index_path.exists());
    assert_convert_project_report(&output, &published);
    assert_report_contains_loss_codes(&output.report_json, &["import.epub.partial_coverage"]);

    let index = fs::read_to_string(published.index_path).expect("read index.md");
    let reopened = import_markdown_bytes("index.md", &index).expect("reopen markdown");
    assert!(reopened.validate().is_valid());
    assert!(reopened.blocks.iter().any(|node| matches!(
        &node.block,
        Block::Section { title, .. }
            if title.iter().any(|inline| matches!(inline, Inline::Text { text } if text == "Chapter One"))
    )));
    assert!(reopened.blocks.iter().any(|node| matches!(
        &node.block,
        Block::Paragraph { content }
            if content.iter().any(|inline| matches!(inline, Inline::Text { text } if text == "Hello EPUB"))
    )));

    let _ = fs::remove_dir_all(&output_dir);
}

fn epub_with_image_zip() -> Vec<u8> {
    let container = br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;
    let opf = br#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Image Book</dc:title>
    <dc:language>en</dc:language>
  </metadata>
  <manifest>
    <item id="cover" href="images/cover.png" media-type="image/png" properties="cover-image"/>
    <item id="ch1" href="chapter.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="ch1"/>
  </spine>
</package>"#;
    stored_zip(&[
        ("mimetype", b"application/epub+zip"),
        ("META-INF/container.xml", container),
        ("OEBPS/content.opf", opf),
        (
            "OEBPS/chapter.xhtml",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <body>
    <p>Before image</p>
    <img src="images/cover.png" alt="Cover art"/>
  </body>
</html>"#,
        ),
        ("OEBPS/images/cover.png", b"\x89PNG\r\n"),
    ])
}

#[test]
fn epub_to_markdown_project_materializes_chapter_image() {
    let output_dir = std::env::temp_dir().join(format!("panduck-epub-image-project-{}", std::process::id()));
    let (output, published) =
        convert_to_markdown_project("epub", "image.epub", epub_with_image_zip(), &output_dir).expect("convert epub project");

    assert!(output.published_chapters.is_empty());
    assert_eq!(output.published_assets.len(), 1);
    let asset_path = &output.published_assets[0];
    assert_content_addressed(asset_path, "png");
    assert_convert_project_report(&output, &published);
    assert_report_contains_loss_codes(&output.report_json, &["import.epub.partial_coverage"]);
    let index = fs::read_to_string(published.index_path).expect("read index.md");
    assert!(index.contains(&format!("![Cover art]({asset_path})")));
    assert_eq!(fs::read(output_dir.join(asset_path)).expect("read asset"), b"\x89PNG\r\n");

    let _ = fs::remove_dir_all(&output_dir);
}

fn multi_chapter_epub_zip() -> Vec<u8> {
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
    <item id="ch1" href="chapter-one.xhtml" media-type="application/xhtml+xml"/>
    <item id="ch2" href="chapter-two.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="ch1"/>
    <itemref idref="ch2"/>
  </spine>
</package>"#;
    stored_zip(&[
        ("mimetype", b"application/epub+zip"),
        ("META-INF/container.xml", container),
        ("OEBPS/content.opf", opf),
        (
            "OEBPS/chapter-one.xhtml",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <body>
    <h1>Chapter One</h1>
    <p>First chapter body.</p>
  </body>
</html>"#,
        ),
        (
            "OEBPS/chapter-two.xhtml",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <body>
    <h1>Chapter Two</h1>
    <p>Second chapter body.</p>
  </body>
</html>"#,
        ),
    ])
}

#[test]
fn epub_to_markdown_project_splits_spine_into_chapters() {
    let output_dir = std::env::temp_dir().join(format!("panduck-epub-chapters-{}", std::process::id()));
    let (output, published) =
        convert_to_markdown_project("epub", "book.epub", multi_chapter_epub_zip(), &output_dir).expect("convert epub project");

    assert_eq!(output.published_chapters.len(), 2);
    assert!(output.index_markdown.contains("# Sample Book"));
    assert!(output.index_markdown.contains("chapters/000-chapter-one.md"));
    assert!(!output.index_markdown.contains("First chapter body."));

    let chapter_one = fs::read_to_string(output_dir.join("chapters/000-chapter-one.md")).expect("chapter one");
    assert!(chapter_one.contains("# Chapter One"));
    let reopened = import_markdown_bytes("chapters/000-chapter-one.md", &chapter_one).expect("reopen chapter");
    assert!(reopened.blocks.iter().any(|node| matches!(
        &node.block,
        Block::Section { title, .. }
            if title.iter().any(|inline| matches!(inline, Inline::Text { text } if text == "Chapter One"))
    )));
    assert_eq!(published.chapter_paths.len(), 2);
    assert_convert_project_report(&output, &published);
    assert_report_contains_loss_codes(&output.report_json, &["import.epub.partial_coverage"]);

    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn html_to_markdown_project_writes_index_and_reopens_semantics() {
    assert!(supports_markdown_project_source("html"));
    let output_dir = std::env::temp_dir().join(format!("panduck-html-project-{}", std::process::id()));
    let html = br#"<html><body><h1>Title</h1><p>Hello HTML.</p></body></html>"#.to_vec();
    let (output, published) =
        convert_to_markdown_project("html", "page.html", html, &output_dir).expect("convert html project");

    assert!(output.index_markdown.contains("# Title"));
    assert!(output.published_chapters.is_empty());
    assert!(published.index_path.exists());
    assert_convert_project_report(&output, &published);
    assert_report_contains_loss_codes(&output.report_json, &["import.html.partial_coverage"]);

    let index = fs::read_to_string(published.index_path).expect("read index.md");
    let reopened = import_markdown_bytes("index.md", &index).expect("reopen markdown");
    assert!(reopened.blocks.iter().any(|node| matches!(
        &node.block,
        Block::Paragraph { content }
            if content.iter().any(|inline| matches!(inline, Inline::Text { text } if text == "Hello HTML."))
    )));

    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn html_to_markdown_project_reports_unsupported_elements_in_panduck_report() {
    let output_dir = std::env::temp_dir().join(format!("panduck-html-loss-project-{}", std::process::id()));
    let html = br#"<html><body><h1>Title</h1><p>Before script.</p><script>alert(1)</script></body></html>"#.to_vec();
    let (output, published) =
        convert_to_markdown_project("html", "page.html", html, &output_dir).expect("convert html project");

    assert_convert_project_report(&output, &published);
    assert_report_contains_loss_codes(&output.report_json, &["import.html.partial_coverage", "import.html.unsupported_script"]);

    let _ = fs::remove_dir_all(&output_dir);
}

fn legacy_plain_doc_fixture() -> &'static [u8] {
    notedown_fixtures::acorn::doc::LEGACY_PLAIN_DOC
}

#[test]
fn doc_to_markdown_project_writes_index_and_reopens_text() {
    assert!(supports_markdown_project_source("doc"));
    let output_dir = std::env::temp_dir().join(format!("panduck-doc-project-{}", std::process::id()));
    let bytes = legacy_plain_doc_fixture();
    let (output, published) =
        convert_to_markdown_project("doc", "sample.doc", bytes.to_vec(), &output_dir).expect("convert doc project");

    assert!(output.index_markdown.contains("Hello legacy DOC"));
    assert!(output.published_chapters.is_empty());
    assert!(output.loss_count > 0);
    assert_convert_project_report(&output, &published);
    assert_report_contains_loss_codes(&output.report_json, &["import.doc.partial_coverage"]);

    let index = fs::read_to_string(published.index_path).expect("read index.md");
    let reopened = import_markdown_bytes("index.md", &index).expect("reopen markdown");
    assert!(reopened.blocks.iter().any(|node| matches!(
        &node.block,
        Block::Paragraph { content }
            if content.iter().any(|inline| matches!(inline, Inline::Text { text } if text == "Second paragraph"))
    )));

    let _ = fs::remove_dir_all(&output_dir);
}

#[test]
fn pdf_to_markdown_project_writes_index_and_reopens_text() {
    use panduck_convert::convert_bytes;

    assert!(supports_markdown_project_source("pdf"));
    let pdf = convert_bytes("markdown", "pdf", "sample.md", b"# Heading\n\nBody text.\n".to_vec())
        .expect("write pdf")
        .binary
        .expect("pdf bytes");

    let output_dir = std::env::temp_dir().join(format!("panduck-pdf-project-{}", std::process::id()));
    let (output, published) = convert_to_markdown_project("pdf", "sample.pdf", pdf, &output_dir).expect("convert pdf project");

    assert!(output.index_markdown.contains("Heading"));
    assert!(output.index_markdown.contains("Body text."));
    assert!(published.index_path.exists());
    assert_convert_project_report(&output, &published);
    assert_report_contains_loss_codes(&output.report_json, &["import.pdf.partial_coverage"]);

    let index = fs::read_to_string(published.index_path).expect("read index.md");
    let reopened = import_markdown_bytes("index.md", &index).expect("reopen markdown");
    assert!(reopened.blocks.iter().any(|node| matches!(
        &node.block,
        Block::Paragraph { content }
            if content.iter().any(|inline| matches!(inline, Inline::Text { text } if text.contains("Body text.")))
    )));

    let _ = fs::remove_dir_all(&output_dir);
}
