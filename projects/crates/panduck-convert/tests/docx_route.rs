use panduck_convert::convert_bytes;

#[test]
fn markdown_to_docx_route() {
    let source = b"# Title\n\nHello world.\n".to_vec();
    let output = convert_bytes("markdown", "docx", "sample.md", source).expect("convert");
    assert!(output.markdown.is_empty());
    let binary = output.binary.expect("docx bytes");
    assert!(binary.starts_with(b"PK\x03\x04"));
    assert!(output.report_json.contains("panduck.report/v1"));
    assert!(output.report_json.contains("docx"));
}
