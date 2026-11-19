use document_convert::{READERS, WRITERS, convert_bytes, supported_routes, supports_route};

const SOURCES: &[&str] = &["docx", "doc", "pdf", "epub", "html", "markdown", "notedown", "tex", "typst", "xlsx", "pptx"];

#[test]
fn every_source_routes_to_deliverable_writers() {
    for from in SOURCES {
        assert!(supports_route(from, "markdown"), "{from} -> markdown");
        assert!(supports_route(from, "html"), "{from} -> html");
        assert!(supports_route(from, "epub"), "{from} -> epub");
    }
}

#[test]
fn partial_exports_keep_their_routes() {
    // The DOCX exporter refuses `Math` / `RawHtml` / opaque blocks with an
    // unsupported diagnostic per document, and the PDF writer goes through
    // `layout → presentation → renderer-pdf`, so both stay advertised.
    for from in SOURCES {
        assert!(supports_route(from, "docx"), "{from} -> docx");
        assert!(supports_route(from, "pdf"), "{from} -> pdf");
    }
}

#[test]
fn unwired_or_unknown_formats_are_rejected() {
    assert!(!supports_route("markdown", "tex"));
    assert!(!supports_route("markdown", "markdown-project"));
    // Both TeX and Typst are now readers, so their outgoing routes exist.
    assert!(supports_route("tex", "markdown"));
    assert!(supports_route("typst", "markdown"));
}

#[test]
fn tex_to_markdown_keeps_the_authored_structure() {
    let source = "\\section{Intro}\nHello \\textbf{world}.\n";
    let output = convert_bytes("tex", "markdown", "intro.tex", source.as_bytes().to_vec(), None).expect("tex to markdown");

    assert!(output.markdown.starts_with("# "), "the section became a heading: {}", output.markdown);
    assert!(output.markdown.contains("Intro"), "the heading text survived: {}", output.markdown);
    assert!(output.markdown.contains("class=\"tex-section-number\""), "the authored class survived: {}", output.markdown);
    assert!(output.markdown.contains("**world**"), "bold run was lost: {}", output.markdown);
    assert_eq!(output.loss_count, 0, "unexpected loss markers: {}", output.report_json);
}

#[test]
fn supported_routes_is_the_filtered_cross_product() {
    let routes = supported_routes();
    assert_eq!(SOURCES.len(), READERS.len(), "the source list must mirror the wired readers");
    assert_eq!(routes.len(), READERS.len() * WRITERS.len());
    assert!(routes.iter().all(|(from, to)| supports_route(from, to)));
}
