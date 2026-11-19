use document_formats::{
    export::{docx::export_docx_bytes, markdown_project::export_markdown_project},
    import::{docx::import_docx_bytes, markdown::import_markdown_bytes},
};
use document_ir::{Block, Inline};

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

#[test]
fn docx_exports_markdown_with_front_matter_rules_and_math() {
    let markdown = "---\ntitle: fidelity\n---\n\n# Heading\n\nBody with $x^2$ math.\n\n---\n\n$$\n\\frac{1}{2}\n$$\n";
    let graph = import_markdown_bytes("fidelity.md", markdown).expect("import markdown");
    let exported = export_docx_bytes(&graph).expect("markdown with front matter, a rule and math exports");
    let xml = String::from_utf8_lossy(&exported);
    assert!(xml.contains("<w:pStyle w:val=\"Heading1\"/>"), "the heading survives: {xml}");
    assert!(xml.contains("title: fidelity"), "front matter stays visible as text");
    assert!(xml.contains("<m:oMath>"), "inline math becomes OMML");
    assert!(xml.contains("<m:oMathPara><m:oMath><m:f>"), "block math becomes an OMML fraction");
    let reopened = import_docx_bytes("fidelity.docx", &exported).expect("reopen exported docx");
    assert!(reopened.blocks.iter().any(|node| matches!(node.block, Block::Math { .. })), "{reopened:#?}");
}

#[test]
fn docx_exports_markdown_project_with_materialized_image() {
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
    let zip =
        stored_zip(&[("word/document.xml", document_xml), ("word/_rels/document.xml.rels", rels_xml), ("word/media/logo.png", b"\x89PNG\r\n")]);

    let graph = import_docx_bytes("image.docx", &zip).expect("import docx");
    let project = export_markdown_project(&graph).expect("export markdown project");

    assert!(project.index_markdown.contains("![Logo](assets/logo.png)"));
    assert_eq!(project.assets.len(), 1);
    assert_eq!(project.assets[0].relative_path, "assets/logo.png");
    assert_eq!(project.assets[0].bytes, b"\x89PNG\r\n");
    assert!(project.unresolved_asset_sources.is_empty());

    let reopened = import_markdown_bytes("index.md", &project.index_markdown).expect("oak markdown reopen");
    assert!(reopened.validate().is_valid());
    assert!(reopened.blocks.iter().any(|node| matches!(
        &node.block,
        Block::Paragraph { content }
            if content.iter().any(|inline| matches!(
                inline,
                Inline::Image { source, alt, .. }
                    if source == "assets/logo.png"
                        && alt == &vec![Inline::Text { text: "Logo".into() }]
            ))
    )));
}
