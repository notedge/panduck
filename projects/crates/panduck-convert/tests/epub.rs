use panduck_convert::convert_bytes;

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
fn epub_to_markdown_route() {
    let zip = minimal_epub_zip();
    let output = convert_bytes("epub", "markdown", "book.epub", zip).expect("convert epub");
    assert!(output.markdown.contains("# Chapter One"));
    assert!(output.markdown.contains("Hello EPUB"));
    assert!(output.report_json.contains("panduck.report/v1"));
    assert!(output.report_json.contains("epub"));
}
