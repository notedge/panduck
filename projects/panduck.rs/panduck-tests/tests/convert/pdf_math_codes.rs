//! Math content codes: the PDF report keeps the losses that change what the
//! page shows, and leaves environment warnings to the logger.
//!
//! The layout pass serves only the PDF writer, so this test needs its own
//! binary: the math install is process-wide and runs once, and the first half
//! proves the code that fires before any host math pack is installed.

#[path = "common/mod.rs"]
mod common;

use std::sync::Arc;

use document_convert::write_document_pdf;
use document_fonts::{BodyFace, BodyFaceSet, BodyFaceStyle, BodyFamily, BodyMetrics, BodyScript, MathFontPack, install_math_fonts};
use document_formats::lower_tex_formula_source;
use document_ir::{Block, DocumentGraph, DocumentId, Inline, MathDisplay};
use document_layout::{Diagrams, LayoutHint};

use common::pdf_body_faces;

fn formula_graph(source: &str) -> DocumentGraph {
    let mut graph = DocumentGraph::new(DocumentId(99));
    graph.push_block(Block::Math { formula: lower_tex_formula_source(source, MathDisplay::Display) });
    graph
}

fn loss_codes(graph: &DocumentGraph) -> Vec<String> {
    graph.coverage.loss.iter().map(|loss| loss.code.clone()).collect()
}

fn assert_reported(graph: &DocumentGraph, expected: &str) {
    let codes = loss_codes(graph);
    assert!(codes.iter().any(|code| code == expected), "expected {expected} in the report: {codes:?}");
}

/// The layout pass must emit `code`, and the report must not carry it.
///
/// Both halves matter: if the diagnostic stops firing the first assert fails,
/// and if a later wiring change starts pushing it as a loss the second fails.
fn assert_logger_only(graph: &DocumentGraph, faces: &BodyFaceSet, code: &str) {
    let metrics = BodyMetrics::from_set(faces);
    let (_, diagnostics) = document_layout::document::layout_with_metrics(graph, &LayoutHint::a4(), &Diagrams::new(), &metrics);
    let emitted: Vec<String> = diagnostics.into_iter().map(|diagnostic| diagnostic.code).collect();
    assert!(emitted.iter().any(|item| item == code), "the layout pass must emit {code}: {emitted:?}");
    let reported = loss_codes(graph);
    assert!(!reported.iter().any(|item| item == code), "{code} must stay logger-only, report has {reported:?}");
}

#[test]
fn pdf_report_keeps_every_math_content_loss_and_drops_environment_warnings() {
    let faces = pdf_body_faces();

    // A paragraph carries no math, so nothing is reported for it.
    let mut plain = DocumentGraph::new(DocumentId(99));
    plain.push_block(Block::Paragraph { content: vec![Inline::Text { text: "Hello".into() }] });
    write_document_pdf(&mut plain, &faces).expect("PDF write");
    assert!(loss_codes(&plain).is_empty(), "{:?}", loss_codes(&plain));

    // A face the metric reader cannot parse is an environment fact. The layout
    // pass reports it, the renderer skips that face, and the text still paints,
    // so the report must stay clean. The damaged face takes the `Mono/Latin/Bold`
    // triple, which the pack does not carry, because a repeated registration is
    // rejected at construction.
    let mut broken_faces = faces.faces().to_vec();
    broken_faces.push(BodyFace::new(BodyFamily::Mono, BodyScript::Latin, BodyFaceStyle::Bold, vec![0u8; 32]));
    let damaged = Arc::new(BodyFaceSet::new(broken_faces).expect("distinct face triples"));
    let mut readable = DocumentGraph::new(DocumentId(99));
    readable.push_block(Block::Paragraph { content: vec![Inline::Text { text: "Hello".into() }] });
    write_document_pdf(&mut readable, &damaged).expect("PDF write with a damaged face");
    assert_logger_only(&readable, damaged.as_ref(), "vutex.layout.metrics.face_unreadable");

    // No math pack is installed in this binary, so the engine cannot typeset a
    // formula and the page keeps the authored source instead.
    let mut graph = formula_graph("x^2");
    write_document_pdf(&mut graph, &faces).expect("PDF write keeps the source fallback");
    assert_reported(&graph, "vutex.layout.math.not_typeset");

    let pack = MathFontPack::load_from(&common::host_font_dir().expect("host font pack")).expect("host math pack");
    install_math_fonts(&pack);

    // A clean formula now typesets, so the report stays clean. The metric and
    // approximation warnings the pass emits along the way are logger-only.
    let mut graph = formula_graph("x^2");
    write_document_pdf(&mut graph, &faces).expect("PDF write");
    assert!(loss_codes(&graph).is_empty(), "{:?}", loss_codes(&graph));

    // An unknown command keeps a placeholder glyph, so the expression and the
    // missing outline are both content losses the report must carry.
    let mut graph = formula_graph("\\unknowncmd{1}");
    write_document_pdf(&mut graph, &faces).expect("PDF write");
    assert_reported(&graph, "vutex.math.unsupported_expression");
    assert_reported(&graph, "vutex.layout.math.outline_missing");

    // `\limits` on a plain atom keeps its scripts beside the base. That is an
    // approximation of the request, not content removed from the page.
    let mut limits = formula_graph("x\\limits_1");
    write_document_pdf(&mut limits, &faces).expect("PDF write");
    assert_logger_only(&limits, &faces, "vutex.layout.math.limits_misuse");

    // Conformance fallbacks substitute font data. The layout ledger records the
    // missing glyph, but the character is still on the page.
    let mut fallback = formula_graph("\\unknowncmd{1}");
    write_document_pdf(&mut fallback, &faces).expect("PDF write");
    assert_logger_only(&fallback, &faces, "vutex.math.conformance.missing_glyph");
}
