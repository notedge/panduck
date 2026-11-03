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

    assert!(html.contains(r#"<figure class="diagram" data-diagram-language="mermaid""#), "{html}");
    assert!(html.contains("<svg"), "rendered diagram must embed real SVG: {html}");
    assert!(html.contains("data-diagram-source-hex="), "{html}");
    assert!(html.contains("Closing paragraph."), "{html}");
    assert!(!html.contains(r#"<pre class="diagram" data-diagram-language"#), "a rendered diagram must not keep the source fallback: {html}");
    assert_eq!(output.loss_count, 0, "a clean render must stay loss-free: {}", output.report_json);
    assert_eq!(report_status(&output.report_json), "success");
    assert_convert_report(&output, "html");
}

#[test]
fn broken_mermaid_falls_back_to_visible_source_with_structured_loss() {
    let output = convert_bytes("markdown", "html", "broken.md", BROKEN_DOC.as_bytes().to_vec(), None).expect("convert broken markdown to html");
    let html = output.html.as_deref().expect("html output");

    assert!(html.contains(r#"<pre class="diagram" data-diagram-language="mermaid""#), "failed renders must keep visible source: {html}");
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

    assert_eq!(html.matches(r#"<figure class="diagram""#).count(), 1, "{html}");
    assert_eq!(html.matches(r#"<pre class="diagram" data-diagram-language="mermaid""#).count(), 1, "{html}");
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

#[test]
fn multiple_identical_diagrams_get_unique_namespaced_ids_and_scoped_classes() {
    let document = "# Two Diagrams\n\n```mermaid\nflowchart LR\n  A --> B\n```\n\n```mermaid\nflowchart LR\n  A --> B\n```\n";
    let output = convert_bytes("markdown", "html", "two.md", document.as_bytes().to_vec(), None).expect("convert two diagrams");
    let html = output.html.as_deref().expect("html output");

    assert_eq!(html.matches(r#"<figure class="diagram""#).count(), 2, "{html}");
    assert_eq!(html.matches(r#"<svg xmlns="http://www.w3.org/2000/svg" class="diagram""#).count(), 2, "{html}");
    assert!(html.contains(r#"class="diagram-node""#), "{html}");
    assert!(html.contains(r#"class="diagram-edge""#), "{html}");

    // Every id in the document must be unique even though both diagrams
    // authored the same node and edge identities.
    let ids: Vec<&str> = html
        .match_indices(" id=\"")
        .map(|(start, _)| {
            let rest = &html[start + 5..];
            &rest[..rest.find('"').expect("closing quote")]
        })
        .collect();
    let mut unique = ids.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(ids.len(), unique.len(), "duplicate ids across diagrams: {ids:?}");
    let diagram_ids: Vec<&&str> = ids.iter().filter(|id| !id.starts_with("node-")).collect();
    assert!(diagram_ids.iter().all(|id| id.starts_with("diagram-")), "diagram ids carry the page namespace: {ids:?}");

    // No bare generic class names may leak into the host page.
    for bare in ["class=\"node\"", "class=\"edge\"", "class=\"label\"", "class=\"cluster\""] {
        assert!(!html.contains(bare), "bare class `{bare}` must not appear: {html}");
    }
}

#[test]
fn puml_alias_reaches_the_plantuml_lowering() {
    use notedown_ir::{Block, DocumentGraph, DocumentId};

    let source = "@startuml\nclass Parser\nclass Renderer\nParser <|-- Renderer\n@enduml\n";
    let mut graph = DocumentGraph::new(DocumentId(1));
    graph.push_block(Block::Diagram { language: "puml".to_string(), source: source.to_string() });

    let rendered = panduck_convert::diagram::render_diagrams(&graph);
    assert!(rendered.losses.is_empty(), "the alias must lower cleanly: {:?}", rendered.losses);
    assert_eq!(rendered.scenes.len(), 1, "the alias must render exactly one scene");

    let scene = rendered.scenes.values().next().expect("rendered scene");
    assert!(scene.svg.contains(r#"class="diagram""#), "{}", scene.svg);
    assert!(scene.svg.contains(r#"data-role="node""#), "{}", scene.svg);
    assert!(scene.svg.contains(r#"data-role="edge""#), "the inheritance edge must render: {}", scene.svg);
}
