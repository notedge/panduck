use panduck_convert::convert_bytes;

mod common;
use common::assert_convert_report;

const FLOWCHART_DOC: &str = "# Diagram Doc\n\n```mermaid\nflowchart TD\n  A[Start] --> B{Done?}\n  B -->|yes| C[Stop]\n```\n\nClosing paragraph.\n";

const BROKEN_DOC: &str = "# Broken Diagram\n\n```mermaid\nthis is not a mermaid diagram at all\n```\n";

fn report_status(report_json: &str) -> String {
    let report: serde_json::Value = serde_json::from_str(report_json).expect("parse report");
    report["status"].as_str().expect("status string").to_string()
}

fn loss_codes(report_json: &str) -> Vec<String> {
    let report: serde_json::Value = serde_json::from_str(report_json).expect("parse report");
    report["losses"].as_array().expect("losses array").iter().filter_map(|loss| loss["code"].as_str().map(str::to_string)).collect()
}

#[test]
fn markdown_mermaid_renders_inline_svg_in_html_output() {
    let output = convert_bytes("markdown", "html", "diagram.md", FLOWCHART_DOC.as_bytes().to_vec(), None).expect("convert markdown to html");
    let html = output.html.as_deref().expect("html output");

    assert!(html.contains(r#"<figure class="notedown-diagram" data-diagram-language="mermaid""#), "{html}");
    assert!(html.contains("<svg"), "rendered diagram must embed real SVG: {html}");
    assert!(html.contains("data-diagram-source-hex="), "{html}");
    assert!(html.contains("Closing paragraph."), "{html}");
    assert!(!html.contains("<pre data-diagram-language"), "a rendered diagram must not keep the source fallback: {html}");
    assert_eq!(output.loss_count, 0, "a clean render must stay loss-free: {}", output.report_json);
    assert_eq!(report_status(&output.report_json), "success");
    assert_convert_report(&output, "html");
}

#[test]
fn broken_mermaid_falls_back_to_visible_source_with_structured_loss() {
    let output = convert_bytes("markdown", "html", "broken.md", BROKEN_DOC.as_bytes().to_vec(), None).expect("convert broken markdown to html");
    let html = output.html.as_deref().expect("html output");

    assert!(html.contains(r#"<pre data-diagram-language="mermaid""#), "failed renders must keep visible source: {html}");
    assert!(html.contains("this is not a mermaid diagram at all"), "{html}");
    assert!(!html.contains("<figure"), "{html}");

    let codes = loss_codes(&output.report_json);
    assert!(codes.iter().any(|code| code.starts_with("davinci.diagram.")), "expected a davinci.diagram.* loss code, got {codes:?}");
    assert_eq!(report_status(&output.report_json), "success_with_loss");
    assert!(output.loss_count > 0);
}

#[test]
fn mixed_document_renders_valid_diagrams_and_reports_broken_ones() {
    let document = "# Mixed\n\n```mermaid\nflowchart LR\n  A --> B\n```\n\n```mermaid\nnot mermaid at all\n```\n";
    let output = convert_bytes("markdown", "html", "mixed.md", document.as_bytes().to_vec(), None).expect("convert mixed document");
    let html = output.html.as_deref().expect("html output");

    assert_eq!(html.matches("<figure class=\"notedown-diagram\"").count(), 1, "{html}");
    assert_eq!(html.matches("<pre data-diagram-language=\"mermaid\"").count(), 1, "{html}");
    assert_eq!(report_status(&output.report_json), "success_with_loss");
}

#[test]
fn mermaid_html_output_is_byte_stable() {
    let first = convert_bytes("markdown", "html", "diagram.md", FLOWCHART_DOC.as_bytes().to_vec(), None).expect("first convert");
    let second = convert_bytes("markdown", "html", "diagram.md", FLOWCHART_DOC.as_bytes().to_vec(), None).expect("second convert");
    assert_eq!(first.html, second.html, "the same input must render byte-identical HTML");
}

#[test]
fn markdown_target_keeps_the_authored_fence_without_rendering() {
    let output = convert_bytes("markdown", "markdown", "diagram.md", FLOWCHART_DOC.as_bytes().to_vec(), None).expect("convert markdown to markdown");
    assert!(output.markdown.contains("```mermaid\nflowchart TD"), "{}", output.markdown);
    assert!(output.markdown.contains("B -->|yes| C[Stop]"), "{}", output.markdown);
    assert_eq!(output.loss_count, 0, "{}", output.report_json);
}

#[test]
fn markdown_mermaid_survives_the_docx_round_trip() {
    let to_docx = convert_bytes("markdown", "docx", "diagram.md", FLOWCHART_DOC.as_bytes().to_vec(), None).expect("markdown to docx");
    let docx = to_docx.binary.expect("docx bytes");
    let back = convert_bytes("docx", "markdown", "diagram.docx", docx.to_vec(), None).expect("docx back to markdown");
    assert!(back.markdown.contains("```mermaid"), "{}", back.markdown);
    assert!(back.markdown.contains("A[Start] --> B{Done?}"), "{}", back.markdown);
}
