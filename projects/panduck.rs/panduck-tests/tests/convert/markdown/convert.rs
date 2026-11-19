use document_convert::{convert_bytes, read_markdown_bytes, write_document_markdown};

#[test]
fn markdown_reader_lowers_heading_and_paragraph() {
    let source = "# Title\n\nHello world.\n";
    let mut graph = read_markdown_bytes("sample.md", source.to_string()).expect("read markdown");
    assert_eq!(graph.blocks.len(), 2);
    let markdown = write_document_markdown(&mut graph).expect("write markdown");
    assert!(markdown.contains("# Title"));
    assert!(markdown.contains("Hello world."));
}

#[test]
fn markdown_reader_lowers_inline_styles_and_links() {
    let source = "Hello **bold** and *italic* with [link](https://example.com).\n";
    let mut graph = read_markdown_bytes("inline.md", source.to_string()).expect("read markdown");
    let markdown = write_document_markdown(&mut graph).expect("write markdown");
    assert!(markdown.contains("**bold**"));
    assert!(markdown.contains("*italic*"));
    assert!(markdown.contains("[link](https://example.com)"));
}

#[test]
fn markdown_to_markdown_route() {
    let source = b"# Route\n\nBody.\n".to_vec();
    let output = convert_bytes("markdown", "markdown", "route.md", source, None).expect("convert");
    assert!(output.markdown.contains("# Route"));
    assert!(output.report_json.contains(document_convert::SCHEMA_REPORT));
}

#[test]
fn markdown_to_html_route_keeps_authored_inline_html() {
    let source = b"# <Route>\n\nHello **world**.\n\nInline <b>markup</b> stays verbatim.\n".to_vec();
    let output = convert_bytes("markdown", "html", "route.md", source, None).expect("convert");
    let html = output.html.expect("html output");
    assert!(html.contains("<h1><Route></h1>"), "{html}");
    assert!(html.contains("<strong>world</strong>"), "{html}");
    assert!(html.contains("<b>markup</b>"), "{html}");
    assert!(output.report_json.contains("\"format\":\"html\""));
}

#[test]
fn markdown_reference_link_route_preserves_title_without_loss() {
    let source = b"Read [guide][docs].\n\n[docs]: https://example.com/guide \"Docs\"\n".to_vec();
    let output = convert_bytes("markdown", "markdown", "reference.md", source, None).expect("convert");
    assert!(output.markdown.contains("[guide](https://example.com/guide \"Docs\")"));
    assert_eq!(output.loss_count, 0, "resolved reference definition title is preserved in IR");
    let reopened = read_markdown_bytes("reference-output.md", output.markdown.clone()).expect("reopen");
    assert!(reopened.blocks.iter().any(|block| {
        matches!(
            &block.block,
            document_ir::Block::Paragraph { content }
                if content.iter().any(|inline| matches!(
                    inline,
                    document_ir::Inline::Link { target, display, .. }
                        if target == &document_ir::RelationEndpoint::ExternalUri(
                            "https://example.com/guide".into()
                        )
                            && display.iter().any(|child| {
                                matches!(child, document_ir::Inline::Text { text } if text == "guide")
                            })
                ))
        )
    }));
}
