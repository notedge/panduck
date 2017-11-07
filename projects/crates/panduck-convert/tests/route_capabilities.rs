use panduck_convert::supports_route;

#[test]
fn legacy_doc_writer_routes_remain_unavailable() {
    assert!(supports_route("doc", "markdown"));
    assert!(supports_route("doc", "html"));
    assert!(!supports_route("doc", "doc"));
    assert!(!supports_route("doc", "docx"));
    assert!(!supports_route("doc", "pdf"));
}

#[test]
fn docx_pdf_regenerate_routes_are_registered() {
    assert!(supports_route("docx", "pdf"));
    assert!(supports_route("pdf", "docx"));
}
