use document_formats::{
    export::{docx::export_docx_bytes, markdown::export_markdown},
    import::{docx::import_docx_bytes, markdown::import_markdown_bytes},
};
use document_ir::{
    Asset, AssetId, AssetKind, AttributePair, Attributes, Block, DocumentGraph, DocumentId, Inline, ListItem, NodeId, RelationEndpoint,
    SemanticStatus, TableCell, TableRow,
};

fn png_header(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    bytes.extend_from_slice(&13u32.to_be_bytes());
    bytes.extend_from_slice(b"IHDR");
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&[8, 2, 0, 0, 0]);
    bytes.extend_from_slice(&crc32(&bytes[12..]).to_be_bytes());
    bytes
}

fn graph_with_image(bytes: Vec<u8>, target: &str) -> DocumentGraph {
    let mut graph = DocumentGraph::new(DocumentId(50));
    graph.push_asset(Asset {
        id: AssetId(1),
        kind: AssetKind::Image,
        content_identity: None,
        source: Some(target.into()),
        media_type: None,
        status: SemanticStatus::Resolved,
        bytes: Some(bytes),
    });
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Image {
            source: target.into(),
            alt: vec![Inline::Text { text: "Image".into() }],
            title: None,
            asset: Some(AssetId(1)),
        }],
    });
    graph
}

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
    stored_zip(&[("word/document.xml", document_xml)])
}

#[test]
fn docx_export_round_trips_paragraphs_and_headings() {
    let zip = minimal_docx_zip();
    let graph = import_docx_bytes("sample.docx", &zip).expect("import docx");
    let exported = export_docx_bytes(&graph).expect("export docx");
    let round = import_docx_bytes("round.docx", &exported).expect("re-import docx");
    let markdown = export_markdown(&round).expect("export markdown");
    assert!(markdown.contains("Hello DOCX"));
    assert!(markdown.contains("# Title"));
}

#[test]
fn docx_export_writes_an_unknown_inline_style_as_a_run_style() {
    let mut graph = DocumentGraph::new(DocumentId(76));
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Styled { style: "zzstyle".into(), children: vec![Inline::Text { text: "ZZR53ZZ".into() }] }],
    });

    let exported = export_docx_bytes(&graph).expect("an unknown style must not reject the DOCX export");
    let xml = String::from_utf8_lossy(&exported);
    assert!(xml.contains(r#"w:rStyle w:val="zzstyle""#), "the style name travels as a run style: {xml}");
    assert!(xml.contains("ZZR53ZZ"), "the wrapped text stays: {xml}");
}

#[test]
fn docx_export_of_a_known_style_writes_no_style_id() {
    let mut graph = DocumentGraph::new(DocumentId(77));
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Styled { style: "bold".into(), children: vec![Inline::Text { text: "ZZ_BOLD_ZZ".into() }] }],
    });

    let exported = export_docx_bytes(&graph).expect("export bold run");
    let xml = String::from_utf8_lossy(&exported);
    assert!(xml.contains("<w:b/>"), "bold still maps to its native property: {xml}");
    assert!(!xml.contains(r#"w:rStyle w:val="bold""#), "a known style needs no style id: {xml}");
}

#[test]
fn docx_export_keeps_an_opaque_kind_and_payload_text_and_reads_them_back() {
    let mut graph = DocumentGraph::new(DocumentId(74));
    graph.push_block(Block::Opaque { kind: "zz-kind".into(), payload_hint: "ZZ_OPAQUEHINT_ZZ".into(), status: SemanticStatus::Resolved });

    let exported = export_docx_bytes(&graph).expect("export opaque block");
    let xml = String::from_utf8_lossy(&exported);
    assert!(xml.contains(r#"w:pStyle w:val="Opaque:zz-kind""#), "the kind travels in the paragraph style: {xml}");
    assert!(xml.contains("ZZ_OPAQUEHINT_ZZ"), "the payload text stays visible: {xml}");

    let reopened = import_docx_bytes("opaque.docx", &exported).expect("reopen DOCX opaque");
    let opaque = reopened.blocks.iter().find_map(|node| match &node.block {
        Block::Opaque { kind, payload_hint, .. } => Some((kind.clone(), payload_hint.clone())),
        _ => None,
    });
    assert_eq!(opaque, Some(("zz-kind".into(), "ZZ_OPAQUEHINT_ZZ".into())), "{:?}", reopened.blocks);
}

#[test]
fn docx_export_of_a_plain_paragraph_writes_no_opaque_style() {
    let mut graph = DocumentGraph::new(DocumentId(75));
    graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "Plain body.".into() }] });

    let exported = export_docx_bytes(&graph).expect("export paragraph");
    let xml = String::from_utf8_lossy(&exported);
    assert!(!xml.contains("Opaque:"), "a paragraph carries no opaque kind: {xml}");
}

#[test]
fn docx_export_keeps_a_code_language_and_reads_it_back() {
    let mut graph = DocumentGraph::new(DocumentId(72));
    graph.push_block(Block::Code { language: Some("zzlang".into()), content: "let x = 1;".into() });

    let exported = export_docx_bytes(&graph).expect("export code");
    let xml = String::from_utf8_lossy(&exported);
    assert!(xml.contains(r#"w:pStyle w:val="Code:zzlang""#), "the language travels in the paragraph style: {xml}");

    let reopened = import_docx_bytes("code-language.docx", &exported).expect("reopen DOCX code");
    let language = reopened.blocks.iter().find_map(|node| match &node.block {
        Block::Code { language, .. } => Some(language.clone()),
        _ => None,
    });
    assert_eq!(language, Some(Some("zzlang".into())), "the code language must reopen: {:?}", reopened.blocks);
}

#[test]
fn docx_export_of_a_code_block_without_a_language_writes_no_code_style() {
    let mut graph = DocumentGraph::new(DocumentId(73));
    graph.push_block(Block::Code { language: None, content: "plain body".into() });

    let exported = export_docx_bytes(&graph).expect("export code");
    let xml = String::from_utf8_lossy(&exported);
    assert!(!xml.contains("Code:"), "an absent language needs no style id: {xml}");
}

#[test]
fn docx_export_keeps_authored_inline_id_and_class_attributes() {
    let mut graph = DocumentGraph::new(DocumentId(70));
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Attributed {
            attributes: Attributes { id: Some("zzr49-id".into()), classes: vec!["zzr49-class".into()], pairs: Vec::new() },
            children: vec![Inline::Text { text: "ZZR49ZZ".into() }],
        }],
    });

    let exported = export_docx_bytes(&graph).expect("export attributed paragraph");
    let xml = String::from_utf8_lossy(&exported);
    assert!(xml.contains("zzr49-id"), "the authored id must survive as a bookmark: {xml}");
    assert!(xml.contains("zzr49-class"), "the authored class must survive as a run style: {xml}");
    assert!(xml.contains("ZZR49ZZ"), "the wrapped text must still be there: {xml}");
}

#[test]
fn docx_export_leaves_a_plain_run_without_a_bookmark_or_style_id() {
    let mut graph = DocumentGraph::new(DocumentId(71));
    graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "ZZ_PLAIN_ZZ".into() }] });

    let exported = export_docx_bytes(&graph).expect("export plain paragraph");
    let xml = String::from_utf8_lossy(&exported);
    assert!(!xml.contains("w:bookmarkStart"), "a plain run needs no bookmark: {xml}");
    assert!(!xml.contains("w:rStyle"), "a plain run needs no authored style id: {xml}");
}

#[test]
fn docx_export_round_trips_underline_and_strike_styles() {
    let mut graph = DocumentGraph::new(DocumentId(54));
    graph.push_block(Block::Paragraph {
        content: vec![
            Inline::Styled { style: "underline".into(), children: vec![Inline::Text { text: "under".into() }] },
            Inline::Styled {
                style: "strike".into(),
                children: vec![Inline::Styled { style: "bold".into(), children: vec![Inline::Text { text: " crossed".into() }] }],
            },
        ],
    });
    let exported = export_docx_bytes(&graph).expect("export styles");
    let xml = String::from_utf8_lossy(&exported);
    assert!(xml.contains("<w:u w:val=\"single\"/>"));
    assert!(xml.contains("<w:strike/>"));
    let reopened = import_docx_bytes("styled-round.docx", &exported).expect("reopen DOCX");
    let Block::Paragraph { content } = &reopened.blocks[0].block
    else {
        panic!("paragraph expected")
    };
    assert!(matches!(&content[0], Inline::Styled { style, .. } if style == "underline"));
    assert!(
        matches!(&content[1], Inline::Styled { style, children } if style == "bold" && matches!(&children[0], Inline::Styled { style, .. } if style == "strike"))
    );
}

#[test]
fn docx_export_accepts_the_xlsx_strong_style_as_bold() {
    let mut graph = DocumentGraph::new(DocumentId(55));
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Styled { style: "strong".into(), children: vec![Inline::Text { text: "strong".into() }] }],
    });
    let exported = export_docx_bytes(&graph).expect("xlsx bold cells must not fail the DOCX export");
    assert!(String::from_utf8_lossy(&exported).contains("<w:b/>"));
    let reopened = import_docx_bytes("strong-round.docx", &exported).expect("reopen DOCX");
    let Block::Paragraph { content } = &reopened.blocks[0].block
    else {
        panic!("paragraph expected")
    };
    assert!(matches!(&content[0], Inline::Styled { style, .. } if style == "bold"));
}

#[test]
fn docx_export_round_trips_vertical_alignment_styles() {
    let mut graph = DocumentGraph::new(DocumentId(56));
    graph.push_block(Block::Paragraph {
        content: vec![
            Inline::Styled { style: "superscript".into(), children: vec![Inline::Text { text: "2".into() }] },
            Inline::Styled { style: "subscript".into(), children: vec![Inline::Text { text: "i".into() }] },
        ],
    });
    let exported = export_docx_bytes(&graph).expect("export vertical alignment");
    let xml = String::from_utf8_lossy(&exported);
    assert!(xml.contains("w:vertAlign w:val=\"superscript\""));
    assert!(xml.contains("w:vertAlign w:val=\"subscript\""));
    let reopened = import_docx_bytes("vertical-round.docx", &exported).expect("reopen vertical alignment");
    let Block::Paragraph { content } = &reopened.blocks[0].block
    else {
        panic!("paragraph expected")
    };
    assert!(matches!(&content[0], Inline::Styled { style, .. } if style == "superscript"));
    assert!(matches!(&content[1], Inline::Styled { style, .. } if style == "subscript"));
}

#[test]
fn docx_export_round_trips_color_style() {
    let mut graph = DocumentGraph::new(DocumentId(57));
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Styled { style: "color:ff0000".into(), children: vec![Inline::Text { text: "red".into() }] }],
    });
    let exported = export_docx_bytes(&graph).expect("export color");
    assert!(String::from_utf8_lossy(&exported).contains("<w:color w:val=\"ff0000\"/>"));
    let reopened = import_docx_bytes("color-round.docx", &exported).expect("reopen color");
    let Block::Paragraph { content } = &reopened.blocks[0].block
    else {
        panic!("paragraph expected")
    };
    assert!(matches!(&content[0], Inline::Styled { style, .. } if style == "color:ff0000"));
}

#[test]
fn docx_export_round_trips_valued_character_styles() {
    let mut graph = DocumentGraph::new(DocumentId(58));
    let styled = Inline::Styled {
        style: "font-size:28".into(),
        children: vec![Inline::Styled {
            style: "font-family:Aptos".into(),
            children: vec![Inline::Styled { style: "highlight:yellow".into(), children: vec![Inline::Text { text: "styled".into() }] }],
        }],
    };
    graph.push_block(Block::Paragraph { content: vec![styled] });
    let exported = export_docx_bytes(&graph).expect("export valued styles");
    let xml = String::from_utf8_lossy(&exported);
    assert!(xml.contains("<w:sz w:val=\"28\"/>"));
    assert!(xml.contains("<w:rFonts w:ascii=\"Aptos\" w:hAnsi=\"Aptos\"/>"));
    assert!(xml.contains("<w:highlight w:val=\"yellow\"/>"));
    let reopened = import_docx_bytes("valued-character-styles-round.docx", &exported).expect("reopen valued styles");
    let Block::Paragraph { content } = &reopened.blocks[0].block
    else {
        panic!("paragraph expected")
    };
    let rendered = format!("{:?}", content);
    assert!(rendered.contains("font-size:28"), "{rendered}");
    assert!(rendered.contains("font-family:Aptos"), "{rendered}");
    assert!(rendered.contains("highlight:yellow"), "{rendered}");
}

#[test]
fn docx_inline_image_order_survives_ir_edit_and_regeneration() {
    let image = r#"<w:drawing><wp:inline><wp:docPr descr="Logo"/><a:graphic><a:graphicData><a:blip r:embed="rId1"/></a:graphicData></a:graphic></wp:inline></w:drawing>"#;
    let document = format!(
        r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><w:body><w:p><w:r><w:rPr><w:b/></w:rPr><w:t>Before</w:t>{image}<w:t>After</w:t></w:r><w:del><w:r>{image}</w:r></w:del><w:ins><w:r><w:t>Inserted</w:t>{image}<w:t>Tail</w:t></w:r></w:ins></w:p></w:body></w:document>"#
    );
    let rels = br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/logo.png"/></Relationships>"#;
    let png = png_header(1, 1);
    let zip = stored_zip(&[("word/document.xml", document.as_bytes()), ("word/_rels/document.xml.rels", rels), ("word/media/logo.png", &png)]);
    let mut graph = import_docx_bytes("ordered.docx", &zip).expect("import inline order");
    let before = export_markdown(&graph).expect("render inline order");
    assert!(before.contains("**Before**![Logo](media/logo.png)**After**Inserted![Logo](media/logo.png)Tail"), "{before}");
    assert_eq!(before.matches("![Logo]").count(), 2);
    let Block::Paragraph { content } = &mut graph.blocks[0].block
    else {
        panic!("paragraph expected")
    };
    content.push(Inline::Text { text: "Edited".into() });
    let bytes = export_docx_bytes(&graph).expect("generate edited DOCX");
    let reopened = import_docx_bytes("edited.docx", &bytes).expect("reopen edited DOCX");
    let after = export_markdown(&reopened).expect("render edited DOCX");
    assert!(after.contains("**Before**![Logo](media/logo.png)**After**Inserted![Logo](media/logo.png)TailEdited"), "{after}");
}

#[test]
fn docx_imports_core_metadata_into_ir() {
    let document = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Body</w:t></w:r></w:p></w:body></w:document>"#;
    let core = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>Document title</dc:title><dc:creator>Author</dc:creator><dc:language>zh-CN</dc:language><cp:keywords>one, two, three</cp:keywords></cp:coreProperties>"#;
    let zip = stored_zip(&[("word/document.xml", document), ("docProps/core.xml", core)]);
    let graph = import_docx_bytes("metadata.docx", &zip).expect("import docx");
    assert_eq!(graph.metadata.title.as_deref(), Some("Document title"));
    assert_eq!(graph.metadata.language.as_deref(), Some("zh-CN"));
    assert_eq!(graph.metadata.authors, vec!["Author"]);
    assert_eq!(graph.metadata.tags, vec!["one", "two", "three"]);
    let exported = export_docx_bytes(&graph).expect("export docx");
    let round = import_docx_bytes("metadata-round.docx", &exported).expect("re-import docx");
    assert_eq!(round.metadata.title.as_deref(), Some("Document title"));
    assert_eq!(round.metadata.authors, vec!["Author"]);
    assert_eq!(round.metadata.tags, vec!["one", "two", "three"]);
}

#[test]
fn docx_export_round_trips_lists_with_numbering() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr>
      <w:r><w:t>One</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr>
      <w:r><w:t>Two</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
    let zip = stored_zip(&[("word/document.xml", document_xml)]);
    let graph = import_docx_bytes("list.docx", &zip).expect("import docx");
    let exported = export_docx_bytes(&graph).expect("export docx");
    let payload = String::from_utf8_lossy(&exported);
    assert!(payload.contains("word/numbering.xml"));
    assert!(
        payload.contains("Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering\" Target=\"numbering.xml\"/>"),
        "the document part must point at the numbering part: {payload}"
    );
    assert!(payload.contains("w:ilvl=\"1\"><w:numFmt"));
    let round = import_docx_bytes("round.docx", &exported).expect("re-import docx");
    let markdown = export_markdown(&round).expect("export markdown");
    assert!(markdown.contains("- One"));
    assert!(markdown.contains("- Two"));
}

#[test]
fn docx_import_and_export_preserve_nested_list_levels() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t>Parent</w:t></w:r></w:p>
    <w:p><w:pPr><w:numPr><w:ilvl w:val="1"/><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t>Child</w:t></w:r></w:p>
    <w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr><w:r><w:t>Sibling</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
    let zip = stored_zip(&[("word/document.xml", document_xml)]);
    let graph = import_docx_bytes("nested.docx", &zip).expect("import docx");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("- Parent\n  - Child\n- Sibling"), "markdown was: {markdown}");
    let exported = export_docx_bytes(&graph).expect("export docx");
    let round = import_docx_bytes("round.docx", &exported).expect("re-import docx");
    let round_markdown = export_markdown(&round).expect("export markdown");
    assert!(round_markdown.contains("- Parent\n  - Child\n- Sibling"), "round markdown was: {round_markdown}");
}

#[test]
fn docx_export_includes_minimal_opc_parts() {
    let zip = minimal_docx_zip();
    let graph = import_docx_bytes("sample.docx", &zip).expect("import docx");
    let exported = export_docx_bytes(&graph).expect("export docx");
    let members = String::from_utf8_lossy(&exported);
    assert!(members.contains("[Content_Types].xml"));
    assert!(members.contains("_rels/.rels"));
    assert!(members.contains("word/_rels/document.xml.rels"));
    assert!(members.contains("word/document.xml"));
}

#[test]
fn docx_export_round_trips_tables() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:tbl>
      <w:tr>
        <w:tc><w:p><w:r><w:t>H1</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>H2</w:t></w:r></w:p></w:tc>
      </w:tr>
      <w:tr>
        <w:tc><w:p><w:r><w:t>A</w:t></w:r></w:p></w:tc>
        <w:tc><w:p><w:r><w:t>B</w:t></w:r></w:p></w:tc>
      </w:tr>
    </w:tbl>
  </w:body>
</w:document>"#;
    let zip = stored_zip(&[("word/document.xml", document_xml)]);
    let graph = import_docx_bytes("table.docx", &zip).expect("import docx");
    let exported = export_docx_bytes(&graph).expect("export docx");
    let exported_xml = String::from_utf8_lossy(&exported);
    // A table without `w:tblPr` opens borderless and lets Word autofit the
    // columns from content. The export must declare the width, the grid and
    // the borders itself.
    assert!(exported_xml.contains("<w:tblPr>"), "{exported_xml}");
    assert!(exported_xml.contains("<w:tblW w:w=\"9026\" w:type=\"dxa\"/>"), "{exported_xml}");
    assert!(exported_xml.contains("<w:tblBorders>"), "{exported_xml}");
    for side in ["top", "left", "bottom", "right", "insideH", "insideV"] {
        assert!(exported_xml.contains(&format!("<w:{side} w:val=\"single\"")), "{side} border is missing");
    }
    assert!(
        exported_xml.contains("<w:tblGrid><w:gridCol w:w=\"4513\"/><w:gridCol w:w=\"4513\"/></w:tblGrid>"),
        "the two columns must split the text box equally: {exported_xml}"
    );
    assert!(exported_xml.contains("<w:tcW w:w=\"4513\" w:type=\"dxa\"/>"), "{exported_xml}");
    assert!(
        !exported_xml.contains("relationships/numbering"),
        "a table-only document must not declare the numbering relationship: {exported_xml}"
    );
    let round = import_docx_bytes("round.docx", &exported).expect("re-import docx");
    let markdown = export_markdown(&round).expect("export markdown");
    assert!(markdown.contains("| H1 | H2 |"));
    assert!(markdown.contains("| A | B |"));
}

#[test]
fn docx_export_writes_grid_span_and_vertical_merge() {
    let mut graph = DocumentGraph::new(DocumentId(51));
    let wide = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "Wide".into() }] });
    let tall = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "Tall".into() }] });
    let r1c2 = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "R1C2".into() }] });
    let r2c2 = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "R2C2".into() }] });
    graph.push_block(Block::Table {
        caption: None,
        head: vec![TableRow { cells: vec![TableCell { children: vec![wide], colspan: 2, rowspan: 1 }] }],
        body: vec![
            TableRow {
                cells: vec![
                    TableCell { children: vec![tall], colspan: 1, rowspan: 2 },
                    TableCell { children: vec![r1c2], colspan: 1, rowspan: 1 },
                ],
            },
            TableRow { cells: vec![TableCell { children: vec![r2c2], colspan: 1, rowspan: 1 }] },
        ],
        foot: Vec::new(),
    });
    let exported = export_docx_bytes(&graph).expect("export docx");
    let exported_xml = String::from_utf8_lossy(&exported);
    // The grid follows the spans, not the raw cell count: the first row has a
    // single cell that still occupies two columns.
    assert!(exported_xml.contains("<w:tblGrid><w:gridCol w:w=\"4513\"/><w:gridCol w:w=\"4513\"/></w:tblGrid>"), "{exported_xml}");
    assert!(
        exported_xml.contains("<w:tcW w:w=\"9026\" w:type=\"dxa\"/><w:gridSpan w:val=\"2\"/></w:tcPr>"),
        "a `colspan` cell must span both grid columns: {exported_xml}"
    );
    assert!(
        exported_xml.contains("<w:tcW w:w=\"4513\" w:type=\"dxa\"/><w:vMerge w:val=\"restart\"/></w:tcPr><w:p>"),
        "a `rowspan` cell must restart the vertical merge: {exported_xml}"
    );
    assert!(
        exported_xml.contains("<w:tcW w:w=\"4513\" w:type=\"dxa\"/><w:vMerge/></w:tcPr><w:p/></w:tc>"),
        "the next row must continue the vertical merge with an empty cell: {exported_xml}"
    );
    assert_eq!(exported_xml.matches("<w:vMerge w:val=\"restart\"/>").count(), 1, "{exported_xml}");
    assert_eq!(exported_xml.matches("<w:vMerge/>").count(), 1, "{exported_xml}");
    let continuation = exported_xml.find("<w:vMerge/>").expect("continuation cell");
    let trailing = exported_xml.find("R2C2").expect("second row cell content");
    assert!(continuation < trailing, "the continuation slot must precede the row's own cell: {exported_xml}");
    // Reopening rebuilds the same spans: `gridSpan` becomes `colspan`, and a
    // continuation cell extends the origin cell's `rowspan` instead of adding a
    // fifth cell to the table.
    let round = import_docx_bytes("spans.docx", &exported).expect("re-import spanned docx");
    let (head, body) = round
        .blocks
        .iter()
        .find_map(|node| match &node.block {
            Block::Table { head, body, .. } => Some((head.as_slice(), body.as_slice())),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected a table, got {:#?}", round.blocks));
    assert_eq!(head.len(), 1, "{head:#?}");
    assert_eq!(body.len(), 2, "{body:#?}");
    assert_eq!(head[0].cells.len(), 1, "{head:#?}");
    assert_eq!((head[0].cells[0].colspan, head[0].cells[0].rowspan), (2, 1));
    assert_eq!(body[0].cells.len(), 2, "{body:#?}");
    assert_eq!((body[0].cells[0].colspan, body[0].cells[0].rowspan), (1, 2));
    assert_eq!(body[1].cells.len(), 1, "{body:#?}");
}

#[test]
fn docx_export_assigns_ids_to_named_footnotes() {
    let graph = import_markdown_bytes("named-footnote.md", "See[^note] and[^1].\n\n[^note]: Named body.\n\n[^1]: Numeric body.\n")
        .expect("import markdown");
    let exported = export_docx_bytes(&graph).expect("a named footnote must not fail the export");
    let payload = String::from_utf8_lossy(&exported);
    // `w:footnoteReference` only takes integers, so `[^1]` keeps id 1 and the
    // named label takes the next free id.
    assert!(payload.contains(r#"<w:footnoteReference w:id="2"/>"#), "the named reference keeps its own id: {payload}");
    assert!(payload.contains(r#"<w:footnoteReference w:id="1"/>"#), "the numeric reference keeps id 1: {payload}");
    assert!(
        payload.contains(r#"<w:footnote w:id="1"><w:p><w:r><w:t>Numeric body.</w:t>"#),
        "the numeric body stays attached to id 1: {payload}"
    );
    assert!(payload.contains(r#"<w:footnote w:id="2"><w:p><w:r><w:t>Named body.</w:t>"#), "the named body stays attached to id 2: {payload}");
    let round = import_docx_bytes("named-footnote-round.docx", &exported).expect("reopen docx");
    let labels: Vec<String> = round
        .blocks
        .iter()
        .flat_map(|node| match &node.block {
            Block::Paragraph { content } => content
                .iter()
                .filter_map(|inline| match inline {
                    Inline::Note { label, .. } => Some(label.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        })
        .collect();
    assert_eq!(labels, vec!["2".to_string(), "1".to_string()], "{:?}", round.blocks);
    let definitions =
        round.blocks.iter().filter(|node| matches!(&node.block, Block::Opaque { kind, .. } if kind == "footnote_definition")).count();
    assert_eq!(definitions, 2, "{:?}", round.blocks);
}

#[test]
fn docx_export_writes_undefined_note_reference_as_text() {
    let mut graph = DocumentGraph::new(DocumentId(52));
    graph.push_block(Block::Paragraph { content: vec![Inline::Note { label: "missing".into(), display: None }] });
    let exported = export_docx_bytes(&graph).expect("an unresolved note must not fail the export");
    let payload = String::from_utf8_lossy(&exported);
    assert!(!payload.contains("w:footnoteReference"), "no reference may point at an absent part: {payload}");
    assert!(payload.contains("[^missing]"), "the authored token stays visible: {payload}");
}

#[test]
fn docx_export_writes_footnote_separators() {
    let graph = import_markdown_bytes("separators.md", "One[^1].\n\n[^1]: Body.\n").expect("import markdown");
    let exported = export_docx_bytes(&graph).expect("export docx");
    let payload = String::from_utf8_lossy(&exported);
    assert!(
        payload.contains(r#"<w:footnote w:type="separator" w:id="-1">"#),
        "the reserved separator note must precede the content notes: {payload}"
    );
    assert!(
        payload.contains(r#"<w:footnote w:type="continuationSeparator" w:id="0">"#),
        "the reserved continuation separator must be present: {payload}"
    );
    assert!(payload.contains("<w:separator/>"), "the separator run draws the rule: {payload}");
    assert!(payload.contains("<w:continuationSeparator/>"), "the continuation separator is drawn: {payload}");
}

#[test]
fn docx_export_round_trips_hyperlinks() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <w:body>
    <w:p>
      <w:hyperlink r:id="rId1">
        <w:r><w:t>Example</w:t></w:r>
      </w:hyperlink>
    </w:p>
  </w:body>
</w:document>"#;
    let rels_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
    Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink"
    Target="https://example.com" TargetMode="External"/>
</Relationships>"#;
    let zip = stored_zip(&[("word/document.xml", document_xml), ("word/_rels/document.xml.rels", rels_xml)]);
    let graph = import_docx_bytes("links.docx", &zip).expect("import docx");
    let exported = export_docx_bytes(&graph).expect("export docx");
    let payload = String::from_utf8_lossy(&exported);
    assert!(payload.contains("https://example.com"));
    let round = import_docx_bytes("round.docx", &exported).expect("re-import docx");
    let markdown = export_markdown(&round).expect("export markdown");
    assert!(markdown.contains("[Example](https://example.com)"));
}

#[test]
fn docx_export_round_trips_footnotes() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:r><w:t>See</w:t></w:r>
      <w:r><w:footnoteReference w:id="1"/></w:r>
      <w:r><w:t> for details.</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
    let footnotes_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:footnote w:id="1">
    <w:p><w:r><w:t>Footnote body.</w:t></w:r></w:p>
  </w:footnote>
</w:footnotes>"#;
    let rels_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
    Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/footnotes"
    Target="footnotes.xml"/>
</Relationships>"#;
    let zip =
        stored_zip(&[("word/document.xml", document_xml), ("word/_rels/document.xml.rels", rels_xml), ("word/footnotes.xml", footnotes_xml)]);
    let graph = import_docx_bytes("footnotes.docx", &zip).expect("import docx");
    let exported = export_docx_bytes(&graph).expect("export docx");
    let payload = String::from_utf8_lossy(&exported);
    assert!(payload.contains("word/footnotes.xml"));
    let round = import_docx_bytes("round.docx", &exported).expect("re-import docx");
    let markdown = export_markdown(&round).expect("export markdown");
    assert!(markdown.contains("See[^1] for details."));
    assert!(markdown.contains("[^1]: Footnote body."));
}

#[test]
fn docx_export_keeps_citation_keys_and_titles() {
    let mut graph = graph_with_image(png_header(1, 1), "media/logo.png");
    let Block::Paragraph { content } = &mut graph.blocks[0].block
    else {
        panic!("image paragraph expected")
    };
    let Inline::Image { title, .. } = &mut content[0]
    else {
        panic!("image expected: {content:?}")
    };
    *title = Some("Logo title".into());

    graph.push_block(Block::Paragraph {
        content: vec![
            Inline::Citation { key: "smith2024".into(), display: Some(vec![Inline::Text { text: "(Smith 2024)".into() }]) },
            Inline::Text { text: " ".into() },
            Inline::Link {
                target: RelationEndpoint::ExternalUri("https://example.com".into()),
                display: vec![Inline::Text { text: "Example".into() }],
                title: Some("Example tooltip".into()),
            },
        ],
    });

    let exported = export_docx_bytes(&graph).expect("export docx");
    let payload = String::from_utf8_lossy(&exported);
    assert!(
        payload.contains(r#"<w:fldSimple w:instr=" CITATION smith2024 ">"#),
        "a citation key must survive in a Word citation field: {payload}"
    );
    assert!(payload.contains("(Smith 2024)"), "the authored display must stay the field result: {payload}");
    assert!(payload.contains(r#"w:tooltip="Example tooltip""#), "a link title must become a hyperlink tooltip: {payload}");
    assert!(payload.contains(r#"title="Logo title""#), "an image title must survive in the drawing properties: {payload}");
}

#[test]
fn docx_reader_recovers_citation_keys_and_titles_it_writes() {
    let mut graph = graph_with_image(png_header(1, 1), "media/logo.png");
    let Block::Paragraph { content } = &mut graph.blocks[0].block
    else {
        panic!("image paragraph expected")
    };
    let Inline::Image { title, .. } = &mut content[0]
    else {
        panic!("image expected: {content:?}")
    };
    *title = Some("Logo title".into());

    graph.push_block(Block::Paragraph {
        content: vec![
            Inline::Citation { key: "smith2024".into(), display: Some(vec![Inline::Text { text: "(Smith 2024)".into() }]) },
            Inline::Text { text: " ".into() },
            Inline::Link {
                target: RelationEndpoint::ExternalUri("https://example.com".into()),
                display: vec![Inline::Text { text: "Example".into() }],
                title: Some("Example tooltip".into()),
            },
        ],
    });

    let exported = export_docx_bytes(&graph).expect("export docx");
    let reopened = import_docx_bytes("fields-round.docx", &exported).expect("reopen docx");
    let dumped = format!("{:?}", reopened.blocks);

    let mut citation = None;
    let mut link = None;
    let mut image = None;
    for node in &reopened.blocks {
        let Block::Paragraph { content } = &node.block
        else {
            continue;
        };
        for inline in content {
            match inline {
                Inline::Citation { key, display } => citation = Some((key.clone(), display.clone())),
                Inline::Link { title, .. } => link = Some(title.clone()),
                Inline::Image { title, .. } => image = Some(title.clone()),
                _ => {}
            }
        }
    }

    assert_eq!(citation.as_ref().map(|entry| entry.0.as_str()), Some("smith2024"), "{dumped}");
    assert_eq!(citation.as_ref().and_then(|entry| entry.1.as_ref()), Some(&vec![Inline::Text { text: "(Smith 2024)".into() }]), "{dumped}");
    assert_eq!(link, Some(Some("Example tooltip".into())), "{dumped}");
    assert_eq!(image, Some(Some("Logo title".into())), "{dumped}");
}

#[test]
fn docx_export_round_trips_embedded_images() {
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
    let image_bytes = png_header(640, 480);
    let zip = stored_zip(&[
        ("word/document.xml", document_xml),
        ("word/_rels/document.xml.rels", rels_xml),
        ("word/media/logo.png", image_bytes.as_slice()),
    ]);
    let graph = import_docx_bytes("image.docx", &zip).expect("import docx");
    let exported = export_docx_bytes(&graph).expect("export docx");
    let payload = String::from_utf8_lossy(&exported);
    assert!(payload.contains("word/media/logo.png"));
    let round = import_docx_bytes("round.docx", &exported).expect("re-import docx");
    let markdown = export_markdown(&round).expect("export markdown");
    assert!(markdown.contains("![Logo](media/logo.png)"));
}

#[test]
fn docx_images_use_consistent_emu_extents_without_upscaling() {
    for (width, height, extent) in [(640, 480, (4_000_000, 3_000_000)), (100, 50, (952_500, 476_250)), (50, 100, (476_250, 952_500))] {
        let graph = graph_with_image(png_header(width, height), "media/size.png");
        let exported = export_docx_bytes(&graph).expect("export sized PNG");
        let xml = String::from_utf8_lossy(&exported);
        let (emu_width, emu_height) = extent;
        assert!(xml.contains(&format!("<wp:extent cx=\"{emu_width}\" cy=\"{emu_height}\"/>")));
        assert!(xml.contains(&format!("<a:ext cx=\"{emu_width}\" cy=\"{emu_height}\"/>")));
        let reopened = import_docx_bytes("sized.docx", &exported).expect("reopen sized PNG");
        assert_eq!(reopened.assets[0].bytes, Some(png_header(width, height)));
    }
}

#[test]
fn docx_export_writes_task_markers_for_task_items_only() {
    let mut graph = DocumentGraph::new(DocumentId(51));
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

    let exported = export_docx_bytes(&graph).expect("export task list DOCX");
    let payload = String::from_utf8_lossy(&exported);
    assert!(payload.contains(r#"<w:sym w:font="Wingdings" w:char="F0FE"/>"#), "a checked item needs a ticked box: {payload}");
    assert!(payload.contains(r#"<w:sym w:font="Wingdings" w:char="F0A8"/>"#), "an open item needs an empty box: {payload}");
    assert_eq!(payload.matches(r#"<w:sym w:font="Wingdings""#).count(), 2, "the plain item must stay marker-free: {payload}");
    assert!(payload.contains("Done") && payload.contains("Todo") && payload.contains("Plain"));
}

#[test]
fn docx_jpeg_frame_dimensions_determine_image_extent() {
    let bytes = vec![0xff, 0xd8, 0xff, 0xc0, 0, 11, 8, 0, 100, 0, 50, 1, 1, 0x11, 0, 0xff, 0xd9];
    let exported = export_docx_bytes(&graph_with_image(bytes, "media/size.jpg")).expect("export JPEG frame");
    assert!(String::from_utf8_lossy(&exported).contains("<wp:extent cx=\"476250\" cy=\"952500\"/>"));
}

#[test]
fn docx_rejects_truncated_or_zero_sized_raster_headers() {
    for bytes in [b"\x89PNG\r\n".to_vec(), png_header(0, 100), vec![0xff, 0xd8, 0xff, 0xd9]] {
        assert!(export_docx_bytes(&graph_with_image(bytes, "media/broken.png")).is_err());
    }
}

#[test]
fn docx_emits_section_and_list_children_in_semantic_order() {
    let mut graph = DocumentGraph::new(DocumentId(51));
    let paragraph = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "child paragraph".into() }] });
    let list = graph.push_block(Block::List {
        ordered: true,
        items: vec![ListItem {
            content: vec![Inline::Text { text: "list item".into() }],
            children: vec![paragraph],
            number: Some(1),
            ..Default::default()
        }],
    });
    graph.push_block(Block::Section { level: 1, title: vec![Inline::Text { text: "section heading".into() }], children: vec![list] });
    let exported = export_docx_bytes(&graph).expect("export section children");
    let xml = String::from_utf8_lossy(&exported);
    assert!(xml.contains("word/numbering.xml"));
    let reopened = import_docx_bytes("section.docx", &exported).expect("reopen section children");
    let markdown = export_markdown(&reopened).expect("render section children");
    assert_eq!(markdown.matches("child paragraph").count(), 1);
    assert_eq!(markdown.matches("list item").count(), 1);
    assert!(markdown.find("section heading").unwrap() < markdown.find("list item").unwrap());
    assert!(markdown.find("list item").unwrap() < markdown.find("child paragraph").unwrap());
}

#[test]
fn docx_rejects_cyclic_or_missing_containment_before_rendering() {
    let mut graph = DocumentGraph::new(DocumentId(52));
    graph.push_block_with_id(NodeId(1), Block::Section { level: 1, title: Vec::new(), children: vec![NodeId(1)] });
    assert!(export_docx_bytes(&graph).is_err());
    graph.blocks.clear();
    graph.push_block_with_id(NodeId(1), Block::Section { level: 1, title: Vec::new(), children: vec![NodeId(2)] });
    assert!(export_docx_bytes(&graph).is_err());
}

fn omml_fraction(source: &str) -> document_ir::MathFormula {
    document_ir::MathFormula {
        source: source.into(),
        language: Some("latex".into()),
        display: document_ir::MathDisplay::Display,
        expression: document_ir::MathExpression::Fraction {
            numerator: Box::new(document_ir::MathExpression::Number { text: "1".into(), span: None }),
            denominator: Box::new(document_ir::MathExpression::Number { text: "2".into(), span: None }),
            line: document_ir::MathFractionLine::Solid,
        },
    }
}

#[test]
fn docx_export_lowers_math_to_omml_and_reads_it_back() {
    let mut graph = DocumentGraph::new(DocumentId(60));
    graph.push_block(Block::Math { formula: omml_fraction("\\frac{1}{2}") });
    graph.push_block(Block::Paragraph {
        content: vec![Inline::InlineMath {
            formula: document_ir::MathFormula {
                source: "x^2".into(),
                language: Some("latex".into()),
                display: document_ir::MathDisplay::Inline,
                expression: document_ir::MathExpression::Script {
                    base: Box::new(document_ir::MathExpression::Atom(document_ir::MathAtom {
                        text: "x".into(),
                        class: document_ir::MathClass::Identifier,
                        span: None,
                    })),
                    superscript: Some(Box::new(document_ir::MathExpression::Number { text: "2".into(), span: None })),
                    subscript: None,
                },
            },
        }],
    });
    let exported = export_docx_bytes(&graph).expect("export math");
    let xml = String::from_utf8_lossy(&exported);
    assert!(xml.contains("xmlns:m=\"http://schemas.openxmlformats.org/officeDocument/2006/math\""), "the math namespace is declared");
    assert!(xml.contains("<m:oMathPara><m:oMath><m:f>"), "display math becomes an OMML fraction");
    assert!(xml.contains("<m:sSup>"), "inline math keeps its superscript");
    let reopened = import_docx_bytes("math-round.docx", &exported).expect("reopen math");
    let display_sources: Vec<String> = reopened
        .blocks
        .iter()
        .filter_map(|node| match &node.block {
            Block::Math { formula } => Some(formula.source.clone()),
            _ => None,
        })
        .collect();
    assert!(display_sources.iter().any(|source| source.contains("\\frac")), "{display_sources:?}");
    let inline_source = reopened.blocks.iter().find_map(|node| match &node.block {
        Block::Paragraph { content } => content.iter().find_map(|inline| match inline {
            Inline::InlineMath { formula } => Some(formula.source.clone()),
            _ => None,
        }),
        _ => None,
    });
    let inline_source = inline_source.expect("inline math survives the round trip");
    assert!(inline_source.contains('x'), "{inline_source:?}");
    assert!(inline_source.contains('2'), "{inline_source:?}");
}

#[test]
fn docx_export_keeps_cases_environment_rows_as_a_matrix() {
    let mut graph = DocumentGraph::new(DocumentId(61));
    graph.push_block(Block::Math {
        formula: document_ir::MathFormula {
            source: "\\begin{cases} x \\\\ y \\end{cases}".into(),
            language: Some("latex".into()),
            display: document_ir::MathDisplay::Display,
            expression: document_ir::MathExpression::Environment(document_ir::MathEnvironment {
                kind: document_ir::MathEnvironmentKind::Cases,
                name: "cases".into(),
                starred: false,
                rows: ["x", "y"]
                    .into_iter()
                    .map(|text| document_ir::MathRow {
                        cells: vec![document_ir::MathExpression::Atom(document_ir::MathAtom {
                            text: text.into(),
                            class: document_ir::MathClass::Identifier,
                            span: None,
                        })],
                        placement: document_ir::MathRowPlacement::Normal,
                        numbering: document_ir::MathRowNumbering::None,
                    })
                    .collect(),
                column_alignment: vec![document_ir::MathColumnAlignment::Left],
                column_pairs: None,
                span: None,
            }),
        },
    });
    let exported = export_docx_bytes(&graph).expect("export cases");
    let xml = String::from_utf8_lossy(&exported);
    assert!(xml.contains("<m:d><m:dPr><m:begChr m:val=\"{\"/>"), "cases keep the left brace");
    assert!(xml.contains("<m:m><m:mPr><m:mcs>"), "cases carry their authored left justification");
    assert!(xml.contains("<m:mr>"), "cases rows become an OMML matrix");
}

#[test]
fn docx_export_aligns_matrix_columns_with_mcs() {
    let mut graph = DocumentGraph::new(DocumentId(66));
    graph.push_block(Block::Math {
        formula: document_ir::MathFormula {
            source: "\\begin{matrix} a & b & c \\end{matrix}".into(),
            language: Some("latex".into()),
            display: document_ir::MathDisplay::Display,
            expression: document_ir::MathExpression::Environment(document_ir::MathEnvironment {
                kind: document_ir::MathEnvironmentKind::Matrix,
                name: "matrix".into(),
                starred: false,
                rows: vec![document_ir::MathRow {
                    cells: vec![math_atom("a"), math_atom("b"), math_atom("c")],
                    placement: document_ir::MathRowPlacement::Normal,
                    numbering: document_ir::MathRowNumbering::None,
                }],
                column_alignment: vec![
                    document_ir::MathColumnAlignment::Left,
                    document_ir::MathColumnAlignment::Left,
                    document_ir::MathColumnAlignment::Right,
                ],
                column_pairs: None,
                span: None,
            }),
        },
    });
    let exported = export_docx_bytes(&graph).expect("export aligned matrix");
    let xml = String::from_utf8_lossy(&exported);
    assert!(
        xml.contains(
            "<m:mPr><m:mcs><m:mc><m:mcPr><m:count m:val=\"2\"/><m:mcJc m:val=\"left\"/></m:mcPr></m:mc><m:mc><m:mcPr><m:count m:val=\"1\"/><m:mcJc m:val=\"right\"/></m:mcPr></m:mc></m:mcs></m:mPr>"
        ),
        "consecutive equal justifications collapse into one `m:mc` run: {xml}"
    );
}

#[test]
fn docx_export_stacks_single_cell_environments_as_equation_array() {
    let mut graph = DocumentGraph::new(DocumentId(67));
    graph.push_block(Block::Math {
        formula: document_ir::MathFormula {
            source: "\\begin{gather} a \\\\ b \\end{gather}".into(),
            language: Some("latex".into()),
            display: document_ir::MathDisplay::Display,
            expression: document_ir::MathExpression::Environment(document_ir::MathEnvironment {
                kind: document_ir::MathEnvironmentKind::Gather,
                name: "gather".into(),
                starred: false,
                rows: ["a", "b"]
                    .into_iter()
                    .map(|text| document_ir::MathRow {
                        cells: vec![math_atom(text)],
                        placement: document_ir::MathRowPlacement::Normal,
                        numbering: document_ir::MathRowNumbering::None,
                    })
                    .collect(),
                column_alignment: vec![document_ir::MathColumnAlignment::Center],
                column_pairs: None,
                span: None,
            }),
        },
    });
    let exported = export_docx_bytes(&graph).expect("export gather");
    let xml = String::from_utf8_lossy(&exported);
    assert!(xml.contains("<m:eqArr><m:e><m:r><m:t>a</m:t></m:r></m:e><m:e>"), "a row-per-equation environment is `m:eqArr`: {xml}");
    assert!(!xml.contains("<m:m>"), "a gather must not become a cell matrix: {xml}");
    let reopened = import_docx_bytes("gather-round.docx", &exported).expect("reopen gather");
    let source = reopened.blocks.iter().find_map(|node| match &node.block {
        Block::Math { formula } => Some(formula.source.clone()),
        _ => None,
    });
    assert!(source.as_deref().is_some_and(|source| source.contains("aligned")), "{source:?}");
}

#[test]
fn docx_export_declares_diagram_and_chart_languages_in_styles() {
    let mut graph = DocumentGraph::new(DocumentId(68));
    graph.push_block(Block::Code { language: Some("rust".into()), content: "fn main() {}".into() });
    graph.push_block(Block::Diagram { language: "mermaid".into(), source: "graph TD".into() });
    graph.push_block(Block::Chart { language: "vega-lite".into(), source: "{\"mark\": \"bar\"}".into() });
    let exported = export_docx_bytes(&graph).expect("export diagram and chart");
    let xml = String::from_utf8_lossy(&exported);
    // Every paragraph style the body references must exist in the styles part.
    assert!(xml.contains("w:pStyle w:val=\"Diagram:mermaid\""), "{xml}");
    assert!(xml.contains("w:pStyle w:val=\"Chart:vega-lite\""), "{xml}");
    assert!(xml.contains("w:styleId=\"Diagram:mermaid\""), "the diagram language must be declared as a named style: {xml}");
    assert!(xml.contains("w:styleId=\"Chart:vega-lite\""), "the chart language must be declared as a named style: {xml}");
    // `VerbatimChar` was declared but never referenced by code before.
    assert!(xml.contains("<w:rStyle w:val=\"VerbatimChar\"/>"), "code runs must reference the declared monospace style: {xml}");
    let reopened = import_docx_bytes("diagram-round.docx", &exported).expect("reopen diagram and chart");
    let rendered = format!("{:?}", reopened.blocks);
    assert!(rendered.contains("Diagram"), "{rendered}");
    // The reader maps only `Diagram:` today, so the chart source stays as text.
    assert!(rendered.contains("mark"), "chart source stays visible: {rendered}");
}

fn math_atom(text: &str) -> document_ir::MathExpression {
    document_ir::MathExpression::Atom(document_ir::MathAtom { text: text.into(), class: document_ir::MathClass::Identifier, span: None })
}

#[test]
fn docx_export_maps_semantic_math_faces_to_run_properties() {
    let mut graph = DocumentGraph::new(DocumentId(64));
    graph.push_block(Block::Math {
        formula: document_ir::MathFormula {
            source: "\\mathbf{x}+\\mathbb{R}+\\text{kg}".into(),
            language: Some("latex".into()),
            display: document_ir::MathDisplay::Display,
            expression: document_ir::MathExpression::Sequence(vec![
                document_ir::MathExpression::Styled { style: document_ir::MathStyle::Bold, body: Box::new(math_atom("x")) },
                document_ir::MathExpression::Styled { style: document_ir::MathStyle::DoubleStruck, body: Box::new(math_atom("R")) },
                document_ir::MathExpression::Text { text: "kg".into(), span: None },
            ]),
        },
    });
    let exported = export_docx_bytes(&graph).expect("export styled math");
    let xml = String::from_utf8_lossy(&exported);
    assert!(xml.contains(r#"<m:r><m:rPr><m:sty m:val="b"/></m:rPr><m:t>x</m:t></m:r>"#), "a bold face becomes `m:sty`: {xml}");
    assert!(
        xml.contains(r#"<m:r><m:rPr><m:scr m:val="double-struck"/></m:rPr><m:t>R</m:t></m:r>"#),
        "a double-struck face becomes `m:scr`: {xml}"
    );
    assert!(xml.contains(r#"<m:r><m:rPr><m:nor/></m:rPr><m:t>kg</m:t></m:r>"#), r"`\text{{...}}` stays upright prose: {xml}");
}

#[test]
fn docx_export_covers_math_expression_variants_with_omml() {
    let mut graph = DocumentGraph::new(DocumentId(65));
    graph.push_block(Block::Math {
        formula: document_ir::MathFormula {
            source: "variants".into(),
            language: Some("latex".into()),
            display: document_ir::MathDisplay::Display,
            expression: document_ir::MathExpression::Sequence(vec![
                document_ir::MathExpression::Radical { index: None, radicand: Box::new(math_atom("x")), span: None },
                document_ir::MathExpression::Radical {
                    index: Some(Box::new(document_ir::MathExpression::Number { text: "3".into(), span: None })),
                    radicand: Box::new(math_atom("y")),
                    span: None,
                },
                document_ir::MathExpression::Delimited {
                    open: Some(document_ir::MathDelimiter { kind: document_ir::MathDelimiterKind::Bracket, character: Some('['), span: None }),
                    close: Some(document_ir::MathDelimiter { kind: document_ir::MathDelimiterKind::Bracket, character: Some(']'), span: None }),
                    body: Box::new(math_atom("a")),
                    middles: vec![math_atom("b")],
                },
                document_ir::MathExpression::Accent { mark: '^', wide: false, base: Box::new(math_atom("c")), span: None },
                document_ir::MathExpression::StackAbove { over: Box::new(math_atom("1")), base: Box::new(math_atom("d")) },
                document_ir::MathExpression::StackBelow { under: Box::new(math_atom("2")), base: Box::new(math_atom("e")) },
                document_ir::MathExpression::Script {
                    base: Box::new(math_atom("f")),
                    superscript: None,
                    subscript: Some(Box::new(math_atom("g"))),
                },
                document_ir::MathExpression::Fraction {
                    numerator: Box::new(math_atom("h")),
                    denominator: Box::new(math_atom("i")),
                    line: document_ir::MathFractionLine::None,
                },
            ]),
        },
    });
    let exported = export_docx_bytes(&graph).expect("export math variants");
    let xml = String::from_utf8_lossy(&exported);
    assert!(xml.contains("<m:rad><m:radPr><m:degHide m:val=\"1\"/></m:radPr><m:deg/>"), "a root without an index hides the degree: {xml}");
    assert!(xml.contains("<m:rad><m:deg><m:r><m:t>3</m:t></m:r></m:deg>"), "a cube root keeps its index: {xml}");
    assert!(
        xml.contains("<m:d><m:dPr><m:begChr m:val=\"[\"/><m:endChr m:val=\"]\"/></m:dPr><m:e>"),
        "a delimited body keeps both brackets: {xml}"
    );
    assert!(xml.contains("<m:acc><m:accPr><m:chr m:val=\"^\"/></m:accPr>"), "an accent keeps its mark: {xml}");
    assert!(xml.contains("<m:limUpp>"), "stack above becomes `m:limUpp`: {xml}");
    assert!(xml.contains("<m:limLow>"), "stack below becomes `m:limLow`: {xml}");
    assert!(xml.contains("<m:sSub>"), "a subscript-only script becomes `m:sSub`: {xml}");
    assert!(xml.contains("<m:fPr><m:fType m:val=\"noBar\"/></m:fPr>"), "a barless fraction keeps `fType`: {xml}");
}

#[test]
fn docx_export_keeps_unsupported_math_fragments_as_visible_text() {
    let mut graph = DocumentGraph::new(DocumentId(62));
    graph.push_block(Block::Math {
        formula: document_ir::MathFormula {
            source: "\\foo{bar}".into(),
            language: Some("latex".into()),
            display: document_ir::MathDisplay::Display,
            expression: document_ir::MathExpression::Unsupported { kind: "unknown-command".into(), source: "\\foo{bar}".into(), span: None },
        },
    });
    let exported = export_docx_bytes(&graph).expect("export unsupported math");
    let xml = String::from_utf8_lossy(&exported);
    assert!(xml.contains("\\foo{bar}"), "the authored fragment stays readable: {xml}");
}

#[test]
fn docx_export_marks_images_without_bytes_instead_of_failing() {
    let mut graph = DocumentGraph::new(DocumentId(63));
    graph.push_asset(Asset {
        id: AssetId(2),
        kind: AssetKind::Image,
        content_identity: None,
        source: Some("media/missing.png".into()),
        media_type: None,
        status: SemanticStatus::Unresolved,
        bytes: None,
    });
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Image {
            source: "media/missing.png".into(),
            alt: vec![Inline::Text { text: "Logo".into() }],
            title: None,
            asset: Some(AssetId(2)),
        }],
    });
    let exported = export_docx_bytes(&graph).expect("an unmaterialized image must not fail the export");
    let xml = String::from_utf8_lossy(&exported);
    assert!(xml.contains("[image unavailable: Logo (media/missing.png)]"), "the gap stays visible: {xml}");
    assert!(!xml.contains("r:embed="), "no image part is attached without bytes");
}

#[test]
fn docx_export_keeps_opaque_and_raw_html_payloads_visible() {
    let mut graph = DocumentGraph::new(DocumentId(64));
    graph.push_block(Block::Opaque {
        kind: "front_matter".into(),
        payload_hint: "title: fidelity".into(),
        status: SemanticStatus::Unsupported,
    });
    graph.push_block(Block::RawHtml { html: "<div class=\"note\">raw block</div>".into() });
    graph.push_block(Block::Paragraph {
        content: vec![
            Inline::Reference { target: NodeId(1), display: "see target".into() },
            Inline::Text { text: " ".into() },
            Inline::Citation { key: "doe2020".into(), display: None },
            Inline::Text { text: " ".into() },
            Inline::Attributed {
                attributes: document_ir::Attributes { id: Some("caption".into()), classes: Vec::new(), pairs: Vec::new() },
                children: vec![Inline::Text { text: "caption".into() }],
            },
            Inline::Text { text: " ".into() },
            Inline::RawHtml { html: "<span>raw inline</span>".into() },
        ],
    });
    let exported = export_docx_bytes(&graph).expect("opaque and raw payloads must not fail the export");
    let xml = String::from_utf8_lossy(&exported);
    assert!(xml.contains("title: fidelity"), "front matter stays visible");
    assert!(xml.contains("raw block"), "raw block visible text stays visible: {xml}");
    assert!(!xml.contains("&lt;div"), "the dropped element structure must not leak as literal markup: {xml}");
    assert!(xml.contains("see target"), "reference display text stays visible");
    assert!(xml.contains("[doe2020]"), "a bare citation keeps its key visible");
    assert!(xml.contains("caption"), "attributed children stay visible");
    assert!(xml.contains("raw inline"), "raw inline visible text stays visible");
    assert!(!xml.contains("&lt;span"), "the dropped inline element must not leak as literal markup");
    let reopened = import_docx_bytes("degraded.docx", &exported).expect("reopen degraded docx");
    let reopened = format!("{:?}", reopened.blocks);
    assert!(reopened.contains("see target"), "{reopened}");
    assert!(reopened.contains("caption"), "{reopened}");
}

#[test]
fn docx_rejects_lists_deeper_than_numbering_definitions() {
    let mut graph = DocumentGraph::new(DocumentId(53));
    let mut children = Vec::new();
    for depth in 0..10 {
        let list = graph.push_block(Block::List {
            ordered: false,
            items: vec![ListItem {
                content: vec![Inline::Text { text: format!("depth {depth}") }],
                children,
                number: None,
                ..Default::default()
            }],
        });
        children = vec![list];
    }
    assert!(graph.validate().is_valid());
    assert!(export_docx_bytes(&graph).is_err());
}

#[test]
fn docx_export_keeps_caption_and_number_in_every_position() {
    let mut graph = DocumentGraph::new(DocumentId(55));
    let first_cell = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "first".into() }] });
    graph.push_block(Block::Table {
        caption: Some("首表".into()),
        head: vec![TableRow { cells: vec![TableCell { children: vec![first_cell], colspan: 1, rowspan: 1 }] }],
        body: Vec::new(),
        foot: Vec::new(),
    });
    let nested = graph.push_block(Block::List {
        ordered: true,
        items: vec![
            ListItem { content: vec![Inline::Text { text: "three".into() }], children: Vec::new(), number: Some(3), ..Default::default() },
            ListItem { content: vec![Inline::Text { text: "four".into() }], children: Vec::new(), number: Some(4), ..Default::default() },
        ],
    });
    graph.push_block(Block::List {
        ordered: true,
        items: vec![
            ListItem { content: vec![Inline::Text { text: "five".into() }], children: vec![nested], number: Some(5), ..Default::default() },
            ListItem { content: vec![Inline::Text { text: "seven".into() }], children: Vec::new(), number: Some(7), ..Default::default() },
        ],
    });
    let last_cell = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "last".into() }] });
    graph.push_block(Block::Table {
        caption: Some("尾表".into()),
        head: vec![TableRow { cells: vec![TableCell { children: vec![last_cell], colspan: 1, rowspan: 1 }] }],
        body: Vec::new(),
        foot: Vec::new(),
    });

    let exported = export_docx_bytes(&graph).expect("export structural blocks");
    let xml = String::from_utf8_lossy(&exported);
    assert!(xml.contains("<w:tblCaption w:val=\"首表\"/>"), "a caption on the first block must stay attached: {xml}");
    assert!(xml.contains("<w:tblCaption w:val=\"尾表\"/>"), "a caption on the last block must stay attached: {xml}");
    for start in ["3", "5", "7"] {
        assert!(
            xml.contains(&format!("<w:startOverride w:val=\"{start}\"/>")),
            "the authored start {start} must reach a numbering override: {xml}"
        );
    }
}

#[test]
fn docx_export_keeps_image_alt_as_alternate_text() {
    let mut graph = graph_with_image(png_header(1, 1), "media/alt.png");
    let Block::Paragraph { content } = &mut graph.blocks[0].block
    else {
        panic!("fixture graph must start with a paragraph")
    };
    let Inline::Image { alt, .. } = &mut content[0]
    else {
        panic!("fixture paragraph must hold the image")
    };
    *alt = vec![Inline::Text { text: "ZZ".into() }];

    let exported = export_docx_bytes(&graph).expect("export image alt");
    let xml = String::from_utf8_lossy(&exported);
    assert!(xml.contains(r#"descr="ZZ""#), "the alternate text must reach `wp:docPr`: {xml}");
}

#[test]
fn docx_export_keeps_table_caption_list_start_and_reference_target() {
    let mut graph = DocumentGraph::new(DocumentId(54));
    let target = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "target".into() }] });
    let heading = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "h".into() }] });
    graph.push_block(Block::Table {
        caption: Some("表 1".into()),
        head: vec![TableRow { cells: vec![TableCell { children: vec![heading], colspan: 1, rowspan: 1 }] }],
        body: Vec::new(),
        foot: Vec::new(),
    });
    graph.push_block(Block::List {
        ordered: true,
        items: vec![
            ListItem { content: vec![Inline::Text { text: "five".into() }], children: Vec::new(), number: Some(5), ..Default::default() },
            ListItem { content: vec![Inline::Text { text: "six".into() }], children: Vec::new(), number: Some(6), ..Default::default() },
        ],
    });
    graph.push_block(Block::Paragraph { content: vec![Inline::Reference { display: "go".into(), target }] });

    let exported = export_docx_bytes(&graph).expect("export structural blocks");
    let xml = String::from_utf8_lossy(&exported);
    assert!(xml.contains("表 1"), "the caption must stay visible: {xml}");
    assert!(xml.contains(r#"<w:tblCaption w:val="表 1"/>"#), "the caption must be attached to the table: {xml}");
    assert!(xml.contains(r#"<w:startOverride w:val="5"/>"#), "the ordered list must start at its authored number: {xml}");
    assert!(xml.contains(r#"<w:bookmarkStart w:id="1" w:name="node_1"/>"#), "the reference target needs a bookmark: {xml}");
    assert!(xml.contains(r#"<w:hyperlink w:anchor="node_1">"#), "the reference must hyperlink to its target: {xml}");
}

#[test]
fn docx_export_indents_plain_paragraph_children_of_list_items() {
    let mut graph = DocumentGraph::new(DocumentId(58));
    let child = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "Child block.".into() }] });
    graph.push_block(Block::List {
        ordered: false,
        items: vec![ListItem { content: vec![Inline::Text { text: "Parent item".into() }], children: vec![child], ..Default::default() }],
    });

    let exported = export_docx_bytes(&graph).expect("export list child DOCX");
    let payload = String::from_utf8_lossy(&exported);
    assert!(
        payload.contains(r#"<w:pPr><w:ind w:left="720"/></w:pPr>"#),
        "a plain child block must carry the item level indent instead of collapsing to the same level: {payload}"
    );
    assert!(payload.contains("Child block."), "the child block text must stay visible: {payload}");
}

#[test]
fn docx_export_does_not_indent_top_level_paragraphs() {
    let mut graph = DocumentGraph::new(DocumentId(59));
    graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "Top level.".into() }] });

    let exported = export_docx_bytes(&graph).expect("export top-level DOCX");
    let payload = String::from_utf8_lossy(&exported);
    assert!(
        !payload.contains(r#"<w:p><w:pPr><w:ind w:left="720"/></w:pPr>"#),
        "list-child indentation must not leak to top-level paragraphs: {payload}"
    );
}

#[test]
fn docx_export_writes_note_display_bodies_into_footnotes_xml() {
    let mut graph = DocumentGraph::new(DocumentId(56));
    graph.push_block(Block::Paragraph {
        content: vec![
            Inline::Text { text: "Body".into() },
            Inline::Note { label: "note".into(), display: Some(vec![Inline::Text { text: "ZZ_NOTE_BODY_ZZ".into() }]) },
        ],
    });

    let exported = export_docx_bytes(&graph).expect("export note DOCX");
    let payload = String::from_utf8_lossy(&exported);
    assert!(payload.contains("word/footnotes.xml"), "a note display body needs a footnotes part: {payload}");
    assert!(payload.contains("ZZ_NOTE_BODY_ZZ"), "the note display body must reach footnotes.xml: {payload}");
    assert!(payload.contains(r#"<w:footnoteReference w:id="1"/>"#), "the note must reference the part: {payload}");
}

#[test]
fn docx_export_of_a_note_without_display_keeps_the_token_and_no_footnotes_part() {
    let mut graph = DocumentGraph::new(DocumentId(57));
    graph.push_block(Block::Paragraph {
        content: vec![Inline::Text { text: "Body".into() }, Inline::Note { label: "note".into(), display: None }],
    });

    let exported = export_docx_bytes(&graph).expect("export note DOCX");
    let payload = String::from_utf8_lossy(&exported);
    assert!(!payload.contains("word/footnotes.xml"), "a note without a body must not invent a footnotes part: {payload}");
    assert!(payload.contains("[^note]"), "the authored token must stay visible: {payload}");
}
