use document_formats::import::doc::{import_doc_bytes, split_doc_paragraphs};
use document_ir::{Block, Inline};

const SECTOR: usize = 512;
const END_OF_CHAIN: u32 = 0xFFFF_FFFE;
const FREE_SECTOR: u32 = 0xFFFF_FFFF;
const FAT_SECTOR: u32 = 0xFFFF_FFFD;
const NO_STREAM: u32 = 0xFFFF_FFFF;

#[test]
fn preserves_leading_and_trailing_spaces_inside_doc_paragraphs() {
    let paragraphs = split_doc_paragraphs("  leading\ttext  \nnext  ").collect::<Vec<_>>();
    assert_eq!(paragraphs, ["  leading\ttext  ", "next  "]);
}

#[test]
fn imports_complex_piece_table_with_compressed_and_unicode_pieces() {
    let bytes = build_compound_file(&complex_word_document(), Some(&complex_table_stream()));
    let graph = import_doc_bytes("complex.doc", &bytes).expect("import doc");

    assert_eq!(graph.blocks.len(), 2, "{:?}", graph.blocks);
    assert!(
        matches!(
            &graph.blocks[0].block,
            Block::Paragraph { content } if content == &[Inline::Text { text: "A".into() }]
        ),
        "{:?}",
        graph.blocks
    );
    assert!(
        matches!(
            &graph.blocks[1].block,
            Block::Paragraph { content } if content == &[Inline::Text { text: "你好".into() }]
        ),
        "{:?}",
        graph.blocks
    );

    let codes = graph.coverage.loss.iter().map(|loss| loss.code.as_str()).collect::<Vec<_>>();
    assert!(codes.contains(&"import.doc.partial_coverage"), "{codes:?}");
    assert!(!codes.contains(&"import.doc.complex_piece_table_unresolved"), "{codes:?}");
}

#[test]
fn reports_complex_text_that_falls_back_to_the_simple_range() {
    let bytes = build_compound_file(&complex_word_document(), None);
    let graph = import_doc_bytes("complex.doc", &bytes).expect("import doc");

    // Without the table stream the CLX cannot be read, so the adapter keeps the
    // FIB text range and records the unresolved structure instead of failing.
    assert!(
        matches!(
            &graph.blocks[0].block,
            Block::Paragraph { content } if content == &[Inline::Text { text: "X".into() }]
        ),
        "{:?}",
        graph.blocks
    );
    let codes = graph.coverage.loss.iter().map(|loss| loss.code.as_str()).collect::<Vec<_>>();
    assert!(codes.contains(&"import.doc.complex_piece_table_unresolved"), "{codes:?}");
}

#[test]
fn rejects_encrypted_legacy_documents_as_unsupported() {
    let mut word_document = plain_word_document();
    write_u16(&mut word_document, 0x0A, 0x0100);
    let bytes = build_compound_file(&word_document, None);
    let error = import_doc_bytes("encrypted.doc", &bytes).expect_err("encrypted doc must fail");
    assert!(
        matches!(
            error,
            document_formats::FormatError::Unsupported { ref format, .. } if format == "doc"
        ),
        "{error:?}"
    );
}

#[test]
fn rejects_non_compound_bytes_as_invalid_input() {
    let error = import_doc_bytes("not-a-doc.doc", b"plain text").expect_err("non compound bytes must fail");
    assert!(matches!(error, document_formats::FormatError::InvalidInput { .. }), "{error:?}");
}

/// A non-complex FIB that keeps the simple text range valid.
fn plain_word_document() -> Vec<u8> {
    let mut bytes = vec![0u8; 4096];
    bytes[0] = 0xEC;
    bytes[1] = 0xA5;
    write_u32(&mut bytes, 0x18, 500);
    write_u32(&mut bytes, 0x1C, 501);
    bytes[500] = b'X';
    bytes
}

/// A complex FIB whose CLX lives at offset 0 of the table stream.
fn complex_word_document() -> Vec<u8> {
    let mut bytes = plain_word_document();
    write_u16(&mut bytes, 0x0A, 0x0204);
    write_u16(&mut bytes, 0x20, 0);
    write_u16(&mut bytes, 0x22, 0);
    write_u16(&mut bytes, 0x24, 34);
    let field = 0x26 + 33 * 8;
    write_u32(&mut bytes, field, 0);
    write_u32(&mut bytes, field + 4, 33);
    // Piece 0 is compressed at byte 400 and piece 1 is UTF-16 at byte 402.
    bytes[400] = b'A';
    bytes[401] = 0x0D;
    bytes[402..406].copy_from_slice(&[0x60, 0x4F, 0x7D, 0x59]);
    bytes
}

fn complex_table_stream() -> Vec<u8> {
    let mut stream = vec![0u8; 4096];
    let mut clx = Vec::new();
    clx.push(0x02);
    clx.extend_from_slice(&28u32.to_le_bytes());
    for cp in [0u32, 2, 4] {
        clx.extend_from_slice(&cp.to_le_bytes());
    }
    clx.extend_from_slice(&0u16.to_le_bytes());
    clx.extend_from_slice(&(0x4000_0000u32 | 800).to_le_bytes());
    clx.extend_from_slice(&0u16.to_le_bytes());
    clx.extend_from_slice(&0u16.to_le_bytes());
    clx.extend_from_slice(&402u32.to_le_bytes());
    clx.extend_from_slice(&0u16.to_le_bytes());
    stream[..clx.len()].copy_from_slice(&clx);
    stream
}

/// Builds a minimal version 3 compound file with regular (non-mini) streams.
fn build_compound_file(word_document: &[u8], table_stream: Option<&[u8]>) -> Vec<u8> {
    assert!(word_document.len() == 4096, "regular sector stream");
    let table_sectors = table_stream.map(|stream| {
        assert!(stream.len() == 4096, "regular sector stream");
        8usize
    });
    let word_sectors = 8usize;
    let directory_sector = 1u32;
    let word_start = 2u32;
    let table_start = word_start + word_sectors as u32;
    let total_sectors = 2 + word_sectors + table_sectors.unwrap_or(0);

    let mut file = vec![0u8; SECTOR + total_sectors * SECTOR];
    file[0..8].copy_from_slice(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]);
    write_u16(&mut file, 0x18, 0x003E);
    write_u16(&mut file, 0x1A, 3);
    write_u16(&mut file, 0x1C, 0xFFFE);
    write_u16(&mut file, 0x1E, 9);
    write_u16(&mut file, 0x20, 6);
    write_u32(&mut file, 0x28, 1);
    write_u32(&mut file, 0x2C, 1);
    write_u32(&mut file, 0x30, directory_sector);
    write_u32(&mut file, 0x38, 4096);
    write_u32(&mut file, 0x3C, END_OF_CHAIN);
    write_u32(&mut file, 0x44, END_OF_CHAIN);
    for slot in 0..109 {
        write_u32(&mut file, 0x4C + slot * 4, FREE_SECTOR);
    }
    write_u32(&mut file, 0x4C, 0);

    let mut fat = vec![FREE_SECTOR; 128];
    fat[0] = FAT_SECTOR;
    fat[directory_sector as usize] = END_OF_CHAIN;
    for index in 0..word_sectors {
        let slot = word_start as usize + index;
        fat[slot] = if index + 1 == word_sectors { END_OF_CHAIN } else { (slot + 1) as u32 };
    }
    if let Some(sectors) = table_sectors {
        for index in 0..sectors {
            let slot = table_start as usize + index;
            fat[slot] = if index + 1 == sectors { END_OF_CHAIN } else { (slot + 1) as u32 };
        }
    }
    for (index, value) in fat.iter().enumerate() {
        write_u32(&mut file, (1 + 0) * SECTOR + index * 4, *value);
    }

    let mut entries = Vec::new();
    let mut root = directory_entry("Root Entry", 5, if table_sectors.is_some() { 2 } else { 1 }, END_OF_CHAIN, 0);
    root[0x43] = 1;
    entries.push(root);
    let word_document_entry = directory_entry("WordDocument", 2, NO_STREAM, word_start, 4096);
    entries.push(word_document_entry);
    if let Some(sectors) = table_sectors {
        let mut table = directory_entry("1Table", 2, NO_STREAM, table_start, 4096);
        write_u32(&mut table, 0x48, 1);
        entries.push(table);
    }
    while entries.len() < 4 {
        entries.push(directory_entry("\0", 0, NO_STREAM, END_OF_CHAIN, 0));
    }
    for (index, entry) in entries.iter().enumerate() {
        let offset = (1 + directory_sector as usize) * SECTOR + index * 128;
        file[offset..offset + 128].copy_from_slice(entry);
    }

    let word_offset = (1 + word_start as usize) * SECTOR;
    file[word_offset..word_offset + 4096].copy_from_slice(word_document);
    if let (Some(stream), Some(_)) = (table_stream, table_sectors) {
        let table_offset = (1 + table_start as usize) * SECTOR;
        file[table_offset..table_offset + 4096].copy_from_slice(stream);
    }
    file
}

fn directory_entry(name: &str, kind: u8, child: u32, start: u32, size: u32) -> [u8; 128] {
    let mut entry = [0u8; 128];
    let units = name.encode_utf16().collect::<Vec<_>>();
    for (index, unit) in units.iter().enumerate() {
        write_u16(&mut entry, index * 2, *unit);
    }
    write_u16(&mut entry, 0x40, ((units.len() + 1) * 2) as u16);
    entry[0x42] = kind;
    entry[0x43] = 1;
    write_u32(&mut entry, 0x44, NO_STREAM);
    write_u32(&mut entry, 0x48, NO_STREAM);
    write_u32(&mut entry, 0x4C, child);
    write_u32(&mut entry, 0x74, start);
    write_u32(&mut entry, 0x78, size);
    entry
}

fn write_u16(buffer: &mut [u8], offset: usize, value: u16) {
    buffer[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(buffer: &mut [u8], offset: usize, value: u32) {
    buffer[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}
