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
fn markdown_reader_lowers_inline_styles_and_links() {
    let source = "Hello **bold** and *italic* with [link](https://example.com).\n";
    let graph = read_markdown_bytes("inline.md", source.to_string()).expect("read markdown");
    let markdown = write_document_markdown(&graph).expect("write markdown");
    assert!(markdown.contains("**bold**"));
    assert!(markdown.contains("*italic*"));
    assert!(markdown.contains("[link](https://example.com)"));
}

#[test]
fn markdown_to_markdown_route() {
    let source = b"# Route\n\nBody.\n".to_vec();
    let output = convert_bytes("markdown", "markdown", "route.md", source).expect("convert");
    assert!(output.markdown.contains("# Route"));
    assert!(output.report_json.contains("panduck.report/v1"));
}

#[test]
fn markdown_to_html_route() {
    let source = b"# <Route>\n\nHello **world**.\n".to_vec();
    let output = convert_bytes("markdown", "html", "route.md", source).expect("convert");
    let html = output.html.expect("html output");
    assert!(html.contains("<h1>"));
    assert!(html.contains("&lt;Route&gt;"));
    assert!(html.contains("<strong>world</strong>"));
    assert!(output.report_json.contains("\"format\":\"html\""));
}

#[test]
fn markdown_reference_link_route_reopens_semantically_and_reports_loss() {
    let source = b"Read [guide][docs].\n\n[docs]: https://example.com/guide \"Docs\"\n".to_vec();
    let output = convert_bytes("markdown", "markdown", "reference.md", source).expect("convert");
    assert!(output.markdown.contains("[guide](https://example.com/guide)"));
    assert_eq!(output.loss_count, 1, "definition title must be reported as lossy");
    let reopened = read_markdown_bytes("reference-output.md", output.markdown.clone()).expect("reopen");
    assert!(reopened.blocks.iter().any(|block| {
        matches!(
            &block.block,
            notedown_ir::Block::Paragraph { content }
                if content.iter().any(|inline| matches!(
                    inline,
                    notedown_ir::Inline::Styled { style, children }
                        if style == "link" && children.iter().any(|child| {
                            matches!(child, notedown_ir::Inline::Text { text } if text == "https://example.com/guide")
                        })
                ))
        )
    }));
}
