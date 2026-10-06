use panduck_convert::{convert_bytes, read_notedown_bytes, write_document_markdown};

#[test]
fn notedown_reader_lowers_heading_and_paragraph() {
    let source = "# Title\n\nHello world.\n";
    let graph = read_notedown_bytes("sample.nd", source.to_string()).expect("read notedown");
    assert!(graph.blocks.len() >= 2);
    let markdown = write_document_markdown(&graph).expect("write markdown");
    assert!(markdown.contains("# Title"));
}

#[test]
fn notedown_to_markdown_route() {
    let source = b"# Route\n\nBody.\n".to_vec();
    let output = convert_bytes("notedown", "markdown", "route.nd", source).expect("convert");
    assert!(output.markdown.contains("# Route"));
    assert!(output.report_json.contains("panduck.report/v1"));
}

#[test]
fn notedown_to_html_route_preserves_semantic_blocks() {
    let source = b"# Route\n\nBody text.\n".to_vec();
    let output = convert_bytes("notedown", "html", "route.nd", source).expect("convert");
    let html = output.html.expect("html output");
    assert!(html.contains("<h1>Route</h1>"));
    assert!(html.contains("<p>Body text."));
    assert!(html.contains("</p>"));
    assert!(output.report_json.contains("\"format\":\"html\""));
}
