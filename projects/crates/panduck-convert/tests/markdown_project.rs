use std::fs;

use notedown_formats::import::markdown::import_markdown_bytes;
use notedown_ir::{Block, Inline};
use panduck_convert::convert_to_markdown_project;

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
