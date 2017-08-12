use panduck_docx::read_docx_bytes;
use panduck_convert::write_document_markdown;

fn stored_zip(path: &str, payload: &[u8]) -> Vec<u8> {
    let name = path.as_bytes();
    let crc = crc32(payload);

    let mut archive = Vec::new();
    archive.extend_from_slice(b"PK\x03\x04");
    archive.extend_from_slice(&[0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    archive.extend_from_slice(&crc.to_le_bytes());
    archive.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    archive.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    archive.extend_from_slice(&(name.len() as u16).to_le_bytes());
    archive.extend_from_slice(&[0x00, 0x00]);
    archive.extend_from_slice(name);
    archive.extend_from_slice(payload);

    let cd_offset = archive.len();
    archive.extend_from_slice(b"PK\x01\x02");
    let mut cd_fixed = [0u8; 46];
    cd_fixed[0..2].copy_from_slice(&[0x14, 0x00]);
    cd_fixed[2..4].copy_from_slice(&[0x14, 0x00]);
    cd_fixed[12..16].copy_from_slice(&crc.to_le_bytes());
    cd_fixed[16..20].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    cd_fixed[20..24].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    cd_fixed[24..26].copy_from_slice(&(name.len() as u16).to_le_bytes());
    cd_fixed[38..42].copy_from_slice(&0u32.to_le_bytes());
    archive.extend_from_slice(&cd_fixed);
    archive.extend_from_slice(name);

    let cd_size = archive.len() - cd_offset;
    archive.extend_from_slice(b"PK\x05\x06");
    archive.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00]);
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
    <w:p>
      <w:pPr><w:pStyle w:val="Heading1"/></w:pPr>
      <w:r><w:t>Title</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
    stored_zip("word/document.xml", document_xml)
}

fn docx_zip(document_xml: &[u8]) -> Vec<u8> {
    stored_zip("word/document.xml", document_xml)
}

#[test]
fn docx_to_markdown_round_trip() {
    let graph = read_docx_bytes("sample.docx", minimal_docx_zip()).expect("read docx");
    let markdown = write_document_markdown(&graph).expect("write markdown");
    assert!(markdown.contains("Hello DOCX"));
    assert!(markdown.contains("# Title"));
}

#[test]
fn docx_imports_run_bold_and_italic() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:rPr><w:b/></w:rPr><w:t>Bold</w:t></w:r>
      <w:r><w:rPr><w:i/></w:rPr><w:t> italic</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
    let graph = read_docx_bytes("styled.docx", docx_zip(document_xml)).expect("read docx");
    let markdown = write_document_markdown(&graph).expect("write markdown");
    assert!(markdown.contains("**Bold**"));
    assert!(markdown.contains("* italic*"));
}
