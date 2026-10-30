use panduck_convert::supports_route;

#[test]
fn doc_reader_supports_all_writers() {
    assert!(supports_route("doc", "markdown"));
    assert!(supports_route("doc", "html"));
    assert!(supports_route("doc", "docx"));
    assert!(supports_route("doc", "pdf"));
}

#[test]
fn docx_pdf_regenerate_routes_are_registered() {
    assert!(supports_route("docx", "pdf"));
    assert!(supports_route("pdf", "docx"));
}
