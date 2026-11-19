use acorn_codec::compression::{deflate::DeflateLevel, zlib::Zlib};
use document_formats::{export::pdf::export_pdf_bytes, import::pdf::import_pdf_bytes};
use document_ir::{Asset, AssetId, AssetKind, Block, DocumentGraph, DocumentId, DocumentMetadata, Inline, ListItem, NodeId, SemanticStatus};

fn jpeg_frame_fixture(width: u16, height: u16, components: u8) -> Vec<u8> {
    let mut bytes = vec![0xff, 0xd8, 0xff, 0xc0];
    bytes.extend_from_slice(&(8 + 3 * u16::from(components)).to_be_bytes());
    bytes.push(8);
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.push(components);
    for component in 0..components {
        bytes.extend_from_slice(&[component + 1, 0x11, 0]);
    }
    bytes.extend_from_slice(&[0xff, 0xd9]);
    bytes
}

fn png_fixture(width: u32, height: u32, color_type: u8, scanlines: &[u8]) -> Vec<u8> {
    let compressed = Zlib::encode(scanlines, DeflateLevel::default());
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let chunk = |kind: &[u8], data: &[u8], output: &mut Vec<u8>| {
        output.extend_from_slice(&(data.len() as u32).to_be_bytes());
        output.extend_from_slice(kind);
        output.extend_from_slice(data);
        let mut crc = 0xffff_ffffu32;
        for byte in kind.iter().chain(data) {
            crc ^= u32::from(*byte);
            for _ in 0..8 {
                crc = if crc & 1 == 1 { (crc >> 1) ^ 0xedb8_8320 } else { crc >> 1 };
            }
        }
        output.extend_from_slice(&(crc ^ 0xffff_ffff).to_be_bytes());
    };
    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, color_type, 0, 0, 0]);
    chunk(b"IHDR", &header, &mut png);
    chunk(b"IDAT", &compressed, &mut png);
    chunk(b"IEND", &[], &mut png);
    png
}

fn flate_image_pdf(predictor: usize, rows: &[&[u8]], encoded_rows: &[u8]) -> Vec<u8> {
    let image = Zlib::encode(encoded_rows, DeflateLevel::default());
    let content = b"q /Im1 Do Q";
    let mut pdf = format!("%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /XObject << /Im1 6 0 R >> >> /Contents 4 0 R >>\nendobj\n4 0 obj\n<< /Length {} >>\nstream\n", content.len()).into_bytes();
    pdf.extend_from_slice(content);
    pdf.extend_from_slice(format!("\nendstream\nendobj\n6 0 obj\n<< /Type /XObject /Subtype /Image /Width 2 /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode /DecodeParms << /Predictor {} /Colors 3 /BitsPerComponent 8 /Columns 2 >> /Length {} >>\nstream\n", rows.len(), predictor, image.len()).as_bytes());
    pdf.extend_from_slice(&image);
    pdf.extend_from_slice(b"\nendstream\nendobj\n%%EOF");
    pdf
}

#[test]
fn generated_pdf_reopens_text() {
    let mut graph = DocumentGraph::new(DocumentId(1));
    graph.push_block(Block::Section { level: 1, title: vec![Inline::Text { text: "Heading".into() }], children: Vec::new() });
    graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "Body (round trip)".into() }] });
    let bytes = export_pdf_bytes(&graph).expect("export PDF");
    let reopened = import_pdf_bytes("generated.pdf", &bytes).expect("import PDF");
    assert!(reopened.blocks.iter().any(|node| matches!(&node.block, Block::Paragraph { content } if format!("{content:?}").contains("Body"))));
}

#[test]
fn reads_literal_text_operator() {
    let bytes = b"%PDF-1.4\n1 0 obj\n<< /Length 18 >>\nstream\nBT (Hello PDF) Tj ET\nendstream\n%%EOF";
    let graph = import_pdf_bytes("sample.pdf", bytes).expect("import PDF");
    assert!(graph.blocks.iter().any(|node| format!("{:?}", node.block).contains("Hello PDF")));
}

#[test]
fn decodes_winansi_font_without_tounicode_map() {
    let content = b"BT /F1 12 Tf (Caf\xE9 \x80 \x97) Tj ET";
    let pdf = format!(
        "%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>\nendobj\n4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>\nendobj\n5 0 obj\n<< /Length {} >>\nstream\n",
        content.len()
    );
    let mut bytes = pdf.into_bytes();
    bytes.extend_from_slice(content);
    bytes.extend_from_slice(b"\nendstream\nendobj\n%%EOF");
    let graph = import_pdf_bytes("winansi-no-cmap.pdf", &bytes).expect("import WinAnsi PDF");
    assert!(format!("{:?}", graph.blocks).contains("Café € —"));
}

#[test]
fn pdf_generation_and_reopen_preserve_text_whitespace_after_editing() {
    let mut graph = DocumentGraph::new(DocumentId(28));
    graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "  first  words\tend  \n  second line  ".into() }] });
    let bytes = export_pdf_bytes(&graph).expect("generate whitespace PDF");
    let mut reopened = import_pdf_bytes("whitespace.pdf", &bytes).expect("reopen whitespace PDF");
    let Block::Paragraph { content } = &mut reopened.blocks[0].block
    else {
        panic!("paragraph expected")
    };
    // PDF carries no authored paragraph breaks, so the authored newline is
    // rebuilt as one geometric paragraph: leading and inner runs survive verbatim,
    // the line join becomes a single space and no trailing spaces survive.
    let text = match &content[0] {
        Inline::Text { text } => text.as_str(),
        other => panic!("text expected, got {other:?}"),
    };
    assert!(text.starts_with("  first  words\tend"), "{text:?}");
    assert!(text.ends_with("second line"), "{text:?}");
    content.push(Inline::Text { text: " edited  ".into() });
    let edited = export_pdf_bytes(&reopened).expect("generate edited PDF");
    let round = import_pdf_bytes("edited-whitespace.pdf", &edited).expect("reopen edited PDF");
    assert_eq!(round.blocks.len(), 1, "{:?}", round.blocks);
    let text = match &round.blocks[0].block {
        Block::Paragraph { content } => match &content[0] {
            Inline::Text { text } => text.as_str(),
            other => panic!("text expected, got {other:?}"),
        },
        other => panic!("paragraph expected, got {other:?}"),
    };
    assert!(text.starts_with("  first  words\tend"), "{text:?}");
    assert!(text.ends_with("edited"), "{text:?}");
}

#[test]
fn pdf_wrapping_preserves_long_tokens_and_spaces() {
    let original = format!("{}  {}   ", "A".repeat(100), "B".repeat(100));
    let mut graph = DocumentGraph::new(DocumentId(29));
    graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: original.clone() }] });
    let bytes = export_pdf_bytes(&graph).expect("generate wrapped PDF");
    let reopened = import_pdf_bytes("wrapped.pdf", &bytes).expect("reopen wrapped PDF");
    let recovered = reopened
        .blocks
        .iter()
        .flat_map(|node| match &node.block {
            Block::Paragraph { content } => content.as_slice(),
            _ => &[],
        })
        .map(|inline| match inline {
            Inline::Text { text } => text.as_str(),
            _ => "",
        })
        .collect::<String>();
    // Line wrapping splits and rejoins the runs, so whitespace is renormalised.
    // What must survive is every non-whitespace character, in order.
    let compact = |value: &str| value.chars().filter(|character| !character.is_whitespace()).collect::<String>();
    assert_eq!(compact(&recovered), compact(&original));
}

#[test]
fn page_index_ignores_non_page_text_streams() {
    let bytes = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /Contents 5 0 R >>\nendobj\n4 0 obj\n<< /Length 17 >>\nstream\nBT (Hidden) Tj ET\nendstream\nendobj\n5 0 obj\n<< /Length 16 >>\nstream\nBT (Visible) Tj ET\nendstream\nendobj\n%%EOF";
    let graph = import_pdf_bytes("pages.pdf", bytes).expect("import PDF");
    let text = format!("{:?}", graph.blocks);
    assert!(text.contains("Visible"));
    assert!(!text.contains("Hidden"));
}

#[test]
fn page_contents_follow_declared_reference_order() {
    let bytes = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /Contents [5 0 R 4 0 R] >>\nendobj\n4 0 obj\n<< /Length 16 >>\nstream\nBT (Second) Tj ET\nendstream\nendobj\n5 0 obj\n<< /Length 16 >>\nstream\nBT (First) Tj ET\nendstream\nendobj\n%%EOF";
    let graph = import_pdf_bytes("ordered-pages.pdf", bytes).expect("import PDF");
    let text = graph.blocks.iter().map(|node| format!("{:?}", node.block)).collect::<String>();
    assert!(text.find("First").expect("First text") < text.find("Second").expect("Second text"));
}

#[test]
fn page_tree_order_wins_over_physical_object_order() {
    let bytes = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [4 0 R 3 0 R] /Count 2 >>\nendobj\n4 0 obj\n<< /Type /Page /Parent 2 0 R /Contents 6 0 R >>\nendobj\n5 0 obj\n<< /Length 16 >>\nstream\nBT (Second) Tj ET\nendstream\nendobj\n6 0 obj\n<< /Length 16 >>\nstream\nBT (First) Tj ET\nendstream\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /Contents 5 0 R >>\nendobj\n%%EOF";
    let graph = import_pdf_bytes("tree-order.pdf", bytes).expect("import PDF");
    let text = graph.blocks.iter().map(|node| format!("{:?}", node.block)).collect::<String>();
    assert!(text.find("First").expect("First text") < text.find("Second").expect("Second text"));
}

#[test]
fn reads_flate_decode_stream_and_tj_array() {
    let compressed = Zlib::encode(b"BT [(Hello) 20 ( PDF)] TJ ET", DeflateLevel::default());
    let mut bytes = format!("%PDF-1.4\n1 0 obj\n<< /Length {} /Filter /FlateDecode >>\nstream\n", compressed.len()).into_bytes();
    bytes.extend_from_slice(&compressed);
    bytes.extend_from_slice(b"\nendstream\nendobj\n%%EOF");
    let graph = import_pdf_bytes("compressed.pdf", &bytes).expect("import PDF");
    assert!(graph.blocks.iter().any(|node| format!("{:?}", node.block).contains("Hello")));
}

#[test]
fn pdf_tj_spacing_is_reported_while_text_remains_editable() {
    let content = b"BT [(Hello) 120 ( PDF)] TJ ET";
    let mut bytes = format!("%PDF-1.4\n1 0 obj\n<< /Length {} >>\nstream\n", content.len()).into_bytes();
    bytes.extend_from_slice(content);
    bytes.extend_from_slice(b"\nendstream\nendobj\n%%EOF");
    let mut graph = import_pdf_bytes("spacing.pdf", &bytes).expect("import spacing PDF");
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "import.pdf.text_spacing"));
    let Block::Paragraph { content } = &mut graph.blocks[0].block
    else {
        panic!("paragraph expected")
    };
    *content = vec![Inline::Text { text: "Edited".into() }];
    let reopened = import_pdf_bytes("edited-spacing.pdf", &export_pdf_bytes(&graph).expect("generate edited PDF")).expect("reopen edited PDF");
    assert!(format!("{:?}", reopened.blocks[0].block).contains("Edited"));
}

#[test]
fn imports_pdf_text_line_operators_as_ordered_line_moves() {
    let bytes = b"%PDF-1.4\n1 0 obj\n<< /Length 42 >>\nstream\nBT (First) Tj T* (Second)' ET\nendstream\nendobj\n%%EOF";
    let graph = import_pdf_bytes("line-operators.pdf", bytes).expect("import PDF line operators");
    let paragraphs = graph
        .blocks
        .iter()
        .filter_map(|node| match &node.block {
            Block::Paragraph { content } => Some(format!("{content:?}")),
            _ => None,
        })
        .collect::<Vec<_>>();
    // `T*` and `'` move to the next visual line, and adjacent lines of one size
    // are rebuilt as one geometric paragraph, so the fragments join in order.
    assert_eq!(paragraphs.len(), 1, "{paragraphs:?}");
    assert!(paragraphs[0].contains("First Second"));
}

#[test]
fn imports_literal_and_hex_quote_operators_through_ir_edit_and_generation() {
    let content = b"BT (First) Tj <5365636F6E64>' 0 0 (Third)\" 0 0 <466F75727468>\" ET";
    let mut bytes = format!("%PDF-1.4\n1 0 obj\n<< /Length {} >>\nstream\n", content.len()).into_bytes();
    bytes.extend_from_slice(content);
    bytes.extend_from_slice(b"\nendstream\nendobj\n%%EOF");
    let mut graph = import_pdf_bytes("quote-operators.pdf", &bytes).expect("import quote operators");
    let text = graph
        .blocks
        .iter()
        .map(|node| match &node.block {
            Block::Paragraph { content } => content
                .iter()
                .map(|inline| match inline {
                    Inline::Text { text } => text.as_str(),
                    _ => panic!("text expected"),
                })
                .collect::<String>(),
            _ => panic!("paragraph expected"),
        })
        .collect::<Vec<_>>();
    assert_eq!(text, ["First Second Third Fourth"]);
    let Block::Paragraph { content } = &mut graph.blocks[0].block
    else {
        panic!("paragraph expected")
    };
    *content = vec![Inline::Text { text: "Edited second".into() }];
    graph.bump_revision();
    let generated = export_pdf_bytes(&graph).expect("generate edited PDF");
    let reopened = import_pdf_bytes("edited-quotes.pdf", &generated).expect("reopen edited PDF");
    assert_eq!(reopened.blocks.len(), 1);
    assert!(format!("{:?}", reopened.blocks[0].block).contains("Edited second"));
}

#[test]
fn pdf_content_comments_and_operator_suffixes_do_not_create_text() {
    let content =
        b"% (Comment) Tj /Im1 Do\nBT (Suffix) TjSuffix (Visible) % (Hidden) Tj\nTj [(Array) % (Ignored) ] TJ\n ( text)] TJ (Invalid) TJ ET";
    let mut bytes = format!("%PDF-1.4\n1 0 obj\n<< /Length {} >>\nstream\n", content.len()).into_bytes();
    bytes.extend_from_slice(content);
    bytes.extend_from_slice(b"\nendstream\nendobj\n%%EOF");
    let graph = import_pdf_bytes("comments.pdf", &bytes).expect("import commented content");
    let text = format!("{:?}", graph.blocks);
    assert!(text.contains("Visible"), "{text}");
    assert!(text.contains("Array text"), "{text}");
    for unexpected in ["Comment", "Suffix", "Hidden", "Ignored", "Invalid"] {
        assert!(!text.contains(unexpected), "{text}");
    }
}

#[test]
fn decodes_type0_text_with_tounicode_cmap() {
    let bytes = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 7 0 R >> >> /Contents 4 0 R >>\nendobj\n4 0 obj\n<< /Length 31 >>\nstream\nBT /F1 12 Tf <0102030405> Tj ET\nendstream\nendobj\n7 0 obj\n<< /Type /Font /Subtype /Type0 /ToUnicode 8 0 R >>\nendobj\n8 0 obj\n<< /Length 220 >>\nstream\n/CIDInit /ProcSet findresource begin\n12 dict begin begincmap\n/CMapName /Test def\n1 begincodespacerange\n<00> <FF>\nendcodespacerange\n2 beginbfchar\n<01> <4F60>\n<02> <597D>\nendbfchar\n1 beginbfrange\n<03> <05> <0041>\nendbfrange\nendcmap CMapName currentdict /CMap defineresource pop end end\nendstream\nendobj\n%%EOF";
    let graph = import_pdf_bytes("tounicode.pdf", bytes).expect("import PDF");
    let text = format!("{:?}", graph.blocks);
    assert!(text.contains("你好ABC"), "decoded text: {text}");
}

#[test]
fn imports_surrogate_pair_range_as_ir_text() {
    let content = "BT /F1 12 Tf <010203> Tj ET";
    let cmap = "1 beginbfrange\n<01> <03> <D83DDE00>\nendbfrange";
    let bytes = format!(
        "%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Resources << /Font << /F1 7 0 R >> >> /Contents 4 0 R >>\nendobj\n4 0 obj\n<< /Length {} >>\nstream\n{content}\nendstream\nendobj\n7 0 obj\n<< /Type /Font /Subtype /Type0 /ToUnicode 8 0 R >>\nendobj\n8 0 obj\n<< /Length {} >>\nstream\n{cmap}\nendstream\nendobj\n%%EOF",
        content.len(),
        cmap.len()
    );
    let graph = import_pdf_bytes("emoji-range.pdf", bytes.as_bytes()).expect("import surrogate pair range");
    assert!(
        graph
            .blocks
            .iter()
            .any(|node| matches!(&node.block, Block::Paragraph { content } if content == &vec![Inline::Text { text: "😀😁😂".into() }]))
    );
}

#[test]
fn imports_jpeg_xobject_as_ir_asset() {
    let bytes = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /XObject << /Im1 7 0 R >> >> /Contents 4 0 R >>\nendobj\n4 0 obj\n<< /Length 9 >>\nstream\n/Im1 Do\nendstream\nendobj\n7 0 obj\n<< /Subtype /Image /Filter /DCTDecode /Width 1 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Length 4 >>\nstream\n\xFF\xD8\xFF\xD9\nendstream\nendobj\n%%EOF";
    let graph = import_pdf_bytes("image.pdf", bytes).expect("import PDF image");
    let image = graph.assets.iter().find(|asset| asset.media_type.as_deref() == Some("image/jpeg")).expect("JPEG asset");
    assert_eq!(image.source.as_deref(), Some("pdf-object-7"));
    assert_eq!(image.bytes.as_deref(), Some(&b"\xFF\xD8\xFF\xD9"[..]));
    assert!(graph.blocks.iter().any(|node| format!("{:?}", node.block).contains("pdf-object-7")));
}

#[test]
fn imports_unfiltered_rgb_xobject_as_ir_asset() {
    // Shape written by document-renderer-pdf: raw RGB samples with no /Filter.
    let bytes = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /XObject << /Im1 7 0 R >> >> /Contents 4 0 R >>\nendobj\n4 0 obj\n<< /Length 9 >>\nstream\n/Im1 Do\nendstream\nendobj\n7 0 obj\n<< /Type /XObject /Subtype /Image /Width 1 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Length 3 >>\nstream\n\xFF\x00\x00\nendstream\nendobj\n%%EOF";
    let graph = import_pdf_bytes("raw-image.pdf", bytes).expect("import raw RGB image");
    let image = graph.assets.iter().find(|asset| asset.media_type.as_deref() == Some("image/png")).expect("raw image asset");
    assert_eq!(image.source.as_deref(), Some("pdf-object-7"));
    // PNG magic proves the raw stream was encoded instead of passed through.
    assert_eq!(image.bytes.as_deref().map(|bytes| &bytes[..4]), Some(&b"\x89PNG"[..]));
    assert!(graph.blocks.iter().any(|node| format!("{:?}", node.block).contains("pdf-object-7")));
}

#[test]
fn imports_jpeg_xobject_from_inherited_page_resources() {
    let bytes = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Resources 8 0 R /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>\nendobj\n4 0 obj\n<< /Length 9 >>\nstream\n/Im1 Do\nendstream\nendobj\n7 0 obj\n<< /Subtype /Image /Filter /DCTDecode /Width 1 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Length 4 >>\nstream\n\xFF\xD8\x02\xD9\nendstream\nendobj\n8 0 obj\n<< /XObject << /Im1 7 0 R >> >>\nendobj\n%%EOF";
    let graph = import_pdf_bytes("inherited-image.pdf", bytes).expect("import inherited PDF image");
    let image = graph.assets.iter().find(|asset| asset.media_type.as_deref() == Some("image/jpeg")).expect("inherited JPEG asset");
    assert_eq!(image.bytes.as_deref(), Some(&b"\xFF\xD8\x02\xD9"[..]));
}

#[test]
fn pdf_import_preserves_supported_text_and_image_event_order() {
    let bytes = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /XObject << /Im1 7 0 R >> >> /Contents 4 0 R >>\nendobj\n4 0 obj\n<< /Length 37 >>\nstream\nBT (Before) Tj ET\n/Im1 Do\nBT (After) Tj ET\nendstream\nendobj\n7 0 obj\n<< /Subtype /Image /Filter /DCTDecode /Width 1 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Length 4 >>\nstream\n\xFF\xD8\xFF\xD9\nendstream\nendobj\n%%EOF";
    let graph = import_pdf_bytes("ordered-image.pdf", bytes).expect("import ordered image page");
    let blocks = graph.blocks.iter().map(|node| format!("{:?}", node.block)).collect::<Vec<_>>();
    assert_eq!(blocks.len(), 3);
    assert!(blocks[0].contains("Before"));
    assert!(blocks[1].contains("pdf-object-7"));
    assert!(blocks[2].contains("After"));
}

#[test]
fn pdf_import_deduplicates_repeated_image_assets_but_keeps_occurrences() {
    let bytes = b"%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /XObject << /Im1 7 0 R >> >> /Contents 4 0 R >>\nendobj\n4 0 obj\n<< /Length 20 >>\nstream\n/Im1 Do /Im1 Do\nendstream\nendobj\n7 0 obj\n<< /Subtype /Image /Filter /DCTDecode /Width 1 /Height 1 /Length 4 >>\nstream\n\xFF\xD8\xFF\xD9\nendstream\nendobj\n%%EOF";
    let graph = import_pdf_bytes("repeated-image.pdf", bytes).expect("import repeated images");
    assert_eq!(graph.assets.iter().filter(|asset| asset.media_type.as_deref() == Some("image/jpeg")).count(), 1);
    assert_eq!(graph.blocks.iter().filter(|node| format!("{:?}", node.block).contains("pdf-object-7")).count(), 2);
}

#[test]
fn imports_inline_jpeg_as_ir_asset() {
    let content = b"BI\n/F /DCTDecode\n/W 1\n/H 1\nID\n\xFF\xD8\x11\xFF\xD9\nEI\n";
    let mut bytes = format!(
        "%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /Contents 4 0 R >>\nendobj\n4 0 obj\n<< /Length {} >>\nstream\n",
        content.len()
    )
    .into_bytes();
    bytes.extend_from_slice(content);
    bytes.extend_from_slice(b"endstream\nendobj\n%%EOF");

    let graph = import_pdf_bytes("inline-image.pdf", &bytes).expect("import inline JPEG");
    let image = graph.assets.iter().find(|asset| asset.media_type.as_deref() == Some("image/jpeg")).expect("inline JPEG asset");
    assert_eq!(image.bytes.as_deref(), Some(&b"\xFF\xD8\x11\xFF\xD9"[..]));
    assert!(graph.blocks.iter().any(|node| format!("{:?}", node.block).contains("pdf-inline-image-")));
}

#[test]
fn jpeg_asset_round_trips_through_pdf_writer() {
    let mut graph = DocumentGraph::new(DocumentId(20));
    graph.push_asset(Asset {
        id: AssetId(1),
        kind: AssetKind::Image,
        content_identity: None,
        source: Some("edited.jpg".into()),
        media_type: Some("image/jpeg".into()),
        status: SemanticStatus::Resolved,
        bytes: Some(jpeg_frame_fixture(640, 480, 3)),
    });
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Image { source: "edited.jpg".into(), alt: Vec::new(), title: None, asset: Some(AssetId(1)) }],
    });
    let exported = export_pdf_bytes(&graph).expect("export JPEG XObject");
    assert!(String::from_utf8_lossy(&exported).contains("/Subtype /Image"));
    assert!(String::from_utf8_lossy(&exported).contains("/Width 640 /Height 480 /ColorSpace /DeviceRGB"));
    assert!(String::from_utf8_lossy(&exported).contains("q 400 0 0 300 106 470 cm"));
    let reopened = import_pdf_bytes("edited-image.pdf", &exported).expect("re-import JPEG XObject");
    let image = reopened.assets.iter().find(|asset| asset.media_type.as_deref() == Some("image/jpeg")).expect("round-tripped JPEG asset");
    assert_eq!(image.bytes, Some(jpeg_frame_fixture(640, 480, 3)));
}

#[test]
fn multiple_jpeg_assets_keep_their_resource_bindings() {
    let mut graph = DocumentGraph::new(DocumentId(21));
    for (id, source, bytes) in [(1, "z.jpg", jpeg_frame_fixture(640, 480, 3)), (2, "a.jpg", jpeg_frame_fixture(200, 600, 1))] {
        graph.push_asset(Asset {
            id: AssetId(id),
            kind: AssetKind::Image,
            content_identity: None,
            source: Some(source.into()),
            media_type: Some("image/jpeg".into()),
            status: SemanticStatus::Resolved,
            bytes: Some(bytes),
        });
        graph.push_block(Block::Paragraph {
            content: vec![Inline::Image { source: source.into(), alt: Vec::new(), title: None, asset: Some(AssetId(id)) }],
        });
    }
    let reopened = import_pdf_bytes("multiple-images.pdf", &export_pdf_bytes(&graph).expect("export images")).expect("re-import images");
    let bytes = reopened.assets.iter().filter_map(|asset| asset.bytes.as_deref()).collect::<Vec<_>>();
    assert!(bytes.contains(&jpeg_frame_fixture(640, 480, 3).as_slice()));
    assert!(bytes.contains(&jpeg_frame_fixture(200, 600, 1).as_slice()));
}

#[test]
fn jpeg_writer_rejects_missing_or_unsupported_frame_metadata() {
    for bytes in [vec![0xff, 0xd8, 0xff, 0xd9], jpeg_frame_fixture(10, 10, 4), jpeg_frame_fixture(0, 10, 3)] {
        let mut graph = DocumentGraph::new(DocumentId(22));
        graph.push_asset(Asset {
            id: AssetId(1),
            kind: AssetKind::Image,
            content_identity: None,
            source: Some("invalid.jpg".into()),
            media_type: Some("image/jpeg".into()),
            status: SemanticStatus::Resolved,
            bytes: Some(bytes),
        });
        graph.push_block(Block::Paragraph {
            content: vec![Inline::Image { source: "invalid.jpg".into(), alt: Vec::new(), title: None, asset: Some(AssetId(1)) }],
        });
        assert!(export_pdf_bytes(&graph).is_err());
    }
}

#[test]
fn pdf_writer_rejects_invalid_graph_containment_before_encoding() {
    let mut graph = DocumentGraph::new(DocumentId(26));
    graph.push_block_with_id(NodeId(1), Block::Section { level: 1, title: Vec::new(), children: vec![NodeId(1)] });
    assert!(export_pdf_bytes(&graph).is_err());

    graph.blocks.clear();
    graph.push_block_with_id(NodeId(1), Block::Section { level: 1, title: Vec::new(), children: vec![NodeId(2)] });
    assert!(export_pdf_bytes(&graph).is_err());
}

#[test]
fn pdf_writer_emits_section_and_list_children_in_ir_order() {
    let mut graph = DocumentGraph::new(DocumentId(27));
    let section_child = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "section child".into() }] });
    let list_child = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "list child".into() }] });
    let list = graph.push_block(Block::List {
        ordered: false,
        items: vec![ListItem {
            content: vec![Inline::Text { text: "list item".into() }],
            children: vec![list_child],
            number: None,
            ..Default::default()
        }],
    });
    graph.push_block(Block::Section {
        level: 1,
        title: vec![Inline::Text { text: "section title".into() }],
        children: vec![section_child, list],
    });
    let bytes = export_pdf_bytes(&graph).expect("export nested PDF graph");
    let pdf = String::from_utf8_lossy(&bytes);
    for text in ["section title", "section child", "list item", "list child"] {
        assert!(pdf.contains(text), "missing nested text {text}: {pdf}");
    }
    assert!(pdf.find("section title").unwrap() < pdf.find("section child").unwrap());
    assert!(pdf.find("section child").unwrap() < pdf.find("list item").unwrap());
    assert!(pdf.find("list item").unwrap() < pdf.find("list child").unwrap());
}

#[test]
fn pdf_writer_converts_8bit_png_to_predictor_flate_image() {
    let png = png_fixture(2, 1, 2, &[0, 0xff, 0, 0, 0, 0, 0xff]);
    let mut graph = DocumentGraph::new(DocumentId(24));
    graph.push_asset(Asset {
        id: AssetId(1),
        kind: AssetKind::Image,
        content_identity: None,
        source: Some("rgb.png".into()),
        media_type: Some("image/png".into()),
        status: SemanticStatus::Resolved,
        bytes: Some(png.clone()),
    });
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Image { source: "rgb.png".into(), alt: Vec::new(), title: None, asset: Some(AssetId(1)) }],
    });
    let exported = export_pdf_bytes(&graph).expect("export PNG PDF");
    let text = String::from_utf8_lossy(&exported);
    assert!(text.contains("/Width 2 /Height 1 /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode"));
    assert!(text.contains("/Predictor 15 /Colors 3 /BitsPerComponent 8 /Columns 2"));
    let reopened = import_pdf_bytes("png-round-trip.pdf", &exported).expect("reopen PNG PDF");
    let image = reopened.assets.iter().find(|asset| asset.media_type.as_deref() == Some("image/png")).expect("PNG asset after re-import");
    assert_eq!(image.bytes.as_deref(), Some(png.as_slice()));
}

#[test]
fn pdf_writer_rejects_indexed_and_alpha_pngs() {
    for color_type in [3, 4, 6] {
        let channels = if color_type == 3 {
            1
        }
        else if color_type == 4 {
            2
        }
        else {
            4
        };
        let png = png_fixture(1, 1, color_type, &vec![0; 1 + channels]);
        let mut graph = DocumentGraph::new(DocumentId(25));
        graph.push_asset(Asset {
            id: AssetId(1),
            kind: AssetKind::Image,
            content_identity: None,
            source: Some("unsupported.png".into()),
            media_type: Some("image/png".into()),
            status: SemanticStatus::Resolved,
            bytes: Some(png),
        });
        graph.push_block(Block::Paragraph {
            content: vec![Inline::Image { source: "unsupported.png".into(), alt: Vec::new(), title: None, asset: Some(AssetId(1)) }],
        });
        assert!(export_pdf_bytes(&graph).is_err());
    }
}

#[test]
fn images_and_text_share_page_height_without_resetting_to_top() {
    let mut graph = DocumentGraph::new(DocumentId(23));
    graph.push_asset(Asset {
        id: AssetId(1),
        kind: AssetKind::Image,
        content_identity: None,
        source: Some("photo.jpg".into()),
        media_type: Some("image/jpeg".into()),
        status: SemanticStatus::Resolved,
        bytes: Some(jpeg_frame_fixture(640, 480, 3)),
    });
    for index in 0..3 {
        graph.push_block(Block::Paragraph {
            content: vec![Inline::Image { source: "photo.jpg".into(), alt: Vec::new(), title: None, asset: Some(AssetId(1)) }],
        });
        graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: format!("after image {index}") }] });
    }
    let exported = export_pdf_bytes(&graph).expect("export image layout");
    let text = String::from_utf8_lossy(&exported);
    assert!(text.contains("/Count 2"));
    assert!(text.contains("q 400 0 0 300 106 470 cm"));
    assert!(text.contains("q 400 0 0 300 106 138 cm"));
    assert!(text.contains("72 454 Td\n(after image 0) Tj"));
    assert!(text.contains("72 122 Td\n(after image 1) Tj"));
    assert!(text.contains("72 454 Td\n(after image 2) Tj"));
}

#[test]
fn export_splits_long_documents_into_pages() {
    let mut graph = DocumentGraph::new(DocumentId(2));
    for index in 0..46 {
        graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: format!("line {index}") }] });
    }
    let bytes = export_pdf_bytes(&graph).expect("export PDF");
    assert!(bytes.windows(12).filter(|window| *window == b"/Type /Page ").count() >= 2);
}

#[test]
fn metadata_round_trips_through_ir_edit_and_pdf() {
    let mut graph = DocumentGraph::new(DocumentId(3));
    graph.metadata = DocumentMetadata {
        title: Some("Original title".into()),
        language: None,
        authors: vec!["Original author".into()],
        tags: vec!["one".into(), "two".into()],
        properties: Vec::new(),
    };
    let imported = import_pdf_bytes("metadata.pdf", &export_pdf_bytes(&graph).expect("export PDF")).expect("import PDF");
    assert_eq!(imported.metadata.title.as_deref(), Some("Original title"));
    assert_eq!(imported.metadata.authors, vec!["Original author"]);
    assert_eq!(imported.metadata.tags, vec!["one", "two"]);

    let mut edited = imported;
    edited.metadata.title = Some("Edited title".into());
    edited.metadata.authors = vec!["Edited author".into()];
    edited.metadata.tags = vec!["edited".into()];
    let reopened = import_pdf_bytes("edited-metadata.pdf", &export_pdf_bytes(&edited).expect("export edited PDF")).expect("re-import PDF");
    assert_eq!(reopened.metadata.title.as_deref(), Some("Edited title"));
    assert_eq!(reopened.metadata.authors, vec!["Edited author"]);
    assert_eq!(reopened.metadata.tags, vec!["edited"]);
}

#[test]
fn unicode_metadata_round_trips_without_body_font() {
    let mut graph = DocumentGraph::new(DocumentId(4));
    graph.metadata.title = Some("中文标题 😀 (编辑)".into());
    graph.metadata.authors = vec!["作者 Élodie".into()];
    graph.metadata.tags = vec!["文档".into(), "测试".into()];
    let bytes = export_pdf_bytes(&graph).expect("Unicode metadata needs no body font");
    assert!(bytes.windows(12).any(|window| window == b"/Title <FEFF"));
    let mut reopened = import_pdf_bytes("unicode-metadata.pdf", &bytes).expect("import Unicode metadata");
    assert_eq!(reopened.metadata, graph.metadata);
    reopened.metadata.title = Some("修改后 🚀".into());
    let edited = export_pdf_bytes(&reopened).expect("export edited metadata");
    let round = import_pdf_bytes("edited.pdf", &edited).expect("read edited metadata");
    assert_eq!(round.metadata, reopened.metadata);
}

#[test]
fn pdf_doc_encoding_body_text_round_trips_through_ir_edit_and_generation() {
    let mut graph = DocumentGraph::new(DocumentId(5));
    graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "café • € — déjà vu".into() }] });
    let bytes = export_pdf_bytes(&graph).expect("PDFDocEncoding body text");
    let mut imported = import_pdf_bytes("pdfdoc-body.pdf", &bytes).expect("import PDFDocEncoding body text");
    let Block::Paragraph { content } = &mut imported.blocks[0].block
    else {
        panic!("paragraph expected")
    };
    let Inline::Text { text } = &mut content[0]
    else {
        panic!("text expected")
    };
    text.push_str(" edited");
    let reopened = import_pdf_bytes("pdfdoc-body-edited.pdf", &export_pdf_bytes(&imported).expect("export edited PDFDocEncoding body text"))
        .expect("re-import edited PDFDocEncoding body text");
    let Block::Paragraph { content } = &reopened.blocks[0].block
    else {
        panic!("paragraph expected")
    };
    assert!(format!("{:?}", content).contains("café • € — déjà vu edited"));
}

#[test]
fn pdf_writer_rejects_body_text_outside_pdf_doc_encoding() {
    let mut graph = DocumentGraph::new(DocumentId(6));
    graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "中文正文".into() }] });
    let error = export_pdf_bytes(&graph).expect_err("full Unicode body needs an embedded font");
    assert!(error.to_string().contains("WinAnsiEncoding"));
}

#[test]
fn metadata_null_value_does_not_consume_next_dictionary_entry() {
    let bytes = b"%PDF-1.4\n1 0 obj\n<< /Title null /Author (Actual author) /Keywords <746167> >>\nendobj\ntrailer\n<< /Info 1 0 R >>\n%%EOF";
    let graph = import_pdf_bytes("null-title.pdf", bytes).expect("read metadata");
    assert_eq!(graph.metadata.title, None);
    assert_eq!(graph.metadata.authors, vec!["Actual author"]);
    assert_eq!(graph.metadata.tags, vec!["tag"]);
}

#[test]
fn pdf_import_reports_structures_without_claiming_to_preserve_them() {
    let bytes = b"%PDF-1.4
1 0 obj << /Type /Catalog /Pages 2 0 R /AcroForm 9 0 R >> endobj
2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj
3 0 obj << /Type /Page /Parent 2 0 R /Annots [8 0 R] /Contents 4 0 R >> endobj
4 0 obj << /Length 0 >> stream
endstream endobj
8 0 obj << /Type /Annot /Subtype /Link /A << /URI (https://example.test) >> >> endobj
9 0 obj << /Fields [] >> endobj
10 0 obj << /Encrypt 11 0 R >> endobj
%%EOF";
    let graph = import_pdf_bytes("structures.pdf", bytes).expect("import structural PDF");
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "import.pdf.annotations"));
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "import.pdf.forms"));
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "import.pdf.encryption" && loss.status == SemanticStatus::Unsupported));
}

#[test]
fn pdf_import_reports_unresolved_named_xobjects() {
    let bytes = b"%PDF-1.4\n1 0 obj\n<< /Length 8 >>\nstream\n/Fm0 Do\nendstream\nendobj\n%%EOF";
    let graph = import_pdf_bytes("unresolved-xobject.pdf", bytes).expect("import PDF");
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "import.pdf.unresolved_xobject"));
}

#[test]
fn imports_flate_images_with_predictor_one_without_filter_bytes() {
    let rows = [&[0x10, 0x20, 0x30, 0x40, 0x50, 0x60][..], &[0x70, 0x80, 0x90, 0xa0, 0xb0, 0xc0][..]];
    let encoded = rows.concat();
    let graph = import_pdf_bytes("predictor-one.pdf", &flate_image_pdf(1, &rows, &encoded)).expect("import predictor one image");
    let image = graph.assets.iter().find(|asset| asset.media_type.as_deref() == Some("image/png")).expect("PNG asset");
    assert_eq!(
        image.bytes.as_deref(),
        Some(png_fixture(2, 2, 2, &[0, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0, 0x70, 0x80, 0x90, 0xa0, 0xb0, 0xc0]).as_slice())
    );
}

#[test]
fn imports_flate_images_with_all_png_predictor_filters() {
    let raw_rows = [&[10, 20, 30, 40, 50, 60][..], &[70, 80, 90, 100, 110, 120][..]];
    for filter in 1u8..=4 {
        let mut encoded = Vec::new();
        let mut previous = [0u8; 6];
        for raw in raw_rows {
            encoded.push(filter);
            for offset in 0..raw.len() {
                let left = if offset >= 3 { raw[offset - 3] } else { 0 };
                let up = previous[offset];
                let upper_left = if offset >= 3 { previous[offset - 3] } else { 0 };
                let prediction = match filter {
                    1 => left,
                    2 => up,
                    3 => ((u16::from(left) + u16::from(up)) / 2) as u8,
                    4 => {
                        let p = i32::from(left) + i32::from(up) - i32::from(upper_left);
                        let pa = (p - i32::from(left)).abs();
                        let pb = (p - i32::from(up)).abs();
                        let pc = (p - i32::from(upper_left)).abs();
                        if pa <= pb && pa <= pc {
                            left
                        }
                        else if pb <= pc {
                            up
                        }
                        else {
                            upper_left
                        }
                    }
                    _ => unreachable!(),
                };
                encoded.push(raw[offset].wrapping_sub(prediction));
            }
            previous.copy_from_slice(raw);
        }
        let graph = import_pdf_bytes("predictor-filter.pdf", &flate_image_pdf(15, &raw_rows, &encoded)).expect("import filtered image");
        let image = graph.assets.iter().find(|asset| asset.media_type.as_deref() == Some("image/png")).expect("PNG asset");
        assert_eq!(
            image.bytes.as_deref(),
            Some(png_fixture(2, 2, 2, &[0, 10, 20, 30, 40, 50, 60, 0, 70, 80, 90, 100, 110, 120]).as_slice()),
            "filter {filter}"
        );
    }
}
/// One page with a fixed-metric font, so every glyph advances a quarter em.
fn quarter_em_font(objects: &mut String, font_id: u32) {
    let descriptor_id = font_id + 1;
    objects.push_str(&format!(
        "{font_id} 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /TestFont /Encoding /WinAnsiEncoding /FontDescriptor {descriptor_id} 0 R >>\nendobj\n{descriptor_id} 0 obj\n<< /Type /FontDescriptor /FontName /TestFont /Flags 4 /FontBBox [0 0 1000 1000] /ItalicAngle 0 /Ascent 800 /Descent -200 /CapHeight 700 /StemV 80 /MissingWidth 250 >>\nendobj\n"
    ));
}

fn text_page_pdf(media_box: &str, font_id: u32, content_id: u32, body: &str) -> Vec<u8> {
    let mut pdf = format!(
        "%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox {media_box} /Resources << /Font << /F1 {font_id} 0 R >> >> /Contents {content_id} 0 R >>\nendobj\n"
    );
    quarter_em_font(&mut pdf, font_id);
    pdf.push_str(&format!("{content_id} 0 obj\n<< /Length {} >>\nstream\n{body}\nendstream\nendobj\n%%EOF", body.len()));
    pdf.into_bytes()
}

#[test]
fn imports_pdf_page_geometry_and_body_metrics_from_the_text_layer() {
    let mut body = String::from("BT /F1 9.5 Tf\n");
    for (index, y) in [700.0, 690.0, 680.0, 670.0, 660.0, 650.0].into_iter().enumerate() {
        body.push_str(&format!("1 0 0 1 36 {y} Tm (row {index} of body text) Tj\n"));
    }
    body.push_str("ET");
    let bytes = text_page_pdf("[0 0 612 792]", 4, 6, &body);
    let graph = import_pdf_bytes("letter.pdf", &bytes).expect("import Letter PDF");
    let page = graph.page_setup.expect("measured page setup");
    assert_eq!(page.page_width, Some(612.0));
    assert_eq!(page.page_height, Some(792.0));
    assert_eq!(page.base_font_size, Some(9.5));
    assert_eq!(page.margin_left, Some(36.0));
    let line_height = page.line_height.expect("measured leading");
    assert!((line_height - 10.0 / 9.5).abs() < 0.01, "{line_height}");
    let margin_top = page.margin_top.expect("measured top margin");
    assert!((margin_top - (792.0 - 700.0 - 9.5)).abs() < 0.01, "{margin_top}");
}

fn table_rows(head: &[document_ir::TableRow], body: &[document_ir::TableRow]) -> Vec<document_ir::TableRow> {
    head.iter().chain(body).cloned().collect()
}

fn table_row_texts(graph: &DocumentGraph, row: &document_ir::TableRow) -> Vec<String> {
    row.cells
        .iter()
        .map(|cell| {
            cell.children
                .iter()
                .filter_map(|id| graph.block(*id))
                .flat_map(|node| match &node.block {
                    Block::Paragraph { content } => content.clone(),
                    _ => Vec::new(),
                })
                .map(|inline| match inline {
                    Inline::Text { text } => text,
                    _ => String::new(),
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
}

#[test]
fn merged_header_rows_with_span_blanks_join_the_table_grid() {
    let body = "BT /F1 12 Tf\n\
        1 0 0 1 72 700 Tm (Item) Tj\n\
        1 0 0 1 192 700 Tm (       ) Tj\n\
        1 0 0 1 357 700 Tm (Total) Tj\n\
        1 0 0 1 72 684 Tm (Name) Tj\n\
        1 0 0 1 192 684 Tm (Qty) Tj\n\
        1 0 0 1 357 684 Tm (Note) Tj\n\
        1 0 0 1 72 668 Tm (Pen) Tj\n\
        1 0 0 1 192 668 Tm (Two) Tj\n\
        1 0 0 1 357 668 Tm (Blue) Tj\n\
        1 0 0 1 72 652 Tm (Pad) Tj\n\
        1 0 0 1 192 652 Tm (One) Tj\n\
        1 0 0 1 357 652 Tm (White) Tj\n\
        ET";
    let bytes = text_page_pdf("[0 0 612 792]", 4, 6, body);
    let graph = import_pdf_bytes("merged-header.pdf", &bytes).expect("import merged header table");
    let table = graph
        .blocks
        .iter()
        .find_map(|node| match &node.block {
            Block::Table { head, body, .. } => Some(table_rows(head, body)),
            _ => None,
        })
        .expect("table block");
    assert_eq!(table.len(), 4, "{table:?}");
    assert_eq!(table_row_texts(&graph, &table[0]), vec!["Item".to_owned(), String::new(), "Total".to_owned()]);
    assert_eq!(table_row_texts(&graph, &table[1]), vec!["Name".to_owned(), "Qty".to_owned(), "Note".to_owned()]);
}

#[test]
fn plain_text_above_a_table_stays_outside_the_grid() {
    let body = "BT /F1 12 Tf\n\
        1 0 0 1 72 700 Tm (A plain sentence without column gaps.) Tj\n\
        1 0 0 1 72 684 Tm (Name) Tj\n\
        1 0 0 1 192 684 Tm (Qty) Tj\n\
        1 0 0 1 357 684 Tm (Note) Tj\n\
        1 0 0 1 72 668 Tm (Pen) Tj\n\
        1 0 0 1 192 668 Tm (Two) Tj\n\
        1 0 0 1 357 668 Tm (Blue) Tj\n\
        ET";
    let bytes = text_page_pdf("[0 0 612 792]", 4, 6, body);
    let graph = import_pdf_bytes("paragraph-table.pdf", &bytes).expect("import paragraph and table");
    let table = graph
        .blocks
        .iter()
        .find_map(|node| match &node.block {
            Block::Table { head, body, .. } => Some(table_rows(head, body)),
            _ => None,
        })
        .expect("table block");
    assert_eq!(table.len(), 2, "{table:?}");
    assert!(table.iter().all(|row| row.cells.len() == 3), "{table:?}");
    let paragraph = graph
        .blocks
        .iter()
        .find_map(|node| match &node.block {
            Block::Paragraph { content } => Some(format!("{content:?}")),
            _ => None,
        })
        .expect("paragraph block");
    assert!(paragraph.contains("plain sentence"), "{paragraph}");
}

#[test]
fn page_scoped_font_resources_do_not_leak_across_pages() {
    let first = "BT /F1 12 Tf (Alpha) Tj ET";
    let second = "BT /F1 12 Tf <0102> Tj ET";
    let cmap = "1 begincodespacerange\n<00> <FF>\nendcodespacerange\n2 beginbfchar\n<01> <4F60>\n<02> <597D>\nendbfchar";
    let bytes = format!(
        "%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 5 0 R >> >> /Contents 6 0 R >>\nendobj\n4 0 obj\n<< /Type /Page /Parent 2 0 R /Resources << /Font << /F1 7 0 R >> >> /Contents 8 0 R >>\nendobj\n5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>\nendobj\n6 0 obj\n<< /Length {} >>\nstream\n{first}\nendstream\nendobj\n7 0 obj\n<< /Type /Font /Subtype /Type0 /ToUnicode 9 0 R >>\nendobj\n8 0 obj\n<< /Length {} >>\nstream\n{second}\nendstream\nendobj\n9 0 obj\n<< /Length {} >>\nstream\n{cmap}\nendstream\nendobj\n%%EOF",
        first.len(),
        second.len(),
        cmap.len()
    );
    let graph = import_pdf_bytes("page-fonts.pdf", bytes.as_bytes()).expect("import page-scoped fonts");
    let paragraphs = graph
        .blocks
        .iter()
        .filter_map(|node| match &node.block {
            Block::Paragraph { content } => match content.as_slice() {
                [Inline::Text { text }] => Some(text.clone()),
                other => panic!("single text run expected, got {other:?}"),
            },
            _ => None,
        })
        .collect::<Vec<_>>();
    // `/F1` is bound to a WinAnsi font on the first page and to a Type0 font
    // with a ToUnicode map on the second. Neither binding may decode the other.
    assert_eq!(paragraphs, ["Alpha", "你好"]);
}

#[test]
fn nested_page_tree_inherits_resources_and_media_box() {
    let first = "BT /F1 12 Tf (First page) Tj ET";
    let second = "BT /F1 12 Tf (Second page) Tj ET";
    let bytes = format!(
        "%PDF-1.4\n1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n2 0 obj\n<< /Type /Pages /Kids [7 0 R] /Count 2 >>\nendobj\n7 0 obj\n<< /Type /Pages /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 5 0 R >> >> /Kids [3 0 R 4 0 R] /Count 2 >>\nendobj\n3 0 obj\n<< /Type /Page /Parent 7 0 R /Contents 6 0 R >>\nendobj\n4 0 obj\n<< /Type /Page /Parent 7 0 R /Contents 8 0 R >>\nendobj\n5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>\nendobj\n6 0 obj\n<< /Length {} >>\nstream\n{first}\nendstream\nendobj\n8 0 obj\n<< /Length {} >>\nstream\n{second}\nendstream\nendobj\n%%EOF",
        first.len(),
        second.len()
    );
    let graph = import_pdf_bytes("nested-pages.pdf", bytes.as_bytes()).expect("import nested page tree");
    let page = graph.page_setup.expect("inherited media box");
    assert_eq!(page.page_width, Some(612.0));
    assert_eq!(page.page_height, Some(792.0));
    let text = graph.blocks.iter().map(|node| format!("{:?}", node.block)).collect::<String>();
    assert!(text.find("First page").expect("first page text") < text.find("Second page").expect("second page text"));
}

#[test]
fn pdf_writer_uses_device_gray_for_grayscale_images_and_keeps_aspect_ratio() {
    let mut jpeg = DocumentGraph::new(DocumentId(30));
    jpeg.push_asset(Asset {
        id: AssetId(1),
        kind: AssetKind::Image,
        content_identity: None,
        source: Some("gray.jpg".into()),
        media_type: Some("image/jpeg".into()),
        status: SemanticStatus::Resolved,
        bytes: Some(jpeg_frame_fixture(200, 600, 1)),
    });
    jpeg.push_block(Block::Paragraph {
        content: vec![Inline::Image { source: "gray.jpg".into(), alt: Vec::new(), title: None, asset: Some(AssetId(1)) }],
    });
    let exported = export_pdf_bytes(&jpeg).expect("export grayscale JPEG");
    let text = String::from_utf8_lossy(&exported);
    assert!(text.contains("/ColorSpace /DeviceGray"), "{text}");
    // A 200x600 portrait image is capped at 300 points high and keeps its ratio.
    assert!(text.contains("q 100 0 0 300 256 470 cm"), "{text}");

    let png = png_fixture(2, 1, 0, &[0, 0x80, 0x40]);
    let mut png_graph = DocumentGraph::new(DocumentId(31));
    png_graph.push_asset(Asset {
        id: AssetId(1),
        kind: AssetKind::Image,
        content_identity: None,
        source: Some("gray.png".into()),
        media_type: Some("image/png".into()),
        status: SemanticStatus::Resolved,
        bytes: Some(png.clone()),
    });
    png_graph.push_block(Block::Paragraph {
        content: vec![Inline::Image { source: "gray.png".into(), alt: Vec::new(), title: None, asset: Some(AssetId(1)) }],
    });
    let exported = export_pdf_bytes(&png_graph).expect("export grayscale PNG");
    let text = String::from_utf8_lossy(&exported);
    assert!(text.contains("/ColorSpace /DeviceGray"), "{text}");
    assert!(text.contains("/Predictor 15 /Colors 1 /BitsPerComponent 8 /Columns 2"), "{text}");
    let reopened = import_pdf_bytes("gray-round-trip.pdf", &exported).expect("reopen grayscale PNG");
    let image = reopened.assets.iter().find(|asset| asset.media_type.as_deref() == Some("image/png")).expect("grayscale PNG asset");
    assert_eq!(image.bytes.as_deref(), Some(png.as_slice()));
}

#[test]
fn unsupported_inline_images_are_skipped_as_whole_blocks() {
    let content = b"BI\n/F /AHx\n/W 1\n/H 1\nID\n00\nEI\nBT (After) Tj ET";
    let mut bytes = format!("%PDF-1.4\n1 0 obj\n<< /Length {} >>\nstream\n", content.len()).into_bytes();
    bytes.extend_from_slice(content);
    bytes.extend_from_slice(b"\nendstream\nendobj\n%%EOF");
    let graph = import_pdf_bytes("unsupported-inline.pdf", &bytes).expect("import unsupported inline image");
    assert!(
        graph.coverage.loss.iter().any(|loss| loss.code == "import.pdf.unsupported_inline_image" && loss.status == SemanticStatus::Unsupported),
        "{:?}",
        graph.coverage.loss
    );
    assert!(graph.blocks.iter().any(|node| format!("{:?}", node.block).contains("After")));
}

mod import_internal;
