use notedown_formats::import::docx::import_docx_bytes;
use notedown_ir::{Block, Inline, RelationEndpoint};
use panduck_convert::convert_bytes;

mod common;
use common::assert_convert_report;

#[test]
fn markdown_to_docx_route() {
    let source = b"# Title\n\nHello world.\n".to_vec();
    let output = convert_bytes("markdown", "docx", "sample.md", source, None).expect("convert");
    assert!(output.markdown.is_empty());
    assert_convert_report(&output, "docx");
    let binary = output.binary.expect("docx bytes");
    assert!(binary.starts_with(b"PK\x03\x04"));
    let reopened = import_docx_bytes("round.docx", &binary).expect("reopen generated docx");
    assert!(reopened.validate().is_valid());
    assert!(
        reopened.blocks.iter().any(|node| matches!(
            &node.block,
            Block::Section { title, .. }
                if title.iter().any(|inline| matches!(inline, Inline::Text { text } if text == "Title"))
        )),
        "reopened blocks: {:?}",
        reopened.blocks
    );
    assert!(reopened.blocks.iter().any(|node| matches!(
        &node.block,
        Block::Paragraph { content }
            if content.iter().any(|inline| matches!(inline, Inline::Text { text } if text.trim_end() == "Hello world.")
        )
    )));
}

#[test]
fn markdown_shortcut_link_to_docx_reopens_with_same_target() {
    let source = b"Read [Guide] now.\n\n[guide]: https://example.com/guide\n".to_vec();
    let output = convert_bytes("markdown", "docx", "shortcut.md", source, None).expect("convert");
    assert_convert_report(&output, "docx");
    assert_eq!(output.loss_count, 0);
    let binary = output.binary.expect("docx bytes");
    let reopened = import_docx_bytes("shortcut.docx", &binary).expect("reopen");
    assert!(reopened.validate().is_valid());
    assert!(
        reopened.blocks.iter().any(|node| matches!(
            &node.block,
            Block::Paragraph { content } if content.iter().any(|inline| matches!(
                inline,
                Inline::Link { target, display, .. }
                    if target == &RelationEndpoint::ExternalUri("https://example.com/guide".into())
                        && display == &[Inline::Text { text: "Guide".into() }]
            ))
        )),
        "{:?}",
        reopened.blocks
    );
}

#[test]
fn docx_to_html_route_preserves_reopened_semantics() {
    let source = convert_bytes("markdown", "docx", "source.md", b"# Title\n\nBody.\n".to_vec(), None)
        .expect("create source docx")
        .binary
        .expect("source docx bytes");
    let output = convert_bytes("docx", "html", "source.docx", source, None).expect("convert docx");
    assert_convert_report(&output, "html");
    let html = output.html.expect("html output");
    assert!(html.contains("<h1>Title</h1>"));
    assert!(html.contains("<p>"));
    assert!(html.contains("Body."));
}
