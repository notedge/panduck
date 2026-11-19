use document_formats::{FormatError, InspectBudget, inspect_docx_bytes, verify_docx_bytes};

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
fn inspect_lists_package_parts_without_decoding() {
    let inspect = inspect_docx_bytes("sample.docx", &minimal_docx_zip(), None, false, InspectBudget::default()).expect("inspect docx");
    assert_eq!(inspect.format, "docx");
    assert_eq!(inspect.outer, "zip");
    assert_eq!(inspect.inner, "opc");
    assert_eq!(inspect.parts.len(), 1);
    assert_eq!(inspect.parts[0].path, "word/document.xml");
    assert!(inspect.parts[0].uncompressed_size > 0);
    assert_eq!(inspect.parts[0].decoded_size, None);
}

#[test]
fn inspect_decodes_package_parts() {
    let inspect = inspect_docx_bytes("sample.docx", &minimal_docx_zip(), None, true, InspectBudget::default()).expect("decode docx");
    assert_eq!(inspect.parts.len(), 1);
    assert!(inspect.parts[0].decoded_size.is_some_and(|size| size > 0));
}

#[test]
fn inspect_decodes_single_part_filter() {
    let inspect =
        inspect_docx_bytes("sample.docx", &minimal_docx_zip(), Some("word/document.xml"), true, InspectBudget::default()).expect("decode docx");
    assert_eq!(inspect.parts.len(), 1);
    assert_eq!(inspect.parts[0].path, "word/document.xml");
}

#[test]
fn inspect_reports_budget_exhaustion() {
    let budget = InspectBudget { max_decoded_bytes: 4, ..InspectBudget::default() };
    let error = inspect_docx_bytes("sample.docx", &minimal_docx_zip(), None, true, budget).expect_err("budget");
    assert!(matches!(error, FormatError::Budget { .. }));
}

#[test]
fn inspect_admits_container_bytes_before_indexing() {
    let bytes = minimal_docx_zip();
    let budget = InspectBudget { max_container_bytes: bytes.len() as u64 - 1, ..InspectBudget::default() };
    let error = inspect_docx_bytes("sample.docx", &bytes, None, false, budget).expect_err("container bytes");
    assert!(matches!(error, FormatError::Budget { .. }), "{error:?}");
}

#[test]
fn inspect_admits_declared_uncompressed_bytes_before_decoding() {
    let budget = InspectBudget { max_declared_uncompressed_bytes: 1, ..InspectBudget::default() };
    let error = inspect_docx_bytes("sample.docx", &minimal_docx_zip(), None, false, budget).expect_err("declared");
    assert!(matches!(error, FormatError::Budget { .. }), "{error:?}");
}

#[test]
fn inspect_indexes_the_container_layout() {
    let bytes = minimal_docx_zip();
    let inspect = inspect_docx_bytes("sample.docx", &bytes, None, false, InspectBudget::default()).expect("inspect");
    let layout = inspect.layout.expect("zip layout");
    assert_eq!(layout.source_len, bytes.len() as u64);
    assert!(layout.central_directory.is_some());
    assert!(layout.spans.len() >= inspect.parts.len());
    assert!(layout.spans.iter().all(|span| span.start + span.length <= layout.source_len));
}

#[test]
fn verification_reopens_the_written_package() {
    let verification = verify_docx_bytes("sample.docx", &minimal_docx_zip(), InspectBudget::default()).expect("verify package");
    assert_eq!(verification.members, 1);
    assert_eq!(verification.parsed_xml_members, 1);
    assert!(verification.main_part_present);
}

#[test]
fn verification_reports_a_missing_main_part() {
    let verification = verify_docx_bytes("sample.docx", &stored_zip(&[("[Content_Types].xml", b"<Types/>")]), InspectBudget::default())
        .expect("verify package");
    assert!(!verification.main_part_present);
}

#[test]
fn verification_rejects_unparseable_xml_members() {
    let error = verify_docx_bytes("sample.docx", &stored_zip(&[("word/document.xml", b"<not-closed>")]), InspectBudget::default())
        .expect_err("unparseable xml");
    assert!(matches!(error, FormatError::Parse { .. }), "{error:?}");
}

#[test]
fn inspect_rejects_non_zip_input() {
    let error = inspect_docx_bytes("sample.docx", b"not a package", None, false, InspectBudget::default()).expect_err("invalid");
    assert!(matches!(error, FormatError::InvalidInput { .. }));
}
