use notedown_formats::import::docx::import_docx_bytes;
use panduck_convert::{
    inspect_docx_decode_bytes, inspect_docx_index_bytes, write_document_markdown,
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

fn minimal_docx_zip() -> Vec<u8> {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Hello DOCX</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
    stored_zip(&[("word/document.xml", document_xml)])
}

#[test]
fn docx_import_via_notedown_formats() {
    let graph = import_docx_bytes("sample.docx", &minimal_docx_zip()).expect("read docx");
    let markdown = write_document_markdown(&graph).expect("write markdown");
    assert!(markdown.contains("Hello DOCX"));
}

#[test]
fn docx_inspect_decodes_package_parts() {
    let zip = minimal_docx_zip();
    let decode = inspect_docx_decode_bytes("sample.docx", zip, None).expect("decode docx");
    assert_eq!(decode.format, "docx");
    assert_eq!(decode.parts.len(), 1);
    assert_eq!(decode.parts[0].path, "word/document.xml");
    assert!(decode.parts[0].decoded_size > 0);
}

#[test]
fn docx_inspect_decodes_single_part_filter() {
    let zip = minimal_docx_zip();
    let decode =
        inspect_docx_decode_bytes("sample.docx", zip, Some("word/document.xml")).expect("decode");
    assert_eq!(decode.parts.len(), 1);
    assert_eq!(decode.parts[0].path, "word/document.xml");
}

#[test]
fn docx_inspect_lists_package_parts() {
    let zip = minimal_docx_zip();
    let index = inspect_docx_index_bytes("sample.docx", zip).expect("inspect docx");
    assert_eq!(index.format, "docx");
    assert_eq!(index.outer, "zip");
    assert_eq!(index.inner, "opc");
    assert!(index.parts.iter().any(|part| part == "word/document.xml"));
}
