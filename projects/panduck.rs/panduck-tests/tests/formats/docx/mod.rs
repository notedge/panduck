use document_formats::{
    export::{docx::export_docx_bytes, markdown::export_markdown},
    import::docx::import_docx_bytes,
};
use document_ir::{Block, BreakKind, Inline, SemanticStatus, SourceKind};

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

fn docx_zip(document_xml: &[u8]) -> Vec<u8> {
    stored_zip(&[("word/document.xml", document_xml)])
}

fn docx_zip_with_footnotes(document_xml: &[u8], footnotes_xml: &[u8]) -> Vec<u8> {
    stored_zip(&[("word/document.xml", document_xml), ("word/footnotes.xml", footnotes_xml)])
}

#[test]
fn docx_section_properties_become_page_setup() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:r><w:t>Body</w:t></w:r></w:p>
    <w:sectPr>
      <w:pgSz w:w="11906" w:h="16838"/>
      <w:pgMar w:top="680" w:right="666" w:bottom="680" w:left="666"/>
      <w:cols w:num="2"/>
    </w:sectPr>
  </w:body>
</w:document>"#;
    let styles_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:docDefaults>
    <w:rPrDefault><w:rPr><w:sz w:val="25"/></w:rPr></w:rPrDefault>
    <w:pPrDefault><w:pPr><w:spacing w:line="302"/></w:pPr></w:pPrDefault>
  </w:docDefaults>
</w:styles>"#;
    let zip = stored_zip(&[("word/document.xml", document_xml), ("word/styles.xml", styles_xml)]);
    let graph = import_docx_bytes("page-setup.docx", &zip).expect("import DOCX");
    let page = graph.page_setup.expect("page setup");
    assert_eq!(page.page_width, Some(595.3), "twips become points");
    assert_eq!(page.page_height, Some(841.9));
    assert_eq!(page.margin_left, Some(33.3));
    assert_eq!(page.columns, Some(2));
    assert_eq!(page.base_font_size, Some(12.5), "half-points become points");
    assert_eq!(page.line_height, Some(302.0 / 240.0));
    assert!(
        !graph.coverage.loss.iter().any(|loss| loss.code == "reader.docx.section_layout"),
        "a represented section must not be reported as a loss"
    );
}

#[test]
fn docx_quote_round_trips_quoted_text() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:pPr><w:pStyle w:val="Quote"/></w:pPr><w:r><w:t>Quoted line</w:t></w:r></w:p>
  </w:body>
</w:document>"#;
    let graph = import_docx_bytes("quote.docx", &docx_zip(document_xml)).expect("import DOCX quote");
    let children = graph
        .blocks
        .iter()
        .find_map(|node| match &node.block {
            Block::Quote { children } => Some(children.as_slice()),
            _ => None,
        })
        .expect("quote block expected");
    assert!(children.iter().any(|child_id| matches!(
        graph.block(*child_id).map(|node| &node.block),
        Some(Block::Paragraph { content })
            if content.iter().any(|inline| matches!(inline, Inline::Text { text } if text.contains("Quoted line")))
    )));
    let markdown = export_markdown(&graph).expect("export quoted markdown");
    assert!(markdown.contains("> Quoted line"), "{markdown}");

    let bytes = export_docx_bytes(&graph).expect("export DOCX quote");
    let reopened = import_docx_bytes("quote-roundtrip.docx", &bytes).expect("reopen DOCX quote");
    let markdown = export_markdown(&reopened).expect("export reopened markdown");
    assert!(markdown.contains("> Quoted line"), "{markdown}");
}

#[test]
fn docx_to_markdown_round_trip() {
    let zip = minimal_docx_zip();
    let graph = import_docx_bytes("sample.docx", &zip).expect("import docx");
    let markdown = export_markdown(&graph).expect("export markdown");
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
    let zip = docx_zip(document_xml);
    let graph = import_docx_bytes("styled.docx", &zip).expect("import docx");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("**Bold**"));
    assert!(markdown.contains("* italic*"));
}

#[test]
fn docx_imports_underline_and_strike_without_losing_markdown_strike() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p>
    <w:r><w:rPr><w:u/></w:rPr><w:t>Underlined</w:t></w:r>
    <w:r><w:rPr><w:u w:val="none"/></w:rPr><w:t> plain</w:t></w:r>
    <w:r><w:rPr><w:strike/></w:rPr><w:t> crossed</w:t></w:r>
    <w:r><w:rPr><w:b/><w:i/><w:u w:val="double"/><w:strike/></w:rPr><w:t> combined</w:t></w:r>
  </w:p></w:body>
</w:document>"#;
    let graph = import_docx_bytes("character-styles.docx", &docx_zip(document_xml)).expect("import DOCX");
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("paragraph expected")
    };
    assert!(matches!(&content[0], Inline::Styled { style, .. } if style == "underline"));
    assert!(matches!(&content[1], Inline::Text { text } if text == " plain"));
    assert!(matches!(&content[2], Inline::Styled { style, .. } if style == "strike"));
    assert!(
        matches!(&content[3], Inline::Styled { style, children } if style == "bold" && matches!(&children[0], Inline::Styled { style, children } if style == "italic" && matches!(&children[0], Inline::Styled { style, .. } if style == "underline")))
    );
    assert!(export_markdown(&graph).is_err(), "Markdown must not silently drop underline");
    let strike_only = document_ir::Inline::Styled { style: "strike".into(), children: vec![Inline::Text { text: "crossed".into() }] };
    let mut strike_graph = document_ir::DocumentGraph::new(document_ir::DocumentId(55));
    strike_graph.push_block(Block::Paragraph { content: vec![strike_only] });
    assert!(export_markdown(&strike_graph).expect("Markdown strike").contains("~~crossed~~"));
}

#[test]
fn docx_import_reports_field_semantics_that_are_not_in_ir() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:r><w:fldChar w:fldCharType="begin"/></w:r><w:r><w:instrText> PAGE </w:instrText></w:r><w:r><w:fldChar w:fldCharType="end"/></w:r><w:r><w:t>visible fallback</w:t></w:r></w:p></w:body>
</w:document>"#;
    let graph = import_docx_bytes("field.docx", &docx_zip(document_xml)).expect("import field DOCX");
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "reader.docx.fields"));
    assert!(format!("{:?}", graph.blocks).contains("visible fallback"));
}

#[test]
fn docx_import_reads_citation_keys_from_simple_fields() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:fldSimple w:instr=" CITATION smith2024 "><w:r><w:t>(Smith 2024)</w:t></w:r></w:fldSimple></w:p></w:body>
</w:document>"#;
    let graph = import_docx_bytes("citation.docx", &docx_zip(document_xml)).expect("import citation DOCX");
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("paragraph expected")
    };
    assert!(
        matches!(&content[0], Inline::Citation { key, display: Some(display) }
            if key == "smith2024"
                && matches!(&display[0], Inline::Text { text } if text == "(Smith 2024)")),
        "{content:?}"
    );
    assert!(
        !graph.coverage.loss.iter().any(|loss| loss.code == "reader.docx.fields"),
        "a mapped citation field must not be reported as an unmapped field"
    );
}

#[test]
fn docx_import_keeps_unmapped_simple_field_text_and_reports_fields() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:fldSimple w:instr=" PAGE "><w:r><w:t>7</w:t></w:r></w:fldSimple></w:p></w:body>
</w:document>"#;
    let graph = import_docx_bytes("page-field.docx", &docx_zip(document_xml)).expect("import page field DOCX");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains('7'), "{markdown}");
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "reader.docx.fields"));
    assert!(!format!("{:?}", graph.blocks).contains("Citation"), "a PAGE field must not become a citation");
}

#[test]
fn docx_import_reads_hyperlink_tooltips_and_image_titles() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
            xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
            xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
            xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing">
  <w:body>
    <w:p>
      <w:hyperlink r:id="rId1" w:tooltip="Example tooltip"><w:r><w:t>Example</w:t></w:r></w:hyperlink>
    </w:p>
    <w:p>
      <w:r>
        <w:drawing>
          <wp:inline>
            <wp:docPr descr="Logo" title="Logo title"/>
            <a:graphic><a:graphicData><a:blip r:embed="rId2"/></a:graphicData></a:graphic>
          </wp:inline>
        </w:drawing>
      </w:r>
    </w:p>
  </w:body>
</w:document>"#;
    let rels_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1"
    Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink"
    Target="https://example.com" TargetMode="External"/>
  <Relationship Id="rId2"
    Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
    Target="media/logo.png"/>
</Relationships>"#;
    let zip =
        stored_zip(&[("word/document.xml", document_xml), ("word/_rels/document.xml.rels", rels_xml), ("media/logo.png", b"\x89PNG\r\n")]);
    let graph = import_docx_bytes("titles.docx", &zip).expect("import titles DOCX");

    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("link paragraph expected")
    };
    assert!(matches!(&content[0], Inline::Link { title: Some(title), .. } if title == "Example tooltip"), "{content:?}");
    let Block::Paragraph { content } = &graph.blocks[1].block
    else {
        panic!("image paragraph expected")
    };
    assert!(matches!(&content[0], Inline::Image { title: Some(title), .. } if title == "Logo title"), "{content:?}");
}

#[test]
fn docx_import_reports_unmapped_embedded_objects() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <w:body><w:p><w:r><w:object><w:oleObject r:id="rId1"/></w:object><w:pict/><w:t>visible fallback</w:t></w:r></w:p></w:body>
</w:document>"#;
    let graph = import_docx_bytes("embedded-object.docx", &docx_zip(document_xml)).expect("import embedded object DOCX");
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "reader.docx.embedded_objects"));
    assert!(format!("{:?}", graph.blocks).contains("visible fallback"));
}

#[test]
fn docx_imports_superscript_and_subscript_runs() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:r><w:rPr><w:vertAlign w:val="superscript"/></w:rPr><w:t>2</w:t></w:r><w:r><w:rPr><w:vertAlign w:val="subscript"/></w:rPr><w:t>i</w:t></w:r></w:p></w:body>
</w:document>"#;
    let graph = import_docx_bytes("vertical-align.docx", &docx_zip(document_xml)).expect("import vertical alignment");
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("paragraph expected")
    };
    assert!(matches!(&content[0], Inline::Styled { style, .. } if style == "superscript"));
    assert!(matches!(&content[1], Inline::Styled { style, .. } if style == "subscript"));
}

#[test]
fn docx_import_lowers_line_breaks_to_a_first_class_inline() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:r><w:t>first</w:t><w:br/><w:t>second</w:t></w:r></w:p></w:body>
</w:document>"#;
    let graph = import_docx_bytes("break.docx", &docx_zip(document_xml)).expect("import break DOCX");
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("paragraph expected")
    };
    assert!(matches!(&content[0], Inline::Text { text } if text == "first"), "{content:?}");
    assert!(matches!(&content[1], Inline::Break { kind: BreakKind::Hard }), "{content:?}");
    assert!(matches!(&content[2], Inline::Text { text } if text == "second"), "{content:?}");
}

#[test]
fn docx_import_reports_unmapped_character_properties() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:r><w:rPr><w:color w:val="FF0000"/><w:sz w:val="28"/><w:highlight w:val="yellow"/></w:rPr><w:t>colored text</w:t></w:r></p></w:body>
</w:document>"#;
    let mut document_xml = document_xml.to_vec();
    let marker = b"</w:body>";
    let insertion = document_xml.windows(marker.len()).position(|window| window == marker).expect("body marker");
    document_xml
        .splice(insertion..insertion, br#"<w:p><w:r><w:rPr><w:spacing w:val="20"/></w:rPr><w:t>unsupported</w:t></w:r></w:p>"#.iter().copied());
    let graph = import_docx_bytes("character-properties.docx", &docx_zip(&document_xml)).expect("import character properties");
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "reader.docx.character_properties"));
    assert!(format!("{:?}", graph.blocks).contains("colored text"));
}

#[test]
fn docx_imports_color_as_a_value_carrying_style() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:r><w:rPr><w:color w:val="FF0000"/></w:rPr><w:t>red</w:t></w:r></w:p></w:body>
</w:document>"#;
    let graph = import_docx_bytes("color.docx", &docx_zip(document_xml)).expect("import color");
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("paragraph expected")
    };
    assert!(matches!(&content[0], Inline::Styled { style, .. } if style == "color:ff0000"));
    assert!(!graph.coverage.loss.iter().any(|loss| loss.code == "reader.docx.character_properties"));
}

#[test]
fn docx_imports_font_size_family_and_highlight_styles() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:r><w:rPr><w:sz w:val="28"/><w:rFonts w:ascii="Aptos"/><w:highlight w:val="yellow"/></w:rPr><w:t>styled</w:t></w:r></w:p></w:body>
</w:document>"#;
    let graph = import_docx_bytes("valued-character-styles.docx", &docx_zip(document_xml)).expect("import valued styles");
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("paragraph expected")
    };
    let rendered = format!("{:?}", content);
    assert!(rendered.contains("font-size:28"), "{rendered}");
    assert!(rendered.contains("font-family:Aptos"), "{rendered}");
    assert!(rendered.contains("highlight:yellow"), "{rendered}");
    assert!(!graph.coverage.loss.iter().any(|loss| loss.code == "reader.docx.character_properties"));
}

#[test]
fn docx_imports_hyperlinks_from_relationships() {
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
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("[Example](https://example.com)"));
}

#[test]
fn docx_imports_embedded_images() {
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
        stored_zip(&[("word/document.xml", document_xml), ("word/_rels/document.xml.rels", rels_xml), ("media/logo.png", b"\x89PNG\r\n")]);
    let graph = import_docx_bytes("image.docx", &zip).expect("import docx");
    assert_eq!(graph.assets.len(), 1);
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("![Logo](media/logo.png)"));
}

const DECIMAL_NUMBERING_XML: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="0">
    <w:lvl w:ilvl="0">
      <w:numFmt w:val="decimal"/>
    </w:lvl>
  </w:abstractNum>
  <w:num w:numId="1">
    <w:abstractNumId w:val="0"/>
  </w:num>
</w:numbering>"#;

#[test]
fn docx_imports_ordered_lists_from_numbering_xml() {
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
    let zip = stored_zip(&[("word/document.xml", document_xml), ("word/numbering.xml", DECIMAL_NUMBERING_XML)]);
    let graph = import_docx_bytes("ol.docx", &zip).expect("import docx");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("1. One"));
    assert!(markdown.contains("2. Two"));
    assert!(!graph.coverage.loss.iter().any(|loss| loss.code == "reader.docx.numbering_unresolved"));
}

const ROMAN_NUMBERING_XML: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="0">
    <w:lvl w:ilvl="0">
      <w:numFmt w:val="lowerRoman"/>
    </w:lvl>
  </w:abstractNum>
  <w:num w:numId="2">
    <w:abstractNumId w:val="0"/>
  </w:num>
</w:numbering>"#;

#[test]
fn docx_import_reports_numbering_formats_that_the_ir_cannot_carry() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="2"/></w:numPr></w:pPr>
      <w:r><w:t>One</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="2"/></w:numPr></w:pPr>
      <w:r><w:t>Two</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
    let zip = stored_zip(&[("word/document.xml", document_xml), ("word/numbering.xml", ROMAN_NUMBERING_XML)]);
    let graph = import_docx_bytes("roman.docx", &zip).expect("import docx");
    let loss = graph.coverage.loss.iter().find(|loss| loss.code == "reader.docx.numbering_format").expect("numbering format loss");
    assert_eq!(loss.status, SemanticStatus::Partial);
    assert!(loss.message.contains("lowerRoman"), "{}", loss.message);
    // The list stays an ordered list with sequential numbers, so only the
    // marker glyph is lost.
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("1. One"), "{markdown}");
    assert!(markdown.contains("2. Two"), "{markdown}");
    assert!(!graph.coverage.loss.iter().any(|loss| loss.code == "reader.docx.numbering_unresolved"));
}

#[test]
fn docx_resolves_numbering_through_the_document_relationship() {
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
    let rels_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId7"
    Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/numbering"
    Target="listed/numbering.xml"/>
</Relationships>"#;
    // The numbering part is not at the conventional path, so only the
    // relationship can point the importer at it.
    let zip = stored_zip(&[
        ("word/document.xml", document_xml),
        ("word/_rels/document.xml.rels", rels_xml),
        ("word/listed/numbering.xml", DECIMAL_NUMBERING_XML),
    ]);
    let graph = import_docx_bytes("related-numbering.docx", &zip).expect("import docx");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("1. One"), "{markdown:?}");
    assert!(markdown.contains("2. Two"), "{markdown:?}");
    assert!(!graph.coverage.loss.iter().any(|loss| loss.code == "reader.docx.numbering_unresolved"));
}

#[test]
fn docx_imports_nested_ordered_lists_with_sequential_numbers() {
    const NESTED_NUMBERING_XML: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:abstractNum w:abstractNumId="0">
    <w:lvl w:ilvl="0">
      <w:numFmt w:val="decimal"/>
    </w:lvl>
    <w:lvl w:ilvl="1">
      <w:numFmt w:val="decimal"/>
    </w:lvl>
  </w:abstractNum>
  <w:num w:numId="1">
    <w:abstractNumId w:val="0"/>
  </w:num>
</w:numbering>"#;
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p>
      <w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr>
      <w:r><w:t>Parent</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:ilvl w:val="1"/><w:numId w:val="1"/></w:numPr></w:pPr>
      <w:r><w:t>Child one</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:ilvl w:val="1"/><w:numId w:val="1"/></w:numPr></w:pPr>
      <w:r><w:t>Child two</w:t></w:r>
    </w:p>
    <w:p>
      <w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr>
      <w:r><w:t>Second</w:t></w:r>
    </w:p>
  </w:body>
</w:document>"#;
    let zip = stored_zip(&[("word/document.xml", document_xml), ("word/numbering.xml", NESTED_NUMBERING_XML)]);
    let graph = import_docx_bytes("nested-ol.docx", &zip).expect("import docx");
    assert!(graph.validate().is_valid(), "{:#?}", graph.validate().issues);
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("1. Parent"), "{markdown:?}");
    assert!(markdown.contains("  1. Child one"), "{markdown:?}");
    assert!(markdown.contains("  2. Child two"), "{markdown:?}");
    assert!(markdown.contains("2. Second"), "{markdown:?}");
}

#[test]
fn docx_imports_numbered_paragraphs_as_list() {
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
    let zip = docx_zip(document_xml);
    let graph = import_docx_bytes("list.docx", &zip).expect("import docx");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("- One"));
    assert!(markdown.contains("- Two"));
}

#[test]
fn docx_imports_unresolved_footnote_references_with_loss() {
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
    let zip = docx_zip(document_xml);
    let graph = import_docx_bytes("sample.docx", &zip).expect("import docx");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("See[^1] for details."));
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "reader.docx.footnote_body"));
}

#[test]
fn docx_imports_footnote_bodies_from_footnotes_xml() {
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
    let zip = docx_zip_with_footnotes(document_xml, footnotes_xml);
    let graph = import_docx_bytes("sample.docx", &zip).expect("import docx");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("See[^1] for details."));
    assert!(markdown.contains("[^1]: Footnote body."));
    assert!(!graph.coverage.loss.iter().any(|loss| loss.code == "reader.docx.footnote_body"));
}

#[test]
fn docx_imports_tables_as_gfm_markdown() {
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
    let zip = docx_zip(document_xml);
    let graph = import_docx_bytes("table.docx", &zip).expect("import docx");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("| H1 | H2 |"));
    assert!(markdown.contains("| --- | --- |"));
    assert!(markdown.contains("| A | B |"));
}

#[test]
fn docx_projects_inserted_text_and_reports_deleted_text() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:r><w:t>Keep </w:t></w:r><w:ins w:id="1"><w:r><w:t>inserted</w:t></w:r></w:ins><w:del w:id="2"><w:r><w:delText>deleted</w:delText></w:r></w:del></w:p></w:body>
</w:document>"#;
    let graph = import_docx_bytes("tracked.docx", &docx_zip(document_xml)).expect("import docx");
    let markdown = export_markdown(&graph).expect("export markdown");
    assert!(markdown.contains("Keep inserted"), "markdown was: {markdown}");
    assert!(!markdown.contains("deleted"));
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "reader.docx.tracked_changes"));
}

#[test]
fn docx_import_reports_page_and_column_break_loss() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:r><w:t>Before</w:t><w:br w:type="page"/><w:t>After</w:t><w:br w:type="column"/></w:r></w:p></w:body>
</w:document>"#;
    let graph = import_docx_bytes("page-breaks.docx", &docx_zip(document_xml)).expect("import page breaks");
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "reader.docx.page_breaks"));
    let markdown = export_markdown(&graph).expect("export page breaks");
    assert!(markdown.contains("Before  \nAfter"), "markdown was: {markdown}");
}

#[test]
fn docx_import_reports_paragraph_and_section_layout_breaks() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
    <w:p><w:pPr><w:pageBreakBefore/></w:pPr><w:r><w:t>Before</w:t><w:lastRenderedPageBreak/><w:t>After</w:t></w:r></w:p>
    <w:sectPr><w:pgSz w:w="11906" w:h="16838"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440"/><w:cols w:num="2"/><w:docGrid w:linePitch="312"/></w:sectPr>
  </w:body>
</w:document>"#;
    let graph = import_docx_bytes("layout-breaks.docx", &docx_zip(document_xml)).expect("import layout breaks");
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "reader.docx.page_breaks"));
    assert!(graph.coverage.loss.iter().any(|loss| loss.code == "reader.docx.section_layout"));
}
mod export;
mod import_xml_extensions;
mod inspect;
mod query_gate;

#[test]
fn docx_import_records_body_xml_part_provenance() {
    let graph = import_docx_bytes("provenance.docx", &minimal_docx_zip()).expect("import docx");
    assert!(!graph.blocks.is_empty());

    for node in &graph.blocks {
        let sources = graph.sources_for(node.id);
        assert_eq!(sources.len(), 1, "each body block records exactly one XML part source");
        assert!(
            matches!(&sources[0].kind, SourceKind::XmlRange { path } if path == "word/document.xml"),
            "unexpected source: {:?}",
            sources[0].kind
        );
        assert_eq!(sources[0].precision, SemanticStatus::Partial);
    }
    assert!(graph.validate().is_valid());
}

#[test]
fn docx_import_records_footnotes_part_provenance() {
    let document_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body><w:p><w:r><w:footnoteReference w:id="1"/></w:r></w:p></w:body>
</w:document>"#;
    let footnotes_xml = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:footnotes xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:footnote w:id="1"><w:p><w:r><w:t>Footnote body.</w:t></w:r></w:p></w:footnote>
</w:footnotes>"#;
    let graph = import_docx_bytes("footnotes.docx", &docx_zip_with_footnotes(document_xml, footnotes_xml)).expect("import docx");

    let definition = graph
        .blocks
        .iter()
        .find(|node| matches!(&node.block, Block::Opaque { kind, .. } if kind == "footnote_definition"))
        .expect("footnote definition block");
    let sources = graph.sources_for(definition.id);
    assert_eq!(sources.len(), 1);
    assert!(
        matches!(&sources[0].kind, SourceKind::XmlRange { path } if path == "word/footnotes.xml"),
        "unexpected source: {:?}",
        sources[0].kind
    );
    assert_eq!(sources[0].precision, SemanticStatus::Partial);
}
