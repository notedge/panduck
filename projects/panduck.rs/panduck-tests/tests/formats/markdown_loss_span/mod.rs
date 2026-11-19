//! Loss markers recorded by the Markdown collector must keep their syntax site.

use document_formats::import::markdown::import_markdown_bytes;

#[test]
fn unresolved_reference_link_keeps_its_syntactic_span() {
    let source = "Read [the guide][missing] now.\n";
    let graph = import_markdown_bytes("inline.md", source).expect("markdown imports");
    let index = graph
        .coverage
        .loss
        .iter()
        .position(|loss| loss.code == "import.markdown.unresolved_reference_link")
        .expect("an explicit unresolved label is recorded");
    let span = graph.loss_span(index).expect("an explicit unresolved loss must carry its span");
    let slice = &source[span.start as usize..span.end as usize];
    assert!(slice.contains("[the guide][missing]"), "span covers {slice:?}");
}
