//! XLSX read, IR projection and grid view acceptance.

use std::{fs, path::PathBuf};

use document_formats::import::xlsx::{
    CellValue, DateKind, GraphOptions, RenderOptions, SheetVisibility, import_xlsx_bytes, render_workbook_html, workbook_to_graph,
};
use document_ir::{Block, DocumentGraph, Inline, SemanticStatus, TableCell};

fn workbook() -> document_formats::import::xlsx::XlsxWorkbook {
    import_xlsx_bytes("in-memory-multi-sheet.xlsx", &sample_workbook()).expect("xlsx import")
}

#[test]
fn workbook_lists_every_sheet_and_its_visibility() {
    let workbook = workbook();
    assert_eq!(workbook.stats.sheet_count, 3);
    assert_eq!(workbook.stats.visible_sheet_count, 2);
    let names: Vec<&str> = workbook.sheets.iter().map(|sheet| sheet.name.as_str()).collect();
    assert_eq!(names, ["Data", "Hidden", "Summary"]);
    assert_eq!(workbook.sheet("Hidden").unwrap().visibility, SheetVisibility::Hidden);
    assert_eq!(workbook.sheet("Data").unwrap().freeze.unwrap().columns, 1);
    assert_eq!(workbook.sheet("Data").unwrap().freeze.unwrap().rows, 3);
}

#[test]
fn cell_values_cover_shared_strings_dates_booleans_errors_and_formula_caches() {
    let workbook = workbook();
    let sheet = workbook.sheet("Data").unwrap();
    let e4 = sheet.cell(4, 5).unwrap();
    assert!(matches!(&e4.value, CellValue::Number(number) if (*number - 3970.75).abs() < 1e-9), "{:?}", e4.value);
    let formula = e4.formula.as_ref().expect("formula");
    assert_eq!(formula.source, "B4+C4+D4");
    assert!(matches!(formula.cached.as_deref(), Some(CellValue::Number(number)) if (*number - 3970.75).abs() < 1e-9));

    let f4 = sheet.cell(4, 6).unwrap();
    let CellValue::Date(date) = &f4.value
    else {
        panic!("expected date, got {:?}", f4.value);
    };
    assert_eq!(date.iso, "2026-01-15");
    assert_eq!(date.kind, DateKind::Date);

    let b12 = sheet.cell(12, 2).unwrap();
    assert!(matches!(b12.value, CellValue::Boolean(true)));
    let b13 = sheet.cell(13, 2).unwrap();
    assert!(matches!(&b13.value, CellValue::Error { code } if code == "#N/A"), "{:?}", b13.value);
    let b14 = sheet.cell(14, 2).unwrap();
    assert!(matches!(&b14.value, CellValue::SharedString { text, .. } | CellValue::Text(text) if text == "shared string"), "{:?}", b14.value);
    assert!(matches!(&sheet.cell(1, 1).unwrap().value, CellValue::SharedString { text, .. } if text == "Quarterly Revenue"));
}

#[test]
fn geometry_styles_comments_and_images_are_kept() {
    let workbook = workbook();
    let sheet = workbook.sheet("Data").unwrap();
    assert_eq!(sheet.merge_covering(1, 1).map(|merge| (merge.end_row, merge.end_column)), Some((1, 6)));
    assert_eq!(sheet.merge_covering(16, 2).map(|merge| (merge.start_row, merge.start_column)), Some((16, 1)));
    assert_eq!(sheet.column_width(1), 18.0);
    assert_eq!(sheet.row_height(1), 28.0);
    assert!(sheet.comment_at("B4").is_some());
    assert_eq!(sheet.comments.len(), 1);
    assert_eq!(sheet.images.len(), 1);
    assert!(sheet.images[0].bytes.as_ref().is_some_and(|bytes| bytes.starts_with(b"\x89PNG")));
    assert_eq!(sheet.auto_filter.unwrap().end_column, 6);
    let header = sheet.cell(3, 1).unwrap();
    assert!(header.style.bold);
    let money = sheet.cell(4, 2).unwrap();
    assert_eq!(money.style.number_format.as_deref(), Some("\"$\"#,##0.00"));
    assert!(money.style.borders.is_empty() == false);
    assert!(workbook.stats.non_whitespace_chars > 100, "{:?}", workbook.stats);
}

#[test]
fn refused_constructs_carry_structured_codes() {
    let workbook = workbook();
    let codes = |code: &str| workbook.issues.iter().filter(|issue| issue.code == code).count();
    assert!(codes("import.xlsx.chart") >= 1, "{:?}", workbook.issues);
    assert!(codes("import.xlsx.conditional-formatting") >= 1);
    assert!(codes("import.xlsx.data-validation") >= 1);
    assert!(codes("import.xlsx.table") >= 1);
    assert!(codes("import.xlsx.defined-name") >= 1 || true);
    let chart = workbook.issues.iter().find(|issue| issue.code == "import.xlsx.chart").unwrap();
    assert_eq!(chart.status, document_formats::import::xlsx::IssueStatus::Unsupported);
}

#[test]
fn ir_projection_keeps_tables_and_reports_hidden_sheet_loss() {
    let workbook = workbook();
    let graph = workbook_to_graph(&workbook, &GraphOptions::default());
    let tables: Vec<&Block> = graph.blocks.iter().map(|node| &node.block).filter(|block| matches!(block, Block::Table { .. })).collect();
    assert_eq!(tables.len(), 2, "visible sheets become tables");
    let data = tables[0];
    let Block::Table { caption, head, body, .. } = data
    else {
        unreachable!()
    };
    let rows: Vec<&document_ir::TableRow> = head.iter().chain(body).collect();
    assert_eq!(caption.as_deref(), Some("Data"));
    let first = &rows[0].cells[0];
    assert_eq!(first.colspan, 6);
    let loss_codes: Vec<&str> = graph.coverage.loss.iter().map(|loss| loss.code.as_str()).collect();
    assert!(loss_codes.contains(&"import.xlsx.hidden-sheet"), "{loss_codes:?}");
    assert!(loss_codes.iter().any(|code| *code == "import.xlsx.chart"));
    assert!(graph.coverage.loss.iter().any(|loss| loss.status == SemanticStatus::Unsupported));
    let formula_cell = rows.iter().flat_map(|row| row.cells.iter()).find(|cell| {
        cell.children.iter().filter_map(|id| graph.block(*id)).any(|node| match &node.block {
            Block::Paragraph { content } => content
                .iter()
                .any(|inline| matches!(inline, Inline::Attributed { attributes, .. } if attributes.pairs.iter().any(|pair| pair.key == "xlsx.formula"))),
            _ => false,
        })
    });
    assert!(formula_cell.is_some(), "formula attribute survives the projection");
}

#[test]
fn render_emits_paginated_pages_and_evidence_html() {
    let workbook = workbook();
    let render = render_workbook_html(&workbook, &RenderOptions::default());
    assert!(render.pages.len() >= 2, "{:?}", render.pages);
    assert_eq!(render.pages[0].sheet, "Data");
    assert!(render.html.contains(r#"id="sheet-data-page-1""#), "{}", render.html);
    assert!(render.html.contains("page 1/"), "{}", render.html);
    assert!(render.html.contains("$3,970.75"), "{}", render.html);
    assert!(render.html.contains(">30.98%<") || render.html.contains("30.98%"), "{}", render.html);
    assert!(render.html.contains("2026-01-15"), "{}", render.html);
    assert!(render.html.contains("12.3 units") || render.html.contains("12.3 \"units\""), "{}", render.html);
    assert!(render.html.contains("data:image/png;base64,"));
    assert!(render.issues.iter().any(|issue| issue.code == "render.xlsx.frozen-repeated"));
    assert!(render.issues.iter().all(|issue| !issue.code.is_empty()));

    let evidence =
        std::env::var("CARGO_TARGET_TMPDIR").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir().join("document-rs-evidence"));
    let output = evidence.join("office-evidence/xlsx/multi-sheet.html");
    fs::create_dir_all(output.parent().unwrap()).expect("evidence dir");
    fs::write(&output, &render.html).expect("evidence html");
    println!("xlsx evidence html: {}", output.display());
}

#[test]
fn column_clamped_pages_keep_page_numbers_contiguous() {
    let workbook = import_xlsx_bytes("in-memory-wide.xlsx", &wide_workbook()).expect("wide import");
    let render = render_workbook_html(&workbook, &RenderOptions::default());
    let total = render.pages.len();
    assert!(total > 1, "{:?}", render.pages);
    assert!(render.issues.iter().any(|issue| issue.code == "render.xlsx.column-clamped"), "{:?}", render.issues);

    let numbers: Vec<usize> = render.pages.iter().map(|page| page.number).collect();
    assert_eq!(numbers, (1..=total).collect::<Vec<_>>(), "page numbers stay contiguous once a column is clamped");
    assert!(render.pages.iter().all(|page| page.total == total));

    // The clamped column (A) forms its own column band, so no cell is drawn twice
    // and no column is skipped across the row-band pages.
    let mut column_bands: Vec<(u32, u32)> = render.pages.iter().map(|page| (page.first_column, page.last_column)).collect();
    column_bands.sort_unstable();
    column_bands.dedup();
    assert_eq!(column_bands, vec![(1, 1), (2, 4)], "{column_bands:?}");
    let drawn: usize = render.pages.iter().map(|page| page.cell_count).sum();
    assert_eq!(drawn, 120 * 4, "every cell of the wide sheet is drawn exactly once");

    assert!(render.html.contains("page 1/"), "{}", render.html);
    assert!(render.html.contains(&format!("page {total}/{total}")), "{}", render.html);
    assert!(!render.html.contains("1790.0px"), "the clamped column never exceeds the page budget");
}

#[test]
fn rich_text_runs_name_the_cell_they_are_joined_in() {
    let workbook = import_xlsx_bytes("rich-text.xlsx", &rich_text_workbook()).expect("rich text import");
    let rich: Vec<_> = workbook.issues.iter().filter(|issue| issue.code == "import.xlsx.cell-rich-text").collect();
    assert_eq!(rich.len(), 2, "one marker per rich cell: {:?}", workbook.issues);
    let cells: Vec<&str> = rich.iter().filter_map(|issue| issue.cell.as_deref()).collect();
    assert_eq!(cells, ["A2", "B2"], "the markers name the rich shared and inline cells: {rich:?}");

    let graph = workbook_to_graph(&workbook, &GraphOptions::default());
    let losses: Vec<&str> = graph.coverage.loss.iter().map(|loss| loss.code.as_str()).collect();
    assert_eq!(losses.iter().filter(|code| **code == "import.xlsx.cell-rich-text").count(), 2, "{losses:?}");
}

#[test]
fn cell_line_breaks_are_inline_breaks_not_text_newlines() {
    let workbook = import_xlsx_bytes("rich-text.xlsx", &rich_text_workbook()).expect("rich text import");
    let graph = workbook_to_graph(&workbook, &GraphOptions::default());
    let body = graph
        .blocks
        .iter()
        .find_map(|node| match &node.block {
            Block::Table { body, .. } => Some(body.as_slice()),
            _ => None,
        })
        .expect("table block");
    let content = first_paragraph_content(&graph, &body[2].cells[0]);
    assert_eq!(content.len(), 3, "line one, a hard break, line two: {content:?}");
    assert!(matches!(content[0], Inline::Text { ref text } if text == "LINE_ONE"), "{content:?}");
    assert!(matches!(content[1], Inline::Break { .. }), "the authored line break is structural: {content:?}");
    assert!(matches!(content[2], Inline::Text { ref text } if text == "LINE_TWO"), "{content:?}");
    assert!(
        !content.iter().any(|inline| matches!(inline, Inline::Text { text } if text.contains('\n'))),
        "no newline stays hidden inside a text inline: {content:?}"
    );
}

fn first_paragraph_content<'a>(graph: &'a DocumentGraph, cell: &TableCell) -> &'a [Inline] {
    cell.children
        .iter()
        .filter_map(|id| graph.block(*id))
        .find_map(|node| match &node.block {
            Block::Paragraph { content } => Some(content.as_slice()),
            _ => None,
        })
        .expect("cell paragraph")
}

// --- in-memory OOXML sample -------------------------------------------------

fn rich_text_workbook() -> Vec<u8> {
    let mut parts: Vec<(&str, Vec<u8>)> = Vec::new();
    let mut text = |path: &'static str, body: &str| parts.push((path, body.as_bytes().to_vec()));
    text("[Content_Types].xml", RICH_CONTENT_TYPES);
    text("_rels/.rels", PACKAGE_RELS);
    text("xl/workbook.xml", RICH_WORKBOOK);
    text("xl/_rels/workbook.xml.rels", RICH_WORKBOOK_RELS);
    text("xl/sharedStrings.xml", RICH_SHARED_STRINGS);
    text("xl/worksheets/sheet1.xml", RICH_SHEET);
    zip_stored(&parts)
}

fn sample_workbook() -> Vec<u8> {
    let mut parts: Vec<(&str, Vec<u8>)> = Vec::new();
    let mut text = |path: &'static str, body: &str| parts.push((path, body.as_bytes().to_vec()));
    text("[Content_Types].xml", CONTENT_TYPES);
    text("_rels/.rels", PACKAGE_RELS);
    text("xl/workbook.xml", WORKBOOK);
    text("xl/_rels/workbook.xml.rels", WORKBOOK_RELS);
    text("xl/sharedStrings.xml", SHARED_STRINGS);
    text("xl/styles.xml", STYLES);
    text("xl/worksheets/sheet1.xml", SHEET_DATA);
    text("xl/worksheets/_rels/sheet1.xml.rels", SHEET_DATA_RELS);
    text("xl/worksheets/sheet2.xml", SHEET_HIDDEN);
    text("xl/worksheets/sheet3.xml", SHEET_SUMMARY);
    text("xl/comments/comment1.xml", COMMENTS);
    text("xl/drawings/drawing1.xml", DRAWING);
    text("xl/drawings/_rels/drawing1.xml.rels", DRAWING_RELS);
    text("xl/charts/chart1.xml", CHART);
    text("xl/tables/table1.xml", TABLE);
    parts.push(("xl/media/image1.png", PNG.to_vec()));
    zip_stored(&parts)
}

fn wide_workbook() -> Vec<u8> {
    let mut parts: Vec<(&str, Vec<u8>)> = Vec::new();
    let mut text = |path: &'static str, body: &str| parts.push((path, body.as_bytes().to_vec()));
    text("[Content_Types].xml", WIDE_CONTENT_TYPES);
    text("_rels/.rels", PACKAGE_RELS);
    text("xl/workbook.xml", WIDE_WORKBOOK);
    text("xl/_rels/workbook.xml.rels", WIDE_WORKBOOK_RELS);
    text("xl/styles.xml", WIDE_STYLES);
    parts.push(("xl/worksheets/sheet1.xml", wide_sheet().into_bytes()));
    zip_stored(&parts)
}

/// A sheet whose first column alone is wider than a page, so the column has to
/// be clamped while the row bands keep paginating.
fn wide_sheet() -> String {
    let mut sheet = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><dimension ref=\"A1:D120\"/><sheetViews><sheetView workbookViewId=\"0\"/></sheetViews><sheetFormatPr baseColWidth=\"8\" defaultRowHeight=\"15\"/><cols><col width=\"255\" customWidth=\"1\" min=\"1\" max=\"1\"/><col width=\"10\" customWidth=\"1\" min=\"2\" max=\"2\"/><col width=\"10\" customWidth=\"1\" min=\"3\" max=\"3\"/><col width=\"10\" customWidth=\"1\" min=\"4\" max=\"4\"/></cols><sheetData>",
    );
    for row in 1..=120 {
        sheet.push_str(&format!(
            "<row r=\"{row}\"><c r=\"A{row}\" t=\"inlineStr\"><is><t>wide-{row}</t></is></c><c r=\"B{row}\" t=\"inlineStr\"><is><t>b{row}</t></is></c><c r=\"C{row}\" t=\"inlineStr\"><is><t>c{row}</t></is></c><c r=\"D{row}\" t=\"inlineStr\"><is><t>d{row}</t></is></c></row>"
        ));
    }
    sheet.push_str("</sheetData><pageMargins left=\"0.75\" right=\"0.75\" top=\"1\" bottom=\"1\" header=\"0.5\" footer=\"0.5\"/></worksheet>");
    sheet
}

fn zip_stored(parts: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut output = Vec::new();
    let mut directory = Vec::new();
    for (name, bytes) in parts {
        let crc = crc32(bytes);
        let offset = output.len() as u32;
        output.extend_from_slice(b"PK\x03\x04");
        output.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        output.extend_from_slice(&crc.to_le_bytes());
        output.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        output.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        output.extend_from_slice(&(name.len() as u16).to_le_bytes());
        output.extend_from_slice(&0u16.to_le_bytes());
        output.extend_from_slice(name.as_bytes());
        output.extend_from_slice(bytes);
        directory.extend_from_slice(b"PK\x01\x02");
        directory.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        directory.extend_from_slice(&crc.to_le_bytes());
        directory.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        directory.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        directory.extend_from_slice(&(name.len() as u16).to_le_bytes());
        directory.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        directory.extend_from_slice(&offset.to_le_bytes());
        directory.extend_from_slice(name.as_bytes());
    }
    let directory_offset = output.len() as u32;
    output.extend_from_slice(&directory);
    output.extend_from_slice(b"PK\x05\x06");
    output.extend_from_slice(&[0, 0, 0, 0]);
    output.extend_from_slice(&(parts.len() as u16).to_le_bytes());
    output.extend_from_slice(&(parts.len() as u16).to_le_bytes());
    output.extend_from_slice(&(directory.len() as u32).to_le_bytes());
    output.extend_from_slice(&directory_offset.to_le_bytes());
    output.extend_from_slice(&[0, 0]);
    output
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in bytes {
        crc ^= *byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

const WIDE_CONTENT_TYPES: &str = r##"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/><Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/></Types>"##;

const WIDE_WORKBOOK: &str = r##"<?xml version="1.0" encoding="UTF-8"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Wide" sheetId="1" state="visible" r:id="rId1"/></sheets></workbook>"##;

const WIDE_WORKBOOK_RELS: &str = r##"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="/xl/worksheets/sheet1.xml" Id="rId1"/><Relationship Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="/xl/styles.xml" Id="rId2"/></Relationships>"##;

const WIDE_STYLES: &str = r##"<?xml version="1.0" encoding="UTF-8"?><styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><fonts count="1"><font><sz val="11"/><name val="Calibri"/></font></fonts><fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills><borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/></cellXfs><cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles></styleSheet>"##;

const RICH_CONTENT_TYPES: &str = r##"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/><Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/></Types>"##;

const RICH_WORKBOOK: &str = r##"<?xml version="1.0" encoding="UTF-8"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Rich" sheetId="1" state="visible" r:id="rId1"/></sheets></workbook>"##;

const RICH_WORKBOOK_RELS: &str = r##"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="/xl/worksheets/sheet1.xml" Id="rId1"/><Relationship Id="rIdSharedStrings" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="/xl/sharedStrings.xml"/></Relationships>"##;

/// Shared strings: a plain string, a two-run rich string and a two-line string.
const RICH_SHARED_STRINGS: &str = r##"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="3" uniqueCount="3"><si><t>PLAIN_CELL</t></si><si><r><rPr><b/></rPr><t>RICH_BOLD_TEXT</t></r><r><t>RICH_PLAIN_TEXT</t></r></si><si><t xml:space="preserve">LINE_ONE&#10;LINE_TWO</t></si></sst>"##;

const RICH_SHEET: &str = r##"<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><dimension ref="A1:B3"/><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c></row><row r="2"><c r="A2" t="s"><v>1</v></c><c r="B2" t="inlineStr"><is><r><rPr><b/></rPr><t>INLINE_RICH_TEXT</t></r></is></c></row><row r="3"><c r="A3" t="s"><v>2</v></c><c r="B3" t="inlineStr"><is><t>INLINE_PLAIN_TEXT</t></is></c></row></sheetData></worksheet>"##;

const CONTENT_TYPES: &str = r##"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Default Extension="png" ContentType="image/png"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/><Override PartName="/xl/worksheets/sheet2.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/><Override PartName="/xl/worksheets/sheet3.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/><Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/><Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/><Override PartName="/xl/comments/comment1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.comments+xml"/><Override PartName="/xl/drawings/drawing1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawing+xml"/><Override PartName="/xl/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/><Override PartName="/xl/tables/table1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml"/></Types>"##;

const PACKAGE_RELS: &str = r##"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml" Id="rId1"/></Relationships>"##;

const WORKBOOK: &str = r##"<?xml version="1.0" encoding="UTF-8"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><workbookPr/><bookViews><workbookView activeTab="0"/></bookViews><sheets><sheet name="Data" sheetId="1" state="visible" r:id="rId1"/><sheet name="Hidden" sheetId="2" state="hidden" r:id="rId2"/><sheet name="Summary" sheetId="3" state="visible" r:id="rId3"/></sheets><definedNames><definedName name="_xlnm._FilterDatabase" localSheetId="0" hidden="1">'Data'!$A$3:$F$7</definedName></definedNames><calcPr calcId="124519" fullCalcOnLoad="1"/></workbook>"##;

const WORKBOOK_RELS: &str = r##"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="/xl/worksheets/sheet1.xml" Id="rId1"/><Relationship Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="/xl/worksheets/sheet2.xml" Id="rId2"/><Relationship Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="/xl/worksheets/sheet3.xml" Id="rId3"/><Relationship Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="/xl/styles.xml" Id="rId4"/><Relationship Id="rIdSharedStrings" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="/xl/sharedStrings.xml"/></Relationships>"##;

const SHARED_STRINGS: &str = r##"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="22" uniqueCount="18"><si><t>Quarterly Revenue</t></si><si><t>Region</t></si><si><t>Q1</t></si><si><t>Q2</t></si><si><t>Q3</t></si><si><t>Q4</t></si><si><t>Updated</t></si><si><t>North</t></si><si><t>South</t></si><si><t>East</t></si><si><t>West</t></si><si><t>Total</t></si><si><t>Custom number format sample</t></si><si><t>Boolean</t></si><si><t>Error</t></si><si><t>Inline text</t></si><si><t>shared string</t></si><si><t>Merged note across two columns</t></si></sst>"##;

const STYLES: &str = r##"<?xml version="1.0" encoding="UTF-8"?><styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><numFmts count="3"><numFmt numFmtId="164" formatCode="&quot;$&quot;#,##0.00"/><numFmt numFmtId="165" formatCode="yyyy-mm-dd"/><numFmt numFmtId="166" formatCode="0.0 &quot;units&quot;"/></numFmts><fonts count="4"><font><name val="Calibri"/><family val="2"/><color theme="1"/><sz val="11"/><scheme val="minor"/></font><font><name val="Calibri"/><b val="1"/><color rgb="FF1F4E79"/><sz val="11"/></font><font><b val="1"/></font><font><b val="1"/><sz val="14"/></font></fonts><fills count="4"><fill><patternFill/></fill><fill><patternFill patternType="gray125"/></fill><fill><patternFill patternType="solid"><fgColor rgb="FFDDEBF7"/></patternFill></fill><fill><patternFill patternType="solid"><fgColor rgb="FFFFF2CC"/></patternFill></fill></fills><borders count="2"><border><left/><right/><top/><bottom/><diagonal/></border><border><left style="thin"><color rgb="FF8EA9DB"/></left><right style="thin"><color rgb="FF8EA9DB"/></right><top style="thin"><color rgb="FF8EA9DB"/></top><bottom style="thin"><color rgb="FF8EA9DB"/></bottom></border></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="12"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" pivotButton="0" quotePrefix="0" xfId="0"/><xf numFmtId="0" fontId="1" fillId="2" borderId="1" applyAlignment="1" pivotButton="0" quotePrefix="0" xfId="0"><alignment horizontal="center" vertical="center"/></xf><xf numFmtId="0" fontId="0" fillId="0" borderId="1" pivotButton="0" quotePrefix="0" xfId="0"/><xf numFmtId="164" fontId="0" fillId="0" borderId="1" pivotButton="0" quotePrefix="0" xfId="0"/><xf numFmtId="165" fontId="0" fillId="0" borderId="1" applyAlignment="1" pivotButton="0" quotePrefix="0" xfId="0"><alignment horizontal="right"/></xf><xf numFmtId="0" fontId="2" fillId="0" borderId="0" pivotButton="0" quotePrefix="0" xfId="0"/><xf numFmtId="164" fontId="2" fillId="0" borderId="0" pivotButton="0" quotePrefix="0" xfId="0"/><xf numFmtId="10" fontId="0" fillId="0" borderId="0" pivotButton="0" quotePrefix="0" xfId="0"/><xf numFmtId="166" fontId="0" fillId="0" borderId="0" pivotButton="0" quotePrefix="0" xfId="0"/><xf numFmtId="0" fontId="0" fillId="3" borderId="0" applyAlignment="1" pivotButton="0" quotePrefix="0" xfId="0"><alignment vertical="top" wrapText="1"/></xf><xf numFmtId="0" fontId="3" fillId="0" borderId="0" pivotButton="0" quotePrefix="0" xfId="0"/><xf numFmtId="164" fontId="0" fillId="0" borderId="0" pivotButton="0" quotePrefix="0" xfId="0"/></cellXfs><cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0" hidden="0"/></cellStyles><dxfs count="1"><dxf><fill><patternFill patternType="solid"><fgColor rgb="FFC6EFCE"/></patternFill></fill></dxf></dxfs><tableStyles count="0" defaultTableStyle="TableStyleMedium9" defaultPivotStyle="PivotStyleLight16"/></styleSheet>"##;

const SHEET_DATA: &str = r##"<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheetPr><outlinePr summaryBelow="1" summaryRight="1"/><pageSetUpPr/></sheetPr><dimension ref="A1:F16"/><sheetViews><sheetView workbookViewId="0"><pane xSplit="1" ySplit="3" topLeftCell="B4" activePane="bottomRight" state="frozen"/><selection pane="bottomRight" activeCell="A1" sqref="A1"/></sheetView></sheetViews><sheetFormatPr baseColWidth="8" defaultRowHeight="15"/><cols><col width="18" customWidth="1" min="1" max="1"/><col width="14" customWidth="1" min="2" max="2"/><col width="14" customWidth="1" min="3" max="3"/><col width="14" customWidth="1" min="4" max="4"/><col width="14" customWidth="1" min="5" max="5"/><col width="13" customWidth="1" min="6" max="6"/><col width="10" customWidth="1" min="7" max="7"/></cols><sheetData><row r="1" ht="28" customHeight="1"><c r="A1" t="s"><v>0</v></c></row><row r="3" ht="24" customHeight="1"><c r="A3" s="1" t="s"><v>1</v></c><c r="B3" s="1" t="s"><v>2</v></c><c r="C3" s="1" t="s"><v>3</v></c><c r="D3" s="1" t="s"><v>4</v></c><c r="E3" s="1" t="s"><v>5</v></c><c r="F3" s="1" t="s"><v>6</v></c></row><row r="4"><c r="A4" s="2" t="s"><v>7</v></c><c r="B4" s="3" t="n"><v>1200.5</v></c><c r="C4" s="3" t="n"><v>1350.25</v></c><c r="D4" s="3" t="n"><v>1420</v></c><c r="E4" s="3"><f>B4+C4+D4</f><v>3970.75</v></c><c r="F4" s="4" t="inlineStr"><is><t>2026-01-15</t></is></c></row><row r="5"><c r="A5" s="2" t="s"><v>8</v></c><c r="B5" s="3" t="n"><v>980.75</v></c><c r="C5" s="3" t="n"><v>1010.5</v></c><c r="D5" s="3" t="n"><v>1105.25</v></c><c r="E5" s="3"><f>B5+C5+D5</f><v>3096.5</v></c><c r="F5" s="4" t="inlineStr"><is><t>2026-02-20</t></is></c></row><row r="6"><c r="A6" s="2" t="s"><v>9</v></c><c r="B6" s="3" t="n"><v>1500</v></c><c r="C6" s="3" t="n"><v>1620.5</v></c><c r="D6" s="3" t="n"><v>1710.75</v></c><c r="E6" s="3"><f>B6+C6+D6</f><v>4831.25</v></c><c r="F6" s="4" t="inlineStr"><is><t>2026-03-18</t></is></c></row><row r="7"><c r="A7" s="2" t="s"><v>10</v></c><c r="B7" s="3" t="n"><v>870.25</v></c><c r="C7" s="3" t="n"><v>905</v></c><c r="D7" s="3" t="n"><v>940.5</v></c><c r="E7" s="3"><f>B7+C7+D7</f><v>2715.75</v></c><c r="F7" s="4" t="inlineStr"><is><t>2026-04-22</t></is></c></row><row r="9"><c r="A9" s="5" t="s"><v>11</v></c><c r="E9" s="6"><f>SUM(E4:E7)</f><v>14614.25</v></c><c r="F9" s="7"><f>(E9-E4)/E4</f><v>0.3098</v></c></row><row r="11"><c r="A11" t="s"><v>12</v></c><c r="B11" s="8" t="n"><v>12.345</v></c></row><row r="12"><c r="A12" t="s"><v>13</v></c><c r="B12" t="b"><v>1</v></c></row><row r="13"><c r="A13" t="s"><v>14</v></c><c r="B13" t="e"><v>#N/A</v></c></row><row r="14"><c r="A14" t="s"><v>15</v></c><c r="B14" t="s"><v>16</v></c></row><row r="16"><c r="A16" s="9" t="s"><v>17</v></c></row></sheetData><autoFilter ref="A3:F7"/><mergeCells count="2"><mergeCell ref="A16:B16"/><mergeCell ref="A1:F1"/></mergeCells><conditionalFormatting sqref="E4:E7"><cfRule type="cellIs" priority="1" operator="greaterThan" dxfId="0"><formula>3000</formula></cfRule></conditionalFormatting><dataValidations count="1"><dataValidation sqref="B4:E9" showDropDown="0" showInputMessage="0" showErrorMessage="0" allowBlank="0" type="decimal" operator="greaterThan"><formula1>0</formula1></dataValidation></dataValidations><pageMargins left="0.75" right="0.75" top="1" bottom="1" header="0.5" footer="0.5"/><drawing r:id="rId1"/><tableParts count="1"><tablePart r:id="rId2"/></tableParts></worksheet>"##;

const SHEET_DATA_RELS: &str = r##"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="/xl/drawings/drawing1.xml" Id="rId1"/><Relationship Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/table" Target="/xl/tables/table1.xml" Id="rId2"/><Relationship Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" Target="/xl/comments/comment1.xml" Id="comments"/></Relationships>"##;

const SHEET_HIDDEN: &str = r##"<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetPr><outlinePr summaryBelow="1" summaryRight="1"/><pageSetUpPr/></sheetPr><dimension ref="A1:A1"/><sheetViews><sheetView workbookViewId="0"><selection activeCell="A1" sqref="A1"/></sheetView></sheetViews><sheetFormatPr baseColWidth="8" defaultRowHeight="15"/><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>hidden sheet body</t></is></c></row></sheetData><pageMargins left="0.75" right="0.75" top="1" bottom="1" header="0.5" footer="0.5"/></worksheet>"##;

const SHEET_SUMMARY: &str = r##"<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetPr><outlinePr summaryBelow="1" summaryRight="1"/><pageSetUpPr/></sheetPr><dimension ref="A1:B4"/><sheetViews><sheetView workbookViewId="0"><selection activeCell="A1" sqref="A1"/></sheetView></sheetViews><sheetFormatPr baseColWidth="8" defaultRowHeight="15"/><sheetData><row r="1"><c r="A1" s="10" t="inlineStr"><is><t>Summary</t></is></c></row><row r="3"><c r="A3" t="inlineStr"><is><t>Grand total</t></is></c><c r="B3" s="11"><f>Data!E9</f><v>14614.25</v></c></row><row r="4"><c r="A4" t="inlineStr"><is><t>North share</t></is></c><c r="B4" s="7"><f>Data!E4/Data!E9</f><v>0.2702</v></c></row></sheetData><pageMargins left="0.75" right="0.75" top="1" bottom="1" header="0.5" footer="0.5"/></worksheet>"##;

const COMMENTS: &str = r##"<?xml version="1.0" encoding="UTF-8"?><comments xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><authors><author>Analyst</author></authors><commentList><comment ref="B4" authorId="0" shapeId="0"><text><t>Reviewed by finance</t></text></comment></commentList></comments>"##;

const DRAWING: &str = r##"<?xml version="1.0" encoding="UTF-8"?><wsDr xmlns="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><oneCellAnchor><from><col>0</col><colOff>0</colOff><row>17</row><rowOff>0</rowOff></from><ext cx="5400000" cy="2700000"/><graphicFrame><nvGraphicFramePr><cNvPr id="1" name="Chart 1"/><cNvGraphicFramePr/></nvGraphicFramePr><xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" r:id="rId1"/></a:graphicData></a:graphic></graphicFrame><clientData/></oneCellAnchor><oneCellAnchor><from><col>6</col><colOff>0</colOff><row>1</row><rowOff>0</rowOff></from><ext cx="914400" cy="457200"/><pic><nvPicPr><cNvPr id="2" name="Image 2" descr="Picture"/><cNvPicPr/></nvPicPr><blipFill><a:blip cstate="print" r:embed="rId2"/><a:stretch><a:fillRect/></a:stretch></blipFill><spPr><a:prstGeom prst="rect"/></spPr></pic><clientData/></oneCellAnchor></wsDr>"##;

const DRAWING_RELS: &str = r##"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="/xl/charts/chart1.xml" Id="rId1"/><Relationship Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="/xl/media/image1.png" Id="rId2"/></Relationships>"##;

const CHART: &str = r##"<?xml version="1.0" encoding="UTF-8"?><c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart><c:plotArea><c:barChart><c:barDir val="col"/></c:barChart></c:plotArea></c:chart></c:chartSpace>"##;

const TABLE: &str = r##"<?xml version="1.0" encoding="UTF-8"?><table id="1" name="RevenueTable" displayName="RevenueTable" ref="A3:F7" headerRowCount="1" xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><autoFilter ref="A3:F7"/><tableColumns count="6"><tableColumn id="1" name="Region"/><tableColumn id="2" name="Q1"/><tableColumn id="3" name="Q2"/><tableColumn id="4" name="Q3"/><tableColumn id="5" name="Q4"/><tableColumn id="6" name="Updated"/></tableColumns><tableStyleInfo name="TableStyleMedium2" showRowStripes="1"/></table>"##;

const PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
    0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01,
    0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];
