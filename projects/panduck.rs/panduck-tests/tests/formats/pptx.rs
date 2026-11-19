//! PPTX read, placeholder inheritance, IR projection and slide canvas acceptance.
//!
//! The sample package is built in memory so the implementation repository stays
//! free of binary office fixtures. The same bytes are written to the target
//! temporary directory for screenshot evidence.

use std::{fs, path::PathBuf};

use document_formats::import::pptx::{
    EstimatedMetrics, Fill, GraphOptions, PlaceholderKind, RenderOptions, ShapeKind, TextMeasurer, import_pptx_bytes, presentation_to_graph,
    render_presentation, render_presentation_with, text_wrap_probe,
};
use document_ir::{Block, DocumentGraph, Inline, TableCell};

fn presentation() -> document_formats::import::pptx::PptxPresentation {
    let bytes = sample_package();
    import_pptx_bytes("generated-layouts-and-chart.pptx", &bytes).expect("pptx import")
}

#[test]
fn slides_layouts_and_stats_are_listed() {
    let presentation = presentation();
    assert_eq!(presentation.stats.slide_count, 2);
    assert_eq!(presentation.stats.hidden_slide_count, 0);
    assert_eq!(presentation.stats.master_count, 1);
    assert_eq!(presentation.stats.layout_count, 2);
    assert_eq!(presentation.stats.shape_count, 8, "{:?}", presentation.stats);
    assert_eq!(presentation.stats.image_count, 1);
    assert_eq!(presentation.stats.table_count, 1);
    assert_eq!(presentation.stats.chart_count, 1);
    assert_eq!(presentation.stats.notes_count, 2);
    assert!(presentation.stats.non_whitespace_chars > 80, "{:?}", presentation.stats);
    assert_eq!(presentation.size.cx, 12192000);
    assert_eq!(presentation.size.width_px(), 1280.0);
    assert_eq!(presentation.size.height_px(), 720.0);
    assert_eq!(presentation.slide(1).unwrap().layout_name.as_deref(), Some("Title Slide"));
    assert_eq!(presentation.slide(2).unwrap().layout_name.as_deref(), Some("Title and Content"));
}

#[test]
fn placeholder_geometry_is_inherited_from_layout_and_master() {
    let presentation = presentation();
    let first = presentation.slide(1).unwrap();
    let title = first
        .shapes
        .iter()
        .find(|shape| shape.placeholder.as_ref().map(|placeholder| placeholder.kind == PlaceholderKind::CenterTitle).unwrap_or(false))
        .unwrap();
    assert!(title.inherited_transform);
    let transform = title.transform.unwrap();
    assert_eq!((transform.x, transform.y), (685800, 2130425));
    assert_eq!((transform.cx, transform.cy), (7772400, 1470025));

    let subtitle = first.shapes.iter().find(|shape| shape.placeholder.as_ref().and_then(|placeholder| placeholder.index) == Some(1)).unwrap();
    assert_eq!(subtitle.transform.unwrap().x, 1371600);

    let second = presentation.slide(2).unwrap();
    assert_eq!(second.shapes[0].transform.unwrap().x, 457200, "the title falls back to the master title placeholder");
    let body = second.shapes.iter().find(|shape| shape.placeholder.as_ref().and_then(|placeholder| placeholder.index) == Some(1)).unwrap();
    let body_transform = body.transform.unwrap();
    assert_eq!((body_transform.x, body_transform.y), (457200, 1600200));
    assert!(body.inherited_transform);
}

#[test]
fn text_shapes_keep_paragraph_levels_fills_lines_and_rotation() {
    let presentation = presentation();
    let slide = presentation.slide(1).unwrap();
    let rounded = slide.shapes.iter().find(|shape| shape.name == "Rounded Rectangle 3").unwrap();
    assert_eq!(rounded.kind, ShapeKind::Shape);
    assert_eq!(rounded.geometry.as_deref(), Some("roundRect"));
    assert!(matches!(rounded.fill, Some(Fill::Solid(_))), "{:?}", rounded.fill);
    assert!(rounded.line.as_ref().and_then(|line| line.color.as_ref()).is_some());
    assert_eq!(rounded.transform.unwrap().rotation_deg, 8.0);

    let body = slide.shapes.iter().find(|shape| shape.placeholder.as_ref().and_then(|placeholder| placeholder.index) == Some(1)).unwrap();
    let text = body.text.as_ref().unwrap();
    assert_eq!(text.paragraphs.len(), 2);
    assert_eq!(text.paragraphs[0].level, 0);
    assert_eq!(text.paragraphs[1].level, 1);
    assert!(text.paragraphs[1].text().contains("Level 1 paragraph"));

    let picture = slide.shapes.iter().find(|shape| shape.name == "Picture 4").unwrap();
    assert!(picture.image.as_ref().unwrap().bytes.as_ref().is_some_and(|bytes| bytes.starts_with(b"\x89PNG")));
}

#[test]
fn tables_charts_and_notes_are_read() {
    let presentation = presentation();
    let slide = presentation.slide(2).unwrap();
    let table_shape = slide.shapes.iter().find(|shape| shape.kind == ShapeKind::Table).unwrap();
    let table = table_shape.table.as_ref().unwrap();
    assert_eq!(table.column_widths.len(), 2);
    assert_eq!(table.rows.len(), 2);
    assert_eq!(table.rows[0].cells.len(), 2);
    assert_eq!(table.rows[0].cells[0].text.paragraphs[0].text(), "Region");

    let chart_shape = slide.shapes.iter().find(|shape| shape.kind == ShapeKind::Chart).unwrap();
    let chart = chart_shape.chart.as_ref().unwrap();
    assert_eq!(chart.kind.as_deref(), Some("barChart"));
    assert_eq!(chart.series_count, 1);
    assert_eq!(chart.categories, vec!["North".to_owned(), "South".to_owned(), "East".to_owned()]);
    assert_eq!(chart.series[0].name.as_deref(), Some("Revenue"));
    assert_eq!(chart.series[0].values, vec![42.0, 31.0, 27.0]);

    assert_eq!(presentation.slide(1).unwrap().notes.as_deref(), Some("Speaker notes: mention the five capability states."));
}

#[test]
fn refused_constructs_carry_structured_codes() {
    let presentation = presentation();
    let codes = |code: &str| presentation.issues.iter().filter(|issue| issue.code == code).count();
    assert!(codes("import.pptx.chart") >= 1, "{:?}", presentation.issues);
    assert!(codes("import.pptx.placeholder-inheritance") >= 4);
    assert!(codes("import.pptx.rotation") >= 1);
    assert!(codes("import.pptx.notes") >= 1);
    assert!(codes("import.pptx.master-shape") >= 1);
    let chart = presentation.issues.iter().find(|issue| issue.code == "import.pptx.chart").unwrap();
    assert_eq!(chart.status, document_formats::import::pptx::IssueStatus::Partial);
    assert_eq!(chart.slide, Some(2));
    assert!(presentation.issues.iter().all(|issue| !issue.code.is_empty()));
}

#[test]
fn ir_projection_keeps_slide_sections_tables_and_refusals() {
    let presentation = presentation();
    let graph = presentation_to_graph(&presentation, &GraphOptions::default());
    let sections: Vec<&Block> = graph.blocks.iter().map(|node| &node.block).filter(|block| matches!(block, Block::Section { .. })).collect();
    assert_eq!(sections.len(), 2);
    let Block::Section { title, .. } = sections[0]
    else {
        unreachable!()
    };
    assert!(matches!(&title[0], Inline::Text { text } if text == "Office line review"), "{title:?}");
    let table = graph.blocks.iter().find_map(|node| match &node.block {
        Block::Table { head, body, foot, caption } => Some((caption, head, body, foot)),
        _ => None,
    });
    let (caption, head, body, foot) = table.expect("table block");
    assert_eq!(caption.as_deref(), Some("Revenue table"));
    let flat: String = head
        .iter()
        .chain(body)
        .chain(foot)
        .flat_map(|row| row.cells.iter().flat_map(|cell| cell_text(&graph, cell)))
        .collect::<Vec<_>>()
        .join(" ");
    assert!(flat.contains("Region"), "{flat}");

    let body: String = graph
        .blocks
        .iter()
        .filter_map(|node| match &node.block {
            Block::Paragraph { content } => Some(
                content
                    .iter()
                    .map(|inline| {
                        let mut text = String::new();
                        flattened_text(inline, &mut text);
                        text
                    })
                    .collect::<Vec<_>>()
                    .join(""),
            ),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(body.contains("Revenue grew in every region."), "{body}");
    assert!(body.contains("Notes: Speaker notes"), "{body}");
    assert!(body.contains("[barChart chart: 3 categories, 1 series, 3 values]"), "{body}");

    let loss_codes: Vec<&str> = graph.coverage.loss.iter().map(|loss| loss.code.as_str()).collect();
    assert!(loss_codes.contains(&"import.pptx.chart"), "{loss_codes:?}");
    assert!(loss_codes.contains(&"import.pptx.rotation"), "{loss_codes:?}");
    let sized = graph.blocks.iter().any(|node| match &node.block {
        Block::Paragraph { content } => content.iter().any(|inline| matches!(inline, Inline::Attributed { attributes, .. } if attributes.pairs.iter().any(|pair| pair.key == "pptx.font-size"))),
        _ => false,
    });
    assert!(sized, "run sizes survive the projection");
}

/// Collects style names from every inline tree reachable from a block, including
/// table cell child blocks.
fn collect_styles(graph: &DocumentGraph, block: &Block, styles: &mut Vec<String>) {
    match block {
        Block::Paragraph { content } => collect_inline_styles(content, styles),
        Block::Section { title, .. } => collect_inline_styles(title, styles),
        Block::List { items, .. } => {
            for item in items {
                collect_inline_styles(&item.content, styles);
            }
        }
        Block::Table { head, body, foot, .. } => {
            for row in head.iter().chain(body).chain(foot) {
                for cell in &row.cells {
                    for id in &cell.children {
                        if let Some(node) = graph.block(*id) {
                            collect_styles(graph, &node.block, styles);
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

fn collect_inline_styles(inlines: &[Inline], styles: &mut Vec<String>) {
    for inline in inlines {
        match inline {
            Inline::Styled { style, children } => {
                styles.push(style.clone());
                collect_inline_styles(children, styles);
            }
            Inline::Attributed { children, .. } => collect_inline_styles(children, styles),
            Inline::Link { display, .. } => collect_inline_styles(display, styles),
            Inline::Image { alt, .. } => collect_inline_styles(alt, styles),
            Inline::Note { display: Some(display), .. } => collect_inline_styles(display, styles),
            Inline::Citation { display: Some(display), .. } => collect_inline_styles(display, styles),
            _ => {}
        }
    }
}

#[test]
fn pptx_bold_runs_use_the_canonical_bold_style_name() {
    let presentation = presentation();
    let graph = presentation_to_graph(&presentation, &GraphOptions::default());
    let mut styles = Vec::new();
    for node in &graph.blocks {
        collect_styles(&graph, &node.block, &mut styles);
    }
    assert!(styles.iter().any(|style| style == "bold"), "bold runs must keep their emphasis: {styles:?}");
    assert!(!styles.iter().any(|style| style == "strong"), "the legacy `strong` spelling must not leak: {styles:?}");
}

/// A measurer that charges 1.5 em per character, so it wraps earlier than the
/// estimate and proves the canvas accepts injected metrics.
struct WideMetrics;

impl TextMeasurer for WideMetrics {
    fn advance(&self, _: char, font_px: f64) -> f64 {
        font_px * 1.5
    }
}

#[test]
fn injected_metrics_wrap_earlier_and_chart_fragments_replace_the_placeholder() {
    let presentation = presentation();
    let estimated = text_wrap_probe(&presentation, &EstimatedMetrics);
    let measured = text_wrap_probe(&presentation, &WideMetrics);
    assert_eq!(estimated.len(), measured.len());
    assert!(estimated.iter().zip(&measured).any(|(left, right)| right.lines.len() > left.lines.len()), "{estimated:?} {measured:?}");

    let chart = presentation.slide(2).expect("slide 2").shapes.iter().find(|shape| shape.kind == ShapeKind::Chart).expect("chart shape");
    let fragment = "<rect x=\"0\" y=\"0\" width=\"10\" height=\"10\" fill=\"#336699\"/>";
    let render = render_presentation_with(&presentation, &RenderOptions::default(), &WideMetrics, &[(2, chart.id, fragment.to_owned())]);
    let slide = render.views.iter().find(|view| view.index == 2).expect("slide view 2");
    assert!(slide.svg.contains(fragment), "{}", slide.svg);
    assert!(!slide.svg.contains("chart refused"), "{}", slide.svg);
    assert!(
        !render.issues.iter().any(|issue| issue.code == "render.pptx.chart-placeholder" && issue.shape == Some(chart.id)),
        "{:?}",
        render.issues
    );
}

/// Collects the text of an inline tree, descending through style wrappers.
fn flattened_text(inline: &Inline, output: &mut String) {
    match inline {
        Inline::Text { text } => output.push_str(text),
        Inline::Styled { children, .. } | Inline::Attributed { children, .. } => {
            for child in children {
                flattened_text(child, output);
            }
        }
        _ => {}
    }
}

fn cell_text(graph: &DocumentGraph, cell: &TableCell) -> Vec<String> {
    cell.children
        .iter()
        .filter_map(|id| graph.block(*id))
        .flat_map(|node| match &node.block {
            Block::Paragraph { content } => content.clone(),
            _ => Vec::new(),
        })
        .map(|inline| {
            let mut text = String::new();
            flattened_text(&inline, &mut text);
            text
        })
        .collect()
}

fn pptx_row_text(row: &document_formats::import::pptx::PptxTableRow) -> Vec<String> {
    row.cells.iter().map(|cell| cell.text.paragraphs.iter().map(|paragraph| paragraph.text()).collect::<Vec<_>>().join("")).collect()
}

#[test]
fn render_paints_every_slide_and_writes_evidence_html() {
    let presentation = presentation();
    let render = render_presentation(&presentation, &RenderOptions::default());
    assert_eq!(render.views.len(), 2);
    assert_eq!(render.views[0].width_px, 1280.0);
    assert_eq!(render.views[0].height_px, 720.0);
    assert!(render.views[0].svg.starts_with("<svg "));
    assert!(render.views[0].svg.contains("Office line review"));
    assert!(render.views.iter().all(|view| view.svg.contains("</svg>")));
    assert!(render.views[1].svg.contains("Region"));
    assert!(render.views[1].svg.contains("chart refused"), "{}", render.views[1].svg);
    assert!(render.html.contains("id=\"slide-2\""));
    assert!(render.html.contains("data:image/png;base64,"));
    assert!(render.html.contains("Speaker notes"));
    assert!(render.issues.iter().any(|issue| issue.code == "render.pptx.chart-placeholder"));
    assert!(render.issues.iter().all(|issue| !issue.code.is_empty()));
    assert!(!render.views[0].deviations.is_empty());

    let evidence =
        std::env::var("CARGO_TARGET_TMPDIR").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir().join("document-rs-evidence"));
    let directory = evidence.join("office-evidence/pptx");
    fs::create_dir_all(&directory).expect("evidence dir");
    fs::write(directory.join("generated-layouts-and-chart.pptx"), sample_package()).expect("sample pptx");
    let output = directory.join("layouts-and-chart.html");
    fs::write(&output, &render.html).expect("evidence html");
    println!("pptx evidence html: {}", output.display());
}

#[test]
fn row_cells_stay_on_their_row() {
    let presentation = presentation();
    let slide = presentation.slide(2).unwrap();
    let table = slide.shapes.iter().find(|shape| shape.kind == ShapeKind::Table).unwrap().table.as_ref().unwrap();
    assert_eq!(pptx_row_text(&table.rows[1]), vec!["North".to_owned(), "42".to_owned()]);
}

#[test]
fn multi_paragraph_table_cells_keep_a_paragraph_boundary_and_name_the_loss() {
    let bytes = sample_package_with_slide_two(SLIDE_TWO_MULTI_PARAGRAPH_CELL);
    let presentation = import_pptx_bytes("multi-paragraph-cell.pptx", &bytes).expect("pptx import");
    let graph = presentation_to_graph(&presentation, &GraphOptions::default());
    let body = graph
        .blocks
        .iter()
        .find_map(|node| match &node.block {
            Block::Table { body, .. } => Some(body.as_slice()),
            _ => None,
        })
        .expect("table block");

    let multi = cell_paragraph_blocks(&graph, &body[1].cells[0]);
    let texts = cell_paragraph_texts(&graph, &body[1].cells[0]);
    assert_eq!(multi.len(), 2, "each authored cell paragraph is its own cell block: {multi:?}");
    assert_eq!(
        texts,
        vec!["ALPHA_CELL_PARAGRAPH".to_owned(), "BETA_CELL_PARAGRAPH".to_owned()],
        "the two authored paragraphs stay separate instead of gluing into one text: {texts:?}"
    );

    let single = cell_paragraph_blocks(&graph, &body[1].cells[1]);
    assert_eq!(single.len(), 1, "a single-paragraph cell keeps one cell block: {single:?}");

    let losses: Vec<&str> = graph.coverage.loss.iter().map(|loss| loss.code.as_str()).collect();
    assert_eq!(
        losses.iter().filter(|code| **code == "import.pptx.table-cell-paragraphs").count(),
        1,
        "one marker names the multi-paragraph cell: {losses:?}"
    );
    let marker = graph.coverage.loss.iter().find(|loss| loss.code == "import.pptx.table-cell-paragraphs").unwrap();
    assert!(marker.message.contains("row 2, column 1"), "the marker names the cell: {}", marker.message);
}

#[test]
fn single_paragraph_table_cells_emit_no_paragraph_marker() {
    let presentation = presentation();
    let graph = presentation_to_graph(&presentation, &GraphOptions::default());
    let losses: Vec<&str> = graph.coverage.loss.iter().map(|loss| loss.code.as_str()).collect();
    assert!(!losses.contains(&"import.pptx.table-cell-paragraphs"), "{losses:?}");
}

fn cell_paragraph_blocks<'a>(graph: &'a DocumentGraph, cell: &TableCell) -> Vec<&'a Block> {
    cell.children
        .iter()
        .filter_map(|id| graph.block(*id))
        .map(|node| &node.block)
        .filter(|block| matches!(block, Block::Paragraph { .. }))
        .collect()
}

fn cell_paragraph_texts(graph: &DocumentGraph, cell: &TableCell) -> Vec<String> {
    cell.children
        .iter()
        .filter_map(|id| graph.block(*id))
        .filter_map(|node| match &node.block {
            Block::Paragraph { content } => {
                let mut text = String::new();
                for inline in content {
                    flattened_text(inline, &mut text);
                }
                Some(text)
            }
            _ => None,
        })
        .collect()
}

// --- in-memory OOXML sample -------------------------------------------------

fn sample_package() -> Vec<u8> {
    sample_package_with_slide_two(SLIDE_TWO)
}

fn sample_package_with_slide_two(slide_two: &str) -> Vec<u8> {
    let mut parts: Vec<(&str, Vec<u8>)> = Vec::new();
    let mut text = |path: &'static str, body: &str| parts.push((path, body.as_bytes().to_vec()));
    text("[Content_Types].xml", CONTENT_TYPES);
    text("_rels/.rels", PACKAGE_RELS);
    text("ppt/presentation.xml", PRESENTATION);
    text("ppt/_rels/presentation.xml.rels", PRESENTATION_RELS);
    text("ppt/slideMasters/slideMaster1.xml", MASTER);
    text("ppt/slideMasters/_rels/slideMaster1.xml.rels", MASTER_RELS);
    text("ppt/slideLayouts/slideLayout1.xml", LAYOUT_ONE);
    text("ppt/slideLayouts/slideLayout2.xml", LAYOUT_TWO);
    text("ppt/theme/theme1.xml", THEME);
    text("ppt/slides/slide1.xml", SLIDE_ONE);
    text("ppt/slides/_rels/slide1.xml.rels", SLIDE_ONE_RELS);
    text("ppt/slides/slide2.xml", slide_two);
    text("ppt/slides/_rels/slide2.xml.rels", SLIDE_TWO_RELS);
    text("ppt/notesSlides/notesSlide1.xml", NOTES_ONE);
    text("ppt/notesSlides/notesSlide2.xml", NOTES_TWO);
    text("ppt/charts/chart1.xml", CHART);
    parts.push(("ppt/media/image1.png", PNG.to_vec()));
    zip_stored(&parts)
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

const CONTENT_TYPES: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Default Extension=\"png\" ContentType=\"image/png\"/><Override PartName=\"/ppt/presentation.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml\"/><Override PartName=\"/ppt/slides/slide1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slide+xml\"/><Override PartName=\"/ppt/slides/slide2.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slide+xml\"/><Override PartName=\"/ppt/slideMasters/slideMaster1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml\"/><Override PartName=\"/ppt/slideLayouts/slideLayout1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml\"/><Override PartName=\"/ppt/slideLayouts/slideLayout2.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml\"/><Override PartName=\"/ppt/notesSlides/notesSlide1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml\"/><Override PartName=\"/ppt/notesSlides/notesSlide2.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml\"/><Override PartName=\"/ppt/charts/chart1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.drawingml.chart+xml\"/><Override PartName=\"/ppt/theme/theme1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.theme+xml\"/></Types>";

const PACKAGE_RELS: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"ppt/presentation.xml\"/></Relationships>";

const PRESENTATION: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><p:presentation xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"><p:sldMasterIdLst><p:sldMasterId id=\"2147483648\" r:id=\"rId1\"/></p:sldMasterIdLst><p:sldIdLst><p:sldId id=\"256\" r:id=\"rId2\"/><p:sldId id=\"257\" r:id=\"rId3\"/></p:sldIdLst><p:sldSz cx=\"12192000\" cy=\"6858000\"/><p:defaultTextStyle><a:lvl1pPr><a:defRPr sz=\"1800\"/></a:lvl1pPr></p:defaultTextStyle></p:presentation>";

const PRESENTATION_RELS: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster\" Target=\"slideMasters/slideMaster1.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" Target=\"slides/slide1.xml\"/><Relationship Id=\"rId3\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" Target=\"slides/slide2.xml\"/></Relationships>";

const MASTER: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><p:sldMaster xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"><p:cSld><p:spTree><p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Master Title\"/><p:cNvSpPr/><p:nvPr><p:ph type=\"title\"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"457200\" y=\"274638\"/><a:ext cx=\"8229600\" cy=\"1143000\"/></a:xfrm></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang=\"en-US\" sz=\"4400\"/><a:t>Master title</a:t></a:r></a:p></p:txBody></p:sp><p:sp><p:nvSpPr><p:cNvPr id=\"3\" name=\"Master Body\"/><p:cNvSpPr/><p:nvPr><p:ph type=\"body\" idx=\"1\"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"457200\" y=\"1600200\"/><a:ext cx=\"8229600\" cy=\"4525963\"/></a:xfrm></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang=\"en-US\" sz=\"2400\"/><a:t>Master body</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld><p:clrMap bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk1\" accent1=\"accent1\"/><p:txStyles><p:titleStyle><a:lvl1pPr><a:defRPr sz=\"4400\" b=\"1\"/></a:lvl1pPr></p:titleStyle><p:bodyStyle><a:lvl1pPr><a:defRPr sz=\"2400\"/></a:lvl1pPr><a:lvl2pPr><a:defRPr sz=\"2000\"/></a:lvl2pPr></p:bodyStyle><p:otherStyle><a:lvl1pPr><a:defRPr sz=\"1800\"/></a:lvl1pPr></p:otherStyle></p:txStyles></p:sldMaster>";

const MASTER_RELS: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout\" Target=\"../slideLayouts/slideLayout1.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout\" Target=\"../slideLayouts/slideLayout2.xml\"/><Relationship Id=\"rId3\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme\" Target=\"../theme/theme1.xml\"/></Relationships>";

const LAYOUT_ONE: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><p:sldLayout xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\" type=\"title\"><p:cSld name=\"Title Slide\"><p:spTree><p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Title 1\"/><p:cNvSpPr/><p:nvPr><p:ph type=\"ctrTitle\"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"685800\" y=\"2130425\"/><a:ext cx=\"7772400\" cy=\"1470025\"/></a:xfrm></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p/></p:txBody></p:sp><p:sp><p:nvSpPr><p:cNvPr id=\"3\" name=\"Subtitle 2\"/><p:cNvSpPr/><p:nvPr><p:ph type=\"subTitle\" idx=\"1\"/></p:nvPr></p:nvSpPr><p:spPr><a:xfrm><a:off x=\"1371600\" y=\"3886200\"/><a:ext cx=\"6400800\" cy=\"1752600\"/></a:xfrm></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p/></p:txBody></p:sp></p:spTree></p:cSld></p:sldLayout>";

const LAYOUT_TWO: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><p:sldLayout xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\" type=\"obj\"><p:cSld name=\"Title and Content\"><p:spTree><p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Title 1\"/><p:cNvSpPr/><p:nvPr><p:ph type=\"title\"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p/></p:txBody></p:sp><p:sp><p:nvSpPr><p:cNvPr id=\"3\" name=\"Content 2\"/><p:cNvSpPr/><p:nvPr><p:ph idx=\"1\"/></p:nvPr></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p/></p:txBody></p:sp></p:spTree></p:cSld></p:sldLayout>";

const THEME: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><a:theme xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" name=\"Office Theme\"><a:themeElements><a:clrScheme name=\"Office\"><a:dk1><a:sysClr val=\"windowText\" lastClr=\"000000\"/></a:dk1><a:lt1><a:sysClr val=\"window\" lastClr=\"FFFFFF\"/></a:lt1><a:dk2><a:srgbClr val=\"44546A\"/></a:dk2><a:lt2><a:srgbClr val=\"E7E6E6\"/></a:lt2><a:accent1><a:srgbClr val=\"4472C4\"/></a:accent1><a:accent2><a:srgbClr val=\"ED7D31\"/></a:accent2></a:clrScheme></a:themeElements></a:theme>";

const SLIDE_ONE: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><p:sld xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"><p:cSld><p:spTree><p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Title 1\"/><p:cNvSpPr/><p:nvPr><p:ph type=\"ctrTitle\"/></p:nvPr></p:nvSpPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang=\"en-US\" sz=\"4400\"/><a:t>Office line review</a:t></a:r></a:p></p:txBody></p:sp><p:sp><p:nvSpPr><p:cNvPr id=\"3\" name=\"Subtitle 2\"/><p:cNvSpPr/><p:nvPr><p:ph idx=\"1\"/></p:nvPr></p:nvSpPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:pPr lvl=\"0\"/><a:r><a:rPr lang=\"en-US\" sz=\"2400\"/><a:t>Revenue grew in every region.</a:t></a:r></a:p><a:p><a:pPr lvl=\"1\"/><a:r><a:rPr lang=\"en-US\" sz=\"2000\"/><a:t>Level 1 paragraph</a:t></a:r></a:p></p:txBody></p:sp><p:sp><p:nvSpPr><p:cNvPr id=\"4\" name=\"Rounded Rectangle 3\"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm rot=\"480000\"><a:off x=\"914400\" y=\"914400\"/><a:ext cx=\"3810000\" cy=\"1270000\"/></a:xfrm><a:prstGeom prst=\"roundRect\"><a:avLst/></a:prstGeom><a:solidFill><a:schemeClr val=\"accent1\"/></a:solidFill><a:ln w=\"12700\"><a:solidFill><a:srgbClr val=\"1F3864\"/></a:solidFill></a:ln></p:spPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang=\"en-US\" sz=\"2000\" b=\"1\"/><a:t>Rotated card</a:t></a:r></a:p></p:txBody></p:sp><p:pic><p:nvPicPr><p:cNvPr id=\"5\" name=\"Picture 4\"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill><a:blip r:embed=\"rId3\"/><a:stretch><a:fillRect/></a:stretch></p:blipFill><p:spPr><a:xfrm><a:off x=\"7620000\" y=\"914400\"/><a:ext cx=\"3048000\" cy=\"1905000\"/></a:xfrm></p:spPr></p:pic></p:spTree></p:cSld></p:sld>";

const SLIDE_ONE_RELS: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout\" Target=\"../slideLayouts/slideLayout1.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide\" Target=\"../notesSlides/notesSlide1.xml\"/><Relationship Id=\"rId3\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/image\" Target=\"../media/image1.png\"/></Relationships>";

const SLIDE_TWO: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><p:sld xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"><p:cSld><p:spTree><p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Title 1\"/><p:cNvSpPr/><p:nvPr><p:ph type=\"title\"/></p:nvPr></p:nvSpPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang=\"en-US\" sz=\"4400\"/><a:t>Data and charts</a:t></a:r></a:p></p:txBody></p:sp><p:sp><p:nvSpPr><p:cNvPr id=\"3\" name=\"Content 2\"/><p:cNvSpPr/><p:nvPr><p:ph idx=\"1\"/></p:nvPr></p:nvSpPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang=\"en-US\" sz=\"2400\"/><a:t>Second slide body</a:t></a:r></a:p></p:txBody></p:sp><p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id=\"4\" name=\"Revenue table\"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x=\"457200\" y=\"2200000\"/><a:ext cx=\"5080000\" cy=\"1600200\"/></p:xfrm><a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/table\"><a:tbl><a:tblPr><a:tableStyleId>{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}</a:tableStyleId></a:tblPr><a:tblGrid><a:gridCol w=\"2540000\"/><a:gridCol w=\"2540000\"/></a:tblGrid><a:tr h=\"370840\"><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:rPr lang=\"en-US\" sz=\"1800\" b=\"1\"/><a:t>Region</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:rPr lang=\"en-US\" sz=\"1800\" b=\"1\"/><a:t>Revenue</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc></a:tr><a:tr h=\"370840\"><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:rPr lang=\"en-US\" sz=\"1800\"/><a:t>North</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:rPr lang=\"en-US\" sz=\"1800\"/><a:t>42</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc></a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame><p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id=\"5\" name=\"Chart 5\"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x=\"5588000\" y=\"2200000\"/><a:ext cx=\"5588000\" cy=\"3000000\"/></p:xfrm><a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/chart\"><c:chart xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\" r:id=\"rId3\"/></a:graphicData></a:graphic></p:graphicFrame></p:spTree></p:cSld></p:sld>";

const SLIDE_TWO_RELS: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout\" Target=\"../slideLayouts/slideLayout2.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide\" Target=\"../notesSlides/notesSlide2.xml\"/><Relationship Id=\"rId3\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart\" Target=\"../charts/chart1.xml\"/></Relationships>";

/// A slide whose table body cell carries two authored paragraphs.
const SLIDE_TWO_MULTI_PARAGRAPH_CELL: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><p:sld xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"><p:cSld><p:spTree><p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Title 1\"/><p:cNvSpPr/><p:nvPr><p:ph type=\"title\"/></p:nvPr></p:nvSpPr><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr lang=\"en-US\" sz=\"4400\"/><a:t>Data and charts</a:t></a:r></a:p></p:txBody></p:sp><p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id=\"4\" name=\"Revenue table\"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x=\"457200\" y=\"2200000\"/><a:ext cx=\"5080000\" cy=\"1600200\"/></p:xfrm><a:graphic><a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/table\"><a:tbl><a:tblPr><a:tableStyleId>{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}</a:tableStyleId></a:tblPr><a:tblGrid><a:gridCol w=\"2540000\"/><a:gridCol w=\"2540000\"/></a:tblGrid><a:tr h=\"370840\"><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:rPr lang=\"en-US\" sz=\"1800\" b=\"1\"/><a:t>Region</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:rPr lang=\"en-US\" sz=\"1800\" b=\"1\"/><a:t>Revenue</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc></a:tr><a:tr h=\"370840\"><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:rPr lang=\"en-US\" sz=\"1800\"/><a:t>ALPHA_CELL_PARAGRAPH</a:t></a:r></a:p><a:p><a:r><a:rPr lang=\"en-US\" sz=\"1800\"/><a:t>BETA_CELL_PARAGRAPH</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:rPr lang=\"en-US\" sz=\"1800\"/><a:t>42</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc></a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame></p:spTree></p:cSld></p:sld>";

const NOTES_ONE: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><p:notes xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"><p:cSld><p:spTree><p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Notes\"/><p:cNvSpPr/><p:nvPr><p:ph type=\"body\" idx=\"1\"/></p:nvPr></p:nvSpPr><p:txBody><a:bodyPr/><a:p><a:r><a:rPr lang=\"en-US\"/><a:t>Speaker notes: mention the five capability states.</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:notes>";

const NOTES_TWO: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><p:notes xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"><p:cSld><p:spTree><p:sp><p:nvSpPr><p:cNvPr id=\"2\" name=\"Notes\"/><p:cNvSpPr/><p:nvPr><p:ph type=\"body\" idx=\"1\"/></p:nvPr></p:nvSpPr><p:txBody><a:bodyPr/><a:p><a:r><a:rPr lang=\"en-US\"/><a:t>Second notes page.</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:notes>";

const CHART: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><c:chartSpace xmlns:c=\"http://schemas.openxmlformats.org/drawingml/2006/chart\" xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"><c:chart><c:plotArea><c:barChart><c:barDir val=\"col\"/><c:ser><c:idx val=\"0\"/><c:tx><c:strRef><c:f>Sheet1!$B$1</c:f><c:strCache><c:pt idx=\"0\"><c:v>Revenue</c:v></c:pt></c:strCache></c:strRef></c:tx><c:cat><c:strRef><c:f>Sheet1!$A$2:$A$4</c:f><c:strCache><c:pt idx=\"0\"><c:v>North</c:v></c:pt><c:pt idx=\"1\"><c:v>South</c:v></c:pt><c:pt idx=\"2\"><c:v>East</c:v></c:pt></c:strCache></c:strRef></c:cat><c:val><c:numRef><c:f>Sheet1!$B$2:$B$4</c:f><c:numCache><c:pt idx=\"0\"><c:v>42</c:v></c:pt><c:pt idx=\"1\"><c:v>31</c:v></c:pt><c:pt idx=\"2\"><c:v>27</c:v></c:pt></c:numCache></c:numRef></c:val></c:ser></c:barChart></c:plotArea></c:chart></c:chartSpace>";

const PNG: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
    0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00, 0x01,
    0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];
