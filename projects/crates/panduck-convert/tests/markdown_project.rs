use std::fs;

use notedown_formats::import::markdown::import_markdown_bytes;
use notedown_ir::{Block, Inline};
use panduck_convert::convert_to_markdown_project;
use panduck_convert::supports_markdown_project_source;

fn stored_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut archive = Vec::new();
    let mut central = Vec::new();

    for (path, payload) in entries {
        let name = path.as_bytes();
        let crc = crc32(payload);
        let local_offset = archive.len();

        archive.extend_from_slice(b"PK\x03\x04");
        archive.extend_from_slice(&[0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        archive.extend_from_slice(&crc.to_le_bytes());
        archive.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        archive.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        archive.extend_from_slice(&(name.len() as u16).to_le_bytes());
        archive.extend_from_slice(&[0x00, 0x00]);
        archive.extend_from_slice(name);
        archive.extend_from_slice(payload);

        central.extend_from_slice(b"PK\x01\x02");
        let mut cd_fixed = [0u8; 46];
        cd_fixed[0..2].copy_from_slice(&[0x14, 0x00]);
        cd_fixed[2..4].copy_from_slice(&[0x14, 0x00]);
        cd_fixed[12..16].copy_from_slice(&crc.to_le_bytes());
        cd_fixed[16..20].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        cd_fixed[20..24].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        cd_fixed[24..26].copy_from_slice(&(name.len() as u16).to_le_bytes());
        cd_fixed[38..42].copy_from_slice(&(local_offset as u32).to_le_bytes());
        central.extend_from_slice(&cd_fixed);
        central.extend_from_slice(name);
    }

    let cd_offset = archive.len();
    archive.extend_from_slice(&central);
    let cd_size = central.len();
    archive.extend_from_slice(b"PK\x05\x06");
    archive.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    archive.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    archive.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    archive.extend_from_slice(&(cd_size as u32).to_le_bytes());
    archive.extend_from_slice(&(cd_offset as u32).to_le_bytes());
    archive.extend_from_slice(&[0x00, 0x00]);
    archive
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let bit = crc & 1;
            crc >>= 1;
            if bit != 0 {
                crc ^= 0xEDB8_8320;
            }
        }
    }
    crc ^ 0xFFFF_FFFF
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
    let (output, published) =
        convert_to_markdown_project("docx", "image.docx", zip, &output_dir).expect("convert project");

    assert!(output.index_markdown.contains("![Logo](assets/logo.png)"));
    assert_eq!(output.published_assets, vec!["assets/logo.png".to_string()]);
    assert!(output.unresolved_assets.is_empty());
    assert!(output.report_json.contains("convert_project"));

    let index = fs::read_to_string(published.index_path).expect("read index.md");
    assert!(index.contains("![Logo](assets/logo.png)"));
    let asset_bytes = fs::read(output_dir.join("assets/logo.png")).expect("read asset");
    assert_eq!(asset_bytes, b"\x89PNG\r\n");
    assert!(published.report_path.exists());
    assert!(published.report_path.ends_with("panduck.report.json"));

    let reopened = import_markdown_bytes("index.md", &index).expect("oak markdown reopen");
    assert!(reopened.validate().is_valid());
    assert!(reopened.blocks.iter().any(|node| matches!(
        &node.block,
        Block::Paragraph { content }
            if content.iter().any(|inline| matches!(
                inline,
                Inline::Styled { style, children }
                    if style == "image"
                        && children.len() >= 2
                        && matches!(&children[0], Inline::Text { text } if text == "Logo")
                        && matches!(&children[1], Inline::Text { text } if text == "assets/logo.png")
            ))
    )));

    let _ = fs::remove_dir_all(&output_dir);
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
    let (output, published) = convert_to_markdown_project(
        "epub",
        "image.epub",
        epub_with_image_zip(),
        &output_dir,
    )
    .expect("convert epub project");

    assert!(output.published_chapters.is_empty());
    assert_eq!(output.published_assets, vec!["assets/cover.png".to_string()]);
    let index = fs::read_to_string(published.index_path).expect("read index.md");
    assert!(index.contains("![Cover art](assets/cover.png)"));
    assert_eq!(fs::read(output_dir.join("assets/cover.png")).expect("read asset"), b"\x89PNG\r\n");

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
    let (output, published) = convert_to_markdown_project(
        "epub",
        "book.epub",
        multi_chapter_epub_zip(),
        &output_dir,
    )
    .expect("convert epub project");

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

    let index = fs::read_to_string(published.index_path).expect("read index.md");
    let reopened = import_markdown_bytes("index.md", &index).expect("reopen markdown");
    assert!(reopened.blocks.iter().any(|node| matches!(
        &node.block,
        Block::Paragraph { content }
            if content.iter().any(|inline| matches!(inline, Inline::Text { text } if text == "Hello HTML."))
    )));

    let _ = fs::remove_dir_all(&output_dir);
}

fn legacy_plain_doc_fixture() -> Vec<u8> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/legacy_plain.doc");
    std::fs::read(path).expect("read legacy_plain.doc fixture")
}

#[test]
fn doc_to_markdown_project_writes_index_and_reopens_text() {
    assert!(supports_markdown_project_source("doc"));
    let output_dir = std::env::temp_dir().join(format!("panduck-doc-project-{}", std::process::id()));
    let bytes = legacy_plain_doc_fixture();
    let (output, published) =
        convert_to_markdown_project("doc", "sample.doc", bytes, &output_dir).expect("convert doc project");

    assert!(output.index_markdown.contains("Hello legacy DOC"));
    assert!(output.published_chapters.is_empty());
    assert!(output.loss_count > 0);
    assert!(output.report_json.contains("convert_project"));
    assert!(published.index_path.exists());
    assert!(published.report_path.exists());
    assert!(published.report_path.ends_with("panduck.report.json"));

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
    let (output, published) =
        convert_to_markdown_project("pdf", "sample.pdf", pdf, &output_dir).expect("convert pdf project");

    assert!(output.index_markdown.contains("Heading"));
    assert!(output.index_markdown.contains("Body text."));
    assert!(published.index_path.exists());

    let index = fs::read_to_string(published.index_path).expect("read index.md");
    let reopened = import_markdown_bytes("index.md", &index).expect("reopen markdown");
    assert!(reopened.blocks.iter().any(|node| matches!(
        &node.block,
        Block::Paragraph { content }
            if content.iter().any(|inline| matches!(inline, Inline::Text { text } if text.contains("Body text.")))
    )));

    let _ = fs::remove_dir_all(&output_dir);
}
