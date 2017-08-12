use panduck_convert::{convert_bytes, read_markdown_bytes, write_document_markdown};

#[test]
fn markdown_reader_lowers_heading_and_paragraph() {
    let source = "# Title\n\nHello world.\n";
    let graph = read_markdown_bytes("sample.md", source.to_string()).expect("read markdown");
    assert_eq!(graph.blocks.len(), 2);
    let markdown = write_document_markdown(&graph).expect("write markdown");
    assert!(markdown.contains("# Title"));
    assert!(markdown.contains("Hello world."));
}

#[test]
fn markdown_to_markdown_route() {
    let source = b"# Route\n\nBody.\n".to_vec();
    let output = convert_bytes("markdown", "markdown", "route.md", source).expect("convert");
    assert!(output.markdown.contains("# Route"));
    assert!(output.report_json.contains("panduck.report/v1"));
}
