use document_formats::{
    export::{
        epub::{export_epub_bytes, export_epub_report},
        markdown::export_markdown,
    },
    import::epub::import_epub_bytes,
};
use document_ir::{
    Asset, AssetId, AssetKind, Block, DocumentGraph, DocumentId, DocumentMetadata, Inline, ListItem, MetadataEntry, MetadataValue,
    SemanticStatus, SourceKind, TableCell, TableRow,
};

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
        // 42 bytes after the signature (APPNOTE central directory total = 46).
        let mut cd_fixed = [0u8; 42];
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
fn epub_import_maps_metadata_and_spine_xhtml() {
    let zip = minimal_epub_zip();
    let graph = import_epub_bytes("book.epub", &zip).expect("import epub");
    assert_eq!(graph.metadata.title.as_deref(), Some("Sample Book"));
    assert_eq!(graph.metadata.language.as_deref(), Some("en"));
    assert!(graph.blocks.len() >= 2);
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("# Chapter One"));
    assert!(markdown.contains("Hello EPUB"));
}

fn epub_with_extended_metadata_zip() -> Vec<u8> {
    let container = br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;
    let opf = br#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Metadata Book</dc:title>
    <dc:language>en</dc:language>
    <dc:identifier id="uid">urn:uuid:1a2b3c4d</dc:identifier>
    <dc:publisher>Example Press</dc:publisher>
    <dc:subject>Reference</dc:subject>
    <meta property="dcterms:modified">2026-11-06T00:00:00Z</meta>
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
  <body><p>Body text</p></body>
</html>"#,
        ),
    ])
}

#[test]
fn epub_import_reports_metadata_it_cannot_model() {
    let graph = import_epub_bytes("metadata.epub", &epub_with_extended_metadata_zip()).expect("import epub");
    let loss = graph.coverage.loss.iter().find(|loss| loss.code == "import.epub.metadata_unmapped").expect("metadata loss");
    assert_eq!(loss.status, document_ir::SemanticStatus::Partial);
    for field in ["identifier", "publisher", "subject", "dcterms:modified"] {
        assert!(loss.message.contains(field), "{}", loss.message);
    }
}

#[test]
fn epub_import_keeps_modelled_metadata_loss_free() {
    let graph = import_epub_bytes("book.epub", &minimal_epub_zip()).expect("import epub");
    assert!(!graph.coverage.loss.iter().any(|loss| loss.code == "import.epub.metadata_unmapped"));
}

fn epub_with_nav_zip() -> Vec<u8> {
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
      <ol>
        <li><a href="chapter.xhtml">Chapter One</a></li>
      </ol>
    </nav>
  </body>
</html>"#;
    stored_zip(&[
        ("mimetype", b"application/epub+zip"),
        ("META-INF/container.xml", container),
        ("OEBPS/content.opf", opf),
        ("OEBPS/nav.xhtml", nav),
        (
            "OEBPS/chapter.xhtml",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <body>
    <h1>Chapter One</h1>
    <p>Body text</p>
  </body>
</html>"#,
        ),
    ])
}

#[test]
fn epub_import_maps_navigation_toc() {
    let zip = epub_with_nav_zip();
    let graph = import_epub_bytes("nav.epub", &zip).expect("import epub");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("[Chapter One](chapter.xhtml)"));
    assert!(markdown.contains("# Chapter One"));
}

fn epub_with_nested_nav_zip() -> Vec<u8> {
    let container = br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;
    let opf = br#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Nested Nav Book</dc:title>
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
      <ol>
        <li><a href="chapter.xhtml">Part One</a>
          <ol>
            <li><a href="chapter.xhtml#section">Section A</a></li>
          </ol>
        </li>
      </ol>
    </nav>
  </body>
</html>"#;
    stored_zip(&[
        ("mimetype", b"application/epub+zip"),
        ("META-INF/container.xml", container),
        ("OEBPS/content.opf", opf),
        ("OEBPS/nav.xhtml", nav),
        (
            "OEBPS/chapter.xhtml",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <body>
    <h1 id="section">Section A</h1>
    <p>Body text</p>
  </body>
</html>"#,
        ),
    ])
}

#[test]
fn epub_import_maps_nested_navigation_toc() {
    let zip = epub_with_nested_nav_zip();
    let graph = import_epub_bytes("nested-nav.epub", &zip).expect("import epub");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("[Part One](chapter.xhtml)"));
    assert!(markdown.contains("  - [Section A](chapter.xhtml#section)"));
}

fn epub_with_image_zip() -> Vec<u8> {
    epub_with_image_member(true)
}

fn epub_with_image_member(include_image: bool) -> Vec<u8> {
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
    let mut entries: Vec<(&str, &[u8])> = vec![
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
    ];
    if include_image {
        entries.push(("OEBPS/images/cover.png", b"\x89PNG\r\n"));
    }
    stored_zip(&entries)
}

#[test]
fn epub_import_registers_embedded_images() {
    let zip = epub_with_image_zip();
    let graph = import_epub_bytes("image.epub", &zip).expect("import epub");
    assert!(graph.assets.len() >= 1);
    assert!(graph.assets.iter().any(|asset| asset.bytes.is_some()));
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("![Cover art](images/cover.png)"));
    assert!(markdown.contains("Before image"));
}

#[test]
fn epub_missing_image_member_is_unresolved_and_reported() {
    let graph = import_epub_bytes("missing-image.epub", &epub_with_image_member(false)).expect("import");
    let image = graph.assets.iter().find(|asset| asset.source.as_deref() == Some("OEBPS/images/cover.png")).expect("image asset");
    assert!(image.bytes.is_none());
    assert_eq!(image.status, document_ir::SemanticStatus::Unresolved);
    assert!(
        graph
            .coverage
            .loss
            .iter()
            .any(|loss| loss.code == "import.epub.asset_unresolved" && loss.status == document_ir::SemanticStatus::Unresolved)
    );
    assert!(!graph.coverage.complete);
    assert!(graph.validate().is_valid());
}

fn epub_with_stylesheet_and_svg_zip() -> Vec<u8> {
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
    <item id="logo" href="images/logo.png" media-type="image/png"/>
  </manifest>
  <spine>
    <itemref idref="ch1"/>
  </spine>
</package>"#;
    stored_zip(&[
        ("mimetype", b"application/epub+zip"),
        ("META-INF/container.xml", container),
        ("OEBPS/content.opf", opf),
        ("OEBPS/styles/main.css", b"body { color: red; }"),
        ("OEBPS/images/logo.png", b"\x89PNG\r\n"),
        (
            "OEBPS/chapter.xhtml",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml"
      xmlns:xlink="http://www.w3.org/1999/xlink">
  <head>
    <link rel="stylesheet" href="styles/main.css"/>
  </head>
  <body>
    <p style="color: blue;">Styled text</p>
    <svg>
      <image xlink:href="images/logo.png" alt="Logo"/>
    </svg>
  </body>
</html>"#,
        ),
    ])
}

#[test]
fn epub_import_registers_stylesheet_assets() {
    let zip = epub_with_stylesheet_and_svg_zip();
    let graph = import_epub_bytes("styled.epub", &zip).expect("import epub");
    assert!(graph.assets.iter().any(|asset| asset.media_type.as_deref() == Some("text/css")));
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "import.epub.css_link_unsupported"));
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "import.epub.inline_style_unsupported"));
}

#[test]
fn epub_import_lowers_svg_external_image_references() {
    let zip = epub_with_stylesheet_and_svg_zip();
    let graph = import_epub_bytes("svg.epub", &zip).expect("import epub");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("![Logo](images/logo.png)"));
}

fn epub_with_table_and_nested_list_zip() -> Vec<u8> {
    let container = br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;
    let opf = br#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Structure Book</dc:title>
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
    <table>
      <tr><th>Name</th><th>Value</th></tr>
      <tr><td>Alpha</td><td>1</td></tr>
    </table>
    <ul>
      <li>Outer
        <ul>
          <li>Inner</li>
        </ul>
      </li>
    </ul>
  </body>
</html>"#,
        ),
    ])
}

fn epub_with_block_elements_zip() -> Vec<u8> {
    let container = br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;
    let opf = br#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Blocks Book</dc:title>
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
    <section>
      <p>Inside section</p>
    </section>
    <blockquote><p>Quoted line</p></blockquote>
    <pre><code class="language-rust">fn main() {}</code></pre>
    <hr/>
  </body>
</html>"#,
        ),
    ])
}

fn epub_with_block_cell_zip() -> Vec<u8> {
    let container = br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;
    let opf = br#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Cell Blocks Book</dc:title>
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
    <table>
      <tr><th>Header</th></tr>
      <tr><td><p>one</p><ul><li>item</li></ul></td></tr>
    </table>
  </body>
</html>"#,
        ),
    ])
}

fn first_body_cell(graph: &document_ir::DocumentGraph) -> &document_ir::TableCell {
    graph
        .blocks
        .iter()
        .find_map(|node| match &node.block {
            Block::Table { body, .. } => body.first().and_then(|row| row.cells.first()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("table body cell expected, got {:?}", graph.blocks))
}

fn cell_block_kinds(graph: &document_ir::DocumentGraph, cell: &document_ir::TableCell) -> Vec<&'static str> {
    cell.children
        .iter()
        .filter_map(|id| graph.block(*id))
        .map(|node| match &node.block {
            Block::Paragraph { .. } => "paragraph",
            Block::List { .. } => "list",
            Block::Table { .. } => "table",
            _ => "other",
        })
        .collect()
}

#[test]
fn epub_import_preserves_table_cell_blocks_and_round_trips() {
    let zip = epub_with_block_cell_zip();
    let graph = import_epub_bytes("cell-blocks.epub", &zip).expect("import epub");
    let cell = first_body_cell(&graph);
    assert_eq!(cell_block_kinds(&graph, cell), ["paragraph", "list"], "cell blocks stay separate: {cell:?}");

    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("<p>one</p>"), "multi-block cells fall back to an inline HTML table: {markdown}");
    assert!(markdown.contains("<li>item</li>"), "{markdown}");

    let bytes = export_epub_bytes(&graph).expect("export epub");
    let reopened = import_epub_bytes("cell-blocks-roundtrip.epub", &bytes).expect("reopen epub");
    let reopened_cell = first_body_cell(&reopened);
    assert_eq!(
        cell_block_kinds(&reopened, reopened_cell),
        ["paragraph", "list"],
        "an EPUB round trip keeps the block shape of the cell: {reopened_cell:?}"
    );
}

#[test]
fn epub_import_lowers_container_blockquote_code_and_hr() {
    let zip = epub_with_block_elements_zip();
    let graph = import_epub_bytes("blocks.epub", &zip).expect("import epub");
    let children = graph
        .blocks
        .iter()
        .find_map(|node| match &node.block {
            Block::Quote { children } => Some(children.as_slice()),
            _ => None,
        })
        .expect("quote expected");
    assert!(!children.is_empty(), "quote body must be materialized as children");
    assert!(
        children.iter().all(|child_id| matches!(graph.block(*child_id).map(|node| &node.block), Some(Block::Paragraph { .. }))),
        "blockquote body must lower to paragraph children: {children:#?}"
    );
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("Inside section"));
    assert!(markdown.contains("> Quoted line"));
    assert!(markdown.contains("```rust"));
    assert!(markdown.contains("fn main() {}"));
    assert!(markdown.contains("---"));

    let bytes = export_epub_bytes(&graph).expect("export epub");
    let reopened = import_epub_bytes("blocks-roundtrip.epub", &bytes).expect("reopen epub");
    let markdown = export_markdown(&reopened).expect("export reopened markdown");
    assert!(markdown.contains("> Quoted line"), "{markdown}");
}

#[test]
fn epub_import_preserves_inline_identity_and_classes() {
    let container = br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;
    let opf = br#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Attributed Book</dc:title>
    <dc:language>en</dc:language>
  </metadata>
  <manifest>
    <item id="ch1" href="chapter.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="ch1"/>
  </spine>
</package>"#;
    let chapter = br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <body><p><span id="anchor" class="lead note">Marked</span></p></body>
</html>"#;
    let zip = stored_zip(&[
        ("mimetype", b"application/epub+zip"),
        ("META-INF/container.xml", container),
        ("OEBPS/content.opf", opf),
        ("OEBPS/chapter.xhtml", chapter),
    ]);
    let graph = import_epub_bytes("attributed.epub", &zip).expect("import epub");
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("paragraph expected")
    };
    let Inline::Attributed { attributes, children } = &content[0]
    else {
        panic!("attributed inline expected, got {content:?}")
    };
    assert_eq!(attributes.id.as_deref(), Some("anchor"));
    assert_eq!(attributes.classes, ["lead", "note"]);
    assert_eq!(children, &[Inline::Text { text: "Marked".into() }]);
}

#[test]
fn epub_import_maps_inline_styles_to_canonical_ir_names() {
    let container = br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;
    let opf = br#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Styles Book</dc:title>
    <dc:language>en</dc:language>
  </metadata>
  <manifest>
    <item id="ch1" href="chapter.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="ch1"/>
  </spine>
</package>"#;
    let chapter = br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <body><p><s>gone</s> <u>under</u> <sup>up</sup> <sub>down</sub></p></body>
</html>"#;
    let zip = stored_zip(&[
        ("mimetype", b"application/epub+zip"),
        ("META-INF/container.xml", container),
        ("OEBPS/content.opf", opf),
        ("OEBPS/chapter.xhtml", chapter),
    ]);
    let graph = import_epub_bytes("styles.epub", &zip).expect("import epub");
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("paragraph expected")
    };
    let styles = content
        .iter()
        .filter_map(|inline| match inline {
            Inline::Styled { style, children } => Some((style.as_str(), children.iter().map(Inline::plain_text).collect::<String>())),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        styles,
        vec![
            ("strike", "gone".to_string()),
            ("underline", "under".to_string()),
            ("superscript", "up".to_string()),
            ("subscript", "down".to_string()),
        ]
    );
}

#[test]
fn epub_import_lowers_multiline_blockquote() {
    let container = br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;
    let opf = br#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Quote Book</dc:title>
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
  <body>
    <blockquote>
      <p>First line</p>
      <p>Second line</p>
    </blockquote>
  </body>
</html>"#,
        ),
    ]);
    let graph = import_epub_bytes("quote.epub", &zip).expect("import epub");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("First line"));
    assert!(markdown.contains("Second line"));
    assert!(markdown.contains('>'));
}

fn epub_with_figure_zip() -> Vec<u8> {
    let container = br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;
    let opf = br#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Figure Book</dc:title>
    <dc:language>en</dc:language>
  </metadata>
  <manifest>
    <item id="photo" href="images/photo.png" media-type="image/png"/>
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
        ("OEBPS/images/photo.png", b"\x89PNG\r\n"),
        (
            "OEBPS/chapter.xhtml",
            br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml">
  <body>
    <figure>
      <img src="images/photo.png" alt="Sunset"/>
      <figcaption>Sunset over the bay</figcaption>
    </figure>
  </body>
</html>"#,
        ),
    ])
}

#[test]
fn epub_import_lowers_figure_image_and_figcaption() {
    let zip = epub_with_figure_zip();
    let graph = import_epub_bytes("figure.epub", &zip).expect("import epub");
    assert!(graph.assets.iter().any(|asset| { asset.source.as_deref() == Some("OEBPS/images/photo.png") }));
    assert!(graph.blocks.iter().any(|node| matches!(
        &node.block,
        Block::Paragraph { content }
            if content.iter().any(|inline| matches!(
                inline,
                Inline::Styled { style, .. } if style == "figcaption"
            ))
    )));

    let error = export_markdown(&graph).expect_err("figcaption must not be flattened to emphasis");
    assert!(error.to_string().contains("figcaption"));
}

#[test]
fn epub_import_lowers_tables_and_nested_lists() {
    let zip = epub_with_table_and_nested_list_zip();
    let graph = import_epub_bytes("structure.epub", &zip).expect("import epub");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("| Name | Value |"));
    assert!(markdown.contains("| Alpha | 1 |"));
    assert!(markdown.contains("- Outer"));
    assert!(markdown.contains("  - Inner"));
}
#[test]
fn epub_spine_idref_missing_from_manifest_is_reported_and_later_pages_continue() {
    let opf = br#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Broken Spine</dc:title>
    <dc:language>en</dc:language>
  </metadata>
  <manifest>
    <item id="ch2" href="chapter2.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="missing"/>
    <itemref idref="ch2"/>
  </spine>
</package>"#;
    let zip = stored_zip(&[
        ("mimetype", b"application/epub+zip"),
        ("META-INF/container.xml", container_xml()),
        ("OEBPS/content.opf", opf),
        ("OEBPS/chapter2.xhtml", br#"<html><body><p>Second chapter still imports</p></body></html>"#),
    ]);
    let graph = import_epub_bytes("broken-spine.epub", &zip).expect("spine anomaly must not abort the book");
    assert!(
        graph
            .coverage
            .loss
            .iter()
            .any(|loss| loss.code == "import.epub.spine_item_unresolved" && loss.status == document_ir::SemanticStatus::Unresolved),
        "{:?}",
        graph.coverage.loss
    );
    assert!(export_markdown(&graph).expect("export markdown").contains("Second chapter still imports"));
}

#[test]
fn epub_missing_spine_member_is_reported_and_later_pages_continue() {
    let opf = br#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Missing Member</dc:title>
    <dc:language>en</dc:language>
  </metadata>
  <manifest>
    <item id="ch1" href="missing.xhtml" media-type="application/xhtml+xml"/>
    <item id="ch2" href="chapter2.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine>
    <itemref idref="ch1"/>
    <itemref idref="ch2"/>
  </spine>
</package>"#;
    let zip = stored_zip(&[
        ("mimetype", b"application/epub+zip"),
        ("META-INF/container.xml", container_xml()),
        ("OEBPS/content.opf", opf),
        ("OEBPS/chapter2.xhtml", br#"<html><body><p>Readable chapter follows</p></body></html>"#),
    ]);
    let graph = import_epub_bytes("missing-member.epub", &zip).expect("spine anomaly must not abort the book");
    assert!(
        graph
            .coverage
            .loss
            .iter()
            .any(|loss| loss.code == "import.epub.spine_member_missing" && loss.status == document_ir::SemanticStatus::Unresolved),
        "{:?}",
        graph.coverage.loss
    );
    assert!(export_markdown(&graph).expect("export markdown").contains("Readable chapter follows"));
}

#[test]
fn epub_empty_spine_is_reported_instead_of_silently_succeeding() {
    let opf = br#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>No Spine</dc:title>
    <dc:language>en</dc:language>
  </metadata>
  <manifest>
    <item id="ch1" href="chapter.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine/>
</package>"#;
    let zip = stored_zip(&[
        ("mimetype", b"application/epub+zip"),
        ("META-INF/container.xml", container_xml()),
        ("OEBPS/content.opf", opf),
        ("OEBPS/chapter.xhtml", br#"<html><body><p>Unreachable</p></body></html>"#),
    ]);
    let graph = import_epub_bytes("empty-spine.epub", &zip).expect("import epub");
    assert!(
        graph.coverage.loss.iter().any(|loss| loss.code == "import.epub.spine_empty" && loss.status == document_ir::SemanticStatus::Unresolved),
        "{:?}",
        graph.coverage.loss
    );
}

#[test]
fn epub_assets_carry_content_identity_after_hydration() {
    let zip = epub_with_image_zip();
    let graph = import_epub_bytes("identity.epub", &zip).expect("import epub");
    let image = graph.assets.iter().find(|asset| asset.source.as_deref() == Some("OEBPS/images/cover.png")).expect("cover asset");
    let expected = blake3::hash(b"\x89PNG\r\n").to_hex().to_string();
    assert_eq!(image.content_identity.as_deref(), Some(expected.as_str()));
}

fn container_xml() -> &'static [u8] {
    br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#
}

#[test]
fn epub_inspect_reports_members_without_decoding_or_exposing_archive_types() {
    let zip =
        stored_zip(&[("mimetype", b"application/epub+zip"), ("META-INF/container.xml", container_xml()), ("OEBPS/content.opf", b"<package/>")]);
    let inspect = document_formats::inspect_epub_bytes("inspect.epub", &zip, document_formats::InspectBudget::default()).expect("inspect epub");
    assert_eq!(inspect.format, "epub");
    assert_eq!(inspect.outer, "zip");
    assert_eq!(inspect.inner, "ocf");
    assert!(inspect.layout.is_none());
    let opf = inspect.parts.iter().find(|part| part.path == "OEBPS/content.opf").expect("opf member");
    assert_eq!(opf.uncompressed_size, 10);
    assert_eq!(opf.compressed_size, 10);
    assert_eq!(opf.decoded_size, None);
}

#[test]
fn epub_inspect_enforces_container_member_and_declared_size_budgets() {
    let zip =
        stored_zip(&[("mimetype", b"application/epub+zip"), ("META-INF/container.xml", container_xml()), ("OEBPS/content.opf", b"<package/>")]);
    let members = document_formats::InspectBudget { max_container_members: 2, ..Default::default() };
    let error = document_formats::inspect_epub_bytes("too-many-members.epub", &zip, members).expect_err("member budget");
    assert!(matches!(error, document_formats::FormatError::Budget { ref format, .. } if format == "epub"), "{error:?}");

    let declared = document_formats::InspectBudget { max_declared_uncompressed_bytes: 8, ..Default::default() };
    let error = document_formats::inspect_epub_bytes("too-large-declared.epub", &zip, declared).expect_err("declared budget");
    assert!(matches!(error, document_formats::FormatError::Budget { ref format, .. } if format == "epub"), "{error:?}");
}

mod import_ast_lowering;
mod import_navigation;
mod import_xhtml;
mod query_gate;

#[test]
fn epub_import_records_package_member_provenance() {
    let graph = import_epub_bytes("book.epub", &minimal_epub_zip()).expect("import epub");
    assert!(!graph.blocks.is_empty());

    for node in &graph.blocks {
        let sources = graph.sources_for(node.id);
        assert_eq!(sources.len(), 1, "each spine block records exactly one package member source");
        assert!(
            matches!(&sources[0].kind, SourceKind::PackageMember { path } if path == "OEBPS/chapter.xhtml"),
            "unexpected source: {:?}",
            sources[0].kind
        );
        assert_eq!(sources[0].precision, SemanticStatus::Partial);
    }
    assert!(graph.validate().is_valid());
}

#[cfg(feature = "docx")]
fn paragraph_with_text<'a>(graph: &'a document_ir::DocumentGraph, text: &str) -> &'a document_ir::BlockNode {
    graph
        .blocks
        .iter()
        .find(|node| {
            matches!(&node.block, Block::Paragraph { content }
                if matches!(content.as_slice(), [Inline::Text { text: authored }] if authored == text))
        })
        .expect("shared paragraph")
}

#[cfg(feature = "docx")]
#[test]
fn docx_and_epub_cross_import_share_content_but_not_provenance() {
    use document_formats::import::docx::import_docx_bytes;

    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:r><w:t>Hello EPUB</w:t></w:r></w:p></w:body>
</w:document>"#;
    let from_docx = import_docx_bytes("cross.docx", &stored_zip(&[("word/document.xml", document_xml)])).expect("import docx");
    let from_epub = import_epub_bytes("cross.epub", &minimal_epub_zip()).expect("import epub");

    let docx_paragraph = paragraph_with_text(&from_docx, "Hello EPUB");
    let epub_paragraph = paragraph_with_text(&from_epub, "Hello EPUB");
    assert_eq!(docx_paragraph.block, epub_paragraph.block);

    let docx_sources = from_docx.sources_for(docx_paragraph.id);
    let epub_sources = from_epub.sources_for(epub_paragraph.id);
    assert_eq!(docx_sources.len(), 1);
    assert_eq!(epub_sources.len(), 1);
    assert!(matches!(&docx_sources[0].kind, SourceKind::XmlRange { path } if path == "word/document.xml"));
    assert!(matches!(&epub_sources[0].kind, SourceKind::PackageMember { path } if path == "OEBPS/chapter.xhtml"));
    assert_ne!(docx_sources[0].kind, epub_sources[0].kind);
}

#[test]
fn epub_export_writes_task_checkboxes_and_leaves_plain_items_alone() {
    let mut graph = DocumentGraph::new(DocumentId(60));
    graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "Tasks".into() }] });
    graph.push_block(Block::List {
        ordered: false,
        items: vec![
            ListItem {
                content: vec![Inline::Text { text: "Done".into() }],
                children: Vec::new(),
                number: None,
                is_task: true,
                is_checked: Some(true),
            },
            ListItem {
                content: vec![Inline::Text { text: "Todo".into() }],
                children: Vec::new(),
                number: None,
                is_task: true,
                is_checked: Some(false),
            },
            ListItem {
                content: vec![Inline::Text { text: "Plain".into() }],
                children: Vec::new(),
                number: None,
                is_task: false,
                is_checked: None,
            },
        ],
    });

    let bytes = export_epub_bytes(&graph).expect("export task EPUB");
    let package = String::from_utf8_lossy(&bytes);
    assert!(
        package.contains(r#"<li class="task-list-item"><input type="checkbox" disabled="disabled" checked="checked" />"#),
        "a checked item needs a checked XHTML checkbox: {package}"
    );
    assert!(
        package.contains(r#"<li class="task-list-item"><input type="checkbox" disabled="disabled" />"#),
        "an open item needs an unchecked XHTML checkbox: {package}"
    );
    assert_eq!(package.matches(r#"class="task-list-item""#).count(), 2, "the plain item must stay marker-free: {package}");
}

#[test]
fn epub_export_reports_only_declared_losses() {
    use document_formats::import::markdown::import_markdown_bytes;

    let graph = import_markdown_bytes("epub-loss-channel", "# Title\n\nBody.\n").expect("markdown import");
    let export = export_epub_report(&graph).expect("epub export");
    assert!(!export.bytes.is_empty());
    let codes: Vec<&str> = export.losses.iter().map(|loss| loss.code.as_str()).collect();
    assert!(codes.is_empty(), "a plain document declares no EPUB loss, but the channel reported {codes:?}");
}

#[test]
fn markdown_image_without_bytes_is_reported_and_absent_from_the_package() {
    use document_formats::import::markdown::import_markdown_bytes;

    let graph = import_markdown_bytes("image-loss.md", "A ![alt](pic.png)\n").expect("markdown import");
    let export = export_epub_report(&graph).expect("epub export");
    let codes: Vec<&str> = export.losses.iter().map(|loss| loss.code.as_str()).collect();
    assert!(codes.contains(&"export.epub.image_asset_unresolved"), "{codes:?}");
    assert!(export.losses.iter().all(|loss| loss.code != "export.epub.image_asset_unresolved" || loss.status == SemanticStatus::Unresolved));

    let package = String::from_utf8_lossy(&export.bytes);
    assert!(package.contains(r#"src="pic.png""#), "the chapter keeps the unresolved source: {package}");
    assert!(!package.contains("OEBPS/assets/"), "the package must not pretend the missing bytes exist: {package}");
}

#[test]
fn packaged_image_bytes_are_exported_without_an_image_loss() {
    let mut graph = DocumentGraph::new(DocumentId(11));
    let asset = graph.push_asset(Asset {
        id: AssetId(1),
        kind: AssetKind::Image,
        content_identity: None,
        source: Some("images/pic.png".into()),
        media_type: Some("image/png".into()),
        status: SemanticStatus::Resolved,
        bytes: Some(b"\x89PNG\r\n".to_vec()),
    });
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Image {
            source: "images/pic.png".into(),
            alt: vec![Inline::Text { text: "alt".into() }],
            title: None,
            asset: Some(asset),
        }],
    });

    let export = export_epub_report(&graph).expect("epub export");
    let codes: Vec<&str> = export.losses.iter().map(|loss| loss.code.as_str()).collect();
    assert!(codes.is_empty(), "a packaged image must not declare an image loss: {codes:?}");

    let package = String::from_utf8_lossy(&export.bytes);
    assert!(package.contains("OEBPS/assets/pic.png"), "the resolved bytes must be a package member: {package}");
    assert!(package.contains(r#"src="assets/pic.png""#), "the chapter must point at the packaged asset: {package}");
}

#[test]
fn epub_export_reports_dropped_document_metadata() {
    let mut graph = DocumentGraph::new(DocumentId(7));
    graph.metadata = DocumentMetadata {
        title: Some("Title".into()),
        language: Some("en".into()),
        authors: vec!["First".into(), "Second".into()],
        tags: vec!["rust".into()],
        properties: vec![MetadataEntry { key: "key".into(), value: MetadataValue::Text("value".into()) }],
    };
    graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "Body".into() }] });

    let export = export_epub_report(&graph).expect("epub export");
    let codes: Vec<&str> = export.losses.iter().map(|loss| loss.code.as_str()).collect();
    assert_eq!(codes, ["export.epub.creator_dropped", "export.epub.tag_dropped", "export.epub.property_dropped"]);
    assert!(export.losses.iter().all(|loss| loss.status == SemanticStatus::Partial));

    let roundtrip = import_epub_bytes("roundtrip.epub", &export.bytes).expect("epub import");
    assert_eq!(roundtrip.metadata.authors, ["First"], "the second creator must really be gone");
    assert!(roundtrip.metadata.tags.is_empty(), "tags must really be gone");
    assert!(roundtrip.metadata.properties.is_empty(), "properties must really be gone");
}

#[test]
fn epub_import_with_several_creators_reports_the_export_drop() {
    let container = br#"<?xml version="1.0" encoding="UTF-8"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#;
    let opf = br#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="uid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Two Creators</dc:title>
    <dc:language>en</dc:language>
    <dc:creator>First</dc:creator>
    <dc:creator>Second</dc:creator>
    <dc:subject>rust</dc:subject>
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
  <body><p>Body.</p></body>
</html>"#,
        ),
    ]);

    let imported = import_epub_bytes("two-creators.epub", &zip).expect("import epub");
    assert_eq!(imported.metadata.authors, ["First", "Second"], "the import keeps every creator");
    assert!(
        imported.coverage.loss.iter().any(|loss| loss.code == "import.epub.metadata_unmapped"),
        "the import already declares the metadata it did not land"
    );

    let export = export_epub_report(&imported).expect("export epub");
    let codes: Vec<&str> = export.losses.iter().map(|loss| loss.code.as_str()).collect();
    assert_eq!(codes, ["export.epub.creator_dropped"], "the export names the creator it drops: {codes:?}");
}

#[test]
fn epub_export_reports_flattened_table_foot_band() {
    let mut graph = DocumentGraph::new(DocumentId(8));
    let header = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "H".into() }] });
    let body = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "B".into() }] });
    let footer = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "F".into() }] });
    graph.push_block(Block::Table {
        caption: None,
        head: vec![TableRow { cells: vec![TableCell { children: vec![header], colspan: 1, rowspan: 1 }] }],
        body: vec![TableRow { cells: vec![TableCell { children: vec![body], colspan: 1, rowspan: 1 }] }],
        foot: vec![TableRow { cells: vec![TableCell { children: vec![footer], colspan: 1, rowspan: 1 }] }],
    });

    let export = export_epub_report(&graph).expect("epub export");
    let loss = export.losses.iter().find(|loss| loss.code == "export.epub.table_foot_band_flattened").expect("foot band loss");
    assert!(loss.message.contains("1 foot row(s)"), "{}", loss.message);

    let roundtrip = import_epub_bytes("roundtrip.epub", &export.bytes).expect("epub import");
    let (head_rows, foot_rows) = roundtrip
        .blocks
        .iter()
        .find_map(|node| match &node.block {
            Block::Table { head, foot, .. } => Some((head.len(), foot.len())),
            _ => None,
        })
        .expect("the table survives the round trip");
    assert_eq!(head_rows, 1, "the head row survives through `<th>`");
    assert_eq!(foot_rows, 0, "the foot band is flattened into the body, not restored");

    let mut plain = DocumentGraph::new(DocumentId(9));
    let cell = plain.push_block(Block::Paragraph { content: vec![Inline::Text { text: "B".into() }] });
    plain.push_block(Block::Table {
        caption: None,
        head: Vec::new(),
        body: vec![TableRow { cells: vec![TableCell { children: vec![cell], colspan: 1, rowspan: 1 }] }],
        foot: Vec::new(),
    });
    let export = export_epub_report(&plain).expect("epub export");
    assert!(
        !export.losses.iter().any(|loss| loss.code == "export.epub.table_foot_band_flattened"),
        "a table without a foot band must not be reported"
    );

    let mut head_only = DocumentGraph::new(DocumentId(10));
    let header = head_only.push_block(Block::Paragraph { content: vec![Inline::Text { text: "H".into() }] });
    head_only.push_block(Block::Table {
        caption: None,
        head: vec![TableRow { cells: vec![TableCell { children: vec![header], colspan: 1, rowspan: 1 }] }],
        body: Vec::new(),
        foot: Vec::new(),
    });
    let export = export_epub_report(&head_only).expect("epub export");
    assert!(
        !export.losses.iter().any(|loss| loss.code == "export.epub.table_foot_band_flattened"),
        "the head band round-trips through `<th>`, so declaring it would over-report"
    );
    let roundtrip = import_epub_bytes("head-only.epub", &export.bytes).expect("epub import");
    let head_rows = roundtrip
        .blocks
        .iter()
        .find_map(|node| match &node.block {
            Block::Table { head, .. } => Some(head.len()),
            _ => None,
        })
        .expect("the table survives the round trip");
    assert_eq!(head_rows, 1, "the head band must survive without a declared loss");
}
