use notedown_formats::import::markdown::import_markdown_bytes;
use panduck_convert::{convert_bytes, supports_route};

mod common;
use common::{assert_convert_report, assert_report_contains_loss_codes};

#[test]
fn html_routes_are_registered_and_reopen_as_markdown() {
    assert!(supports_route("html", "markdown"));
    let output = convert_bytes(
        "html",
        "markdown",
        "source.html",
        br#"<html><body><h1>Title</h1><p>Hello <strong>world</strong>.</p></body></html>"#.to_vec(),
        None,
    )
    .expect("convert html");
    assert_convert_report(&output, "markdown");
    let reopened = import_markdown_bytes("round.md", &output.markdown).expect("reopen markdown");
    assert_report_contains_loss_codes(&output.report_json, &["import.html.partial_coverage"]);
    assert!(reopened.blocks.iter().any(|node| matches!(node.block, notedown_ir::Block::Section { ref title, .. } if title.iter().any(|inline| matches!(inline, notedown_ir::Inline::Text { text } if text == "Title")))));
}

#[test]
fn html_routes_preserve_image_loss_as_reported_semantics() {
    let output = convert_bytes(
        "html",
        "html",
        "image.html",
        br#"<body><p><img src="picture.png" alt="Picture"></p></body>"#.to_vec(),
        None,
    )
    .expect("convert html");
    assert_convert_report(&output, "html");
    assert!(output.html.expect("html output").contains(r#"<img alt="Picture" src="picture.png">"#));
    assert_report_contains_loss_codes(&output.report_json, &["import.html.partial_coverage"]);
}
