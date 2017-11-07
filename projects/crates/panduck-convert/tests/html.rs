use panduck_convert::{convert_bytes, supports_route};
use notedown_formats::import::markdown::import_markdown_bytes;

#[test]
fn html_routes_are_registered_and_reopen_as_markdown() {
    assert!(supports_route("html", "markdown"));
    let output = convert_bytes(
        "html",
        "markdown",
        "source.html",
        br#"<html><body><h1>Title</h1><p>Hello <strong>world</strong>.</p></body></html>"#.to_vec(),
    ).expect("convert html");
    let reopened = import_markdown_bytes("round.md", &output.markdown).expect("reopen markdown");
    assert!(output.loss_count > 0);
    assert!(reopened.blocks.iter().any(|node| matches!(node.block, notedown_ir::Block::Section { ref title, .. } if title.iter().any(|inline| matches!(inline, notedown_ir::Inline::Text { text } if text == "Title")))));
    assert!(output.report_json.contains("success_with_loss"));
}

#[test]
fn html_routes_preserve_image_loss_as_reported_semantics() {
    let output = convert_bytes(
        "html",
        "html",
        "image.html",
        br#"<body><p><img src="picture.png" alt="Picture"></p></body>"#.to_vec(),
    ).expect("convert html");
    assert!(output.html.expect("html output").contains(r#"<img alt="Picture" src="picture.png">"#));
    assert!(output.report_json.contains("import.html.partial_coverage"));
}
