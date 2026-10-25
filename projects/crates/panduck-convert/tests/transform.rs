use notedown_formats::import::html::import_html_bytes;
use panduck_convert::{apply_transform_passes, convert_bytes};
use serde_json::Value;

mod common;
use common::assert_convert_report;

fn plan_json(transforms: &str) -> String {
    format!(
        r#"{{"version":"panduck-conversion-v0","id":"test-plan","input":{{"format":"html"}},"transforms":[{transforms}],"output":{{"format":"markdown"}},"publish":{{"kind":"none"}}}}"#
    )
}

fn report(output: &panduck_convert::ConvertOutput) -> Value {
    serde_json::from_str(&output.report_json).expect("parse report")
}

fn diagnostic_codes(report: &Value) -> Vec<String> {
    report["diagnostics"]["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .filter_map(|diagnostic| diagnostic["code"].as_str().map(ToString::to_string))
        .collect()
}

#[test]
fn replace_text_pass_rewrites_heading_and_paragraph() {
    let plan = plan_json(
        r#"{"name":"replace-text","description":"rewrite greeting","params":{"search":"Hello","replacement":"Bonjour"}}"#,
    );
    let output = convert_bytes(
        "html",
        "markdown",
        "greeting.html",
        br#"<html><body><h1>Hello World</h1><p>Hello there.</p></body></html>"#.to_vec(),
        Some(&plan),
    )
    .expect("convert with replace-text");

    assert_convert_report(&output, "markdown");
    assert!(output.markdown.contains("Bonjour World"), "markdown was: {}", output.markdown);
    assert!(output.markdown.contains("Bonjour there."), "markdown was: {}", output.markdown);
    assert!(!output.markdown.contains("Hello"));

    let report = report(&output);
    let transform = &report["pipeline"]["transforms"][0];
    assert_eq!(transform["name"], "replace-text");
    assert_eq!(transform["status"], "applied");
    assert_eq!(transform["changedNodes"], 2);
    assert_eq!(transform["replacedOccurrences"], 2);
    assert_eq!(transform["description"], "rewrite greeting");
    assert!(diagnostic_codes(&report).iter().any(|code| code == "panduck.transform.pass-applied"));
}

#[test]
fn remove_blocks_pass_deletes_matching_block_kind() {
    let plan = plan_json(r#"{"name":"remove-blocks","targetKinds":["quote"]}"#);
    let output = convert_bytes(
        "html",
        "markdown",
        "quote.html",
        br#"<html><body><blockquote>Quoted text.</blockquote><p>Keep me.</p></body></html>"#.to_vec(),
        Some(&plan),
    )
    .expect("convert with remove-blocks");

    assert_convert_report(&output, "markdown");
    assert!(!output.markdown.contains("Quoted text."), "markdown was: {}", output.markdown);
    assert!(output.markdown.contains("Keep me."));

    let report = report(&output);
    assert_eq!(report["pipeline"]["transforms"][0]["status"], "applied");
    assert_eq!(report["pipeline"]["transforms"][0]["changedNodes"], 1);
    assert_eq!(report["pipeline"]["transforms"][0]["targetKinds"][0], "quote");
}

#[test]
fn remove_blocks_pass_sweeps_descendants_of_removed_section() {
    use notedown_ir::{Block, DocumentGraph, DocumentId, Inline};

    let mut graph = DocumentGraph::new(DocumentId(1));
    let paragraph = graph.push_block(Block::Paragraph { content: vec![Inline::Text { text: "Nested body.".to_string() }] });
    let section = graph.push_block(Block::Section {
        level: 1,
        title: vec![Inline::Text { text: "Section title".to_string() }],
        children: vec![paragraph],
    });
    let plan = plan_json(r#"{"name":"remove-blocks","targetKinds":["section"]}"#);
    let outcome = apply_transform_passes(&mut graph, Some(&plan));

    assert_eq!(outcome.passes.len(), 1);
    assert_eq!(outcome.passes[0].status, panduck_convert::TransformPassStatus::Applied);
    assert_eq!(outcome.passes[0].changed_nodes, 1);
    assert_eq!(outcome.passes[0].swept_nodes, Some(1));
    assert!(graph.block(section).is_none());
    assert!(graph.block(paragraph).is_none());

    let markdown = notedown_formats::export::markdown::export_markdown(&graph).expect("export after sweep");
    assert!(!markdown.contains("Section title"), "markdown was: {markdown}");
    assert!(!markdown.contains("Nested body."), "markdown was: {markdown}");
}

#[test]
fn unsupported_pass_name_is_reported_and_skipped() {
    let plan = plan_json(r#"{"name":"resolve-references"}"#);
    let output = convert_bytes(
        "html",
        "markdown",
        "unsupported.html",
        br#"<html><body><p>Unchanged body.</p></body></html>"#.to_vec(),
        Some(&plan),
    )
    .expect("convert with unsupported pass");

    assert_convert_report(&output, "markdown");
    assert!(output.markdown.contains("Unchanged body."));

    let report = report(&output);
    let transform = &report["pipeline"]["transforms"][0];
    assert_eq!(transform["name"], "resolve-references");
    assert_eq!(transform["status"], "unsupported");
    assert_eq!(transform["changedNodes"], 0);
    assert!(diagnostic_codes(&report).iter().any(|code| code == "panduck.transform.unsupported-pass"));
}

#[test]
fn replace_text_without_search_is_reported_as_invalid_params() {
    let plan = plan_json(r#"{"name":"replace-text","params":{"replacement":"x"}}"#);
    let output =
        convert_bytes("html", "markdown", "invalid.html", br#"<html><body><p>Body.</p></body></html>"#.to_vec(), Some(&plan))
            .expect("convert with invalid replace-text params");

    assert_convert_report(&output, "markdown");
    let report = report(&output);
    assert_eq!(report["pipeline"]["transforms"][0]["status"], "invalid-params");
    assert!(diagnostic_codes(&report).iter().any(|code| code == "panduck.transform.invalid-params"));
}

#[test]
fn conversion_without_transforms_keeps_pipeline_and_graph_unchanged() {
    let bytes = br#"<html><body><h1>Title</h1><p>Body.</p></body></html>"#.to_vec();
    let baseline = convert_bytes("html", "markdown", "plain.html", bytes.clone(), None).expect("baseline convert");
    let empty_plan =
        convert_bytes("html", "markdown", "plain.html", bytes, Some(&plan_json(""))).expect("convert with empty pass list");

    assert_eq!(baseline.markdown, empty_plan.markdown);
    let baseline_report = report(&baseline);
    assert!(baseline_report["pipeline"].get("transforms").is_none());
    let empty_report = report(&empty_plan);
    assert!(empty_report["pipeline"].get("transforms").is_none());
}

#[test]
fn apply_transform_passes_reports_skip_when_no_text_matches() {
    let mut graph = import_html_bytes("skip.html", br#"<p>No match here.</p>"#).expect("import html");
    let plan = plan_json(r#"{"name":"replace-text","params":{"search":"Absent","replacement":"x"}}"#);
    let outcome = apply_transform_passes(&mut graph, Some(&plan));

    assert_eq!(outcome.passes.len(), 1);
    assert_eq!(outcome.passes[0].status, panduck_convert::TransformPassStatus::Skipped);
    assert_eq!(outcome.passes[0].changed_nodes, 0);
    assert!(outcome.diagnostics.is_empty());
}
