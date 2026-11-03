//! Cross-entry rejection catalog.
//!
//! The same illegal fence must produce the same `davinci.*` code and a span
//! that points inside the authored fence, no matter which entry consumes it.
//! This binary pins the catalog for the panduck HTML entry, and the sibling
//! tests in `document_pipeline` (VuTeX) and `diagram_fidelity` (notedown)
//! assert the same codes for their entries.

use panduck_convert::convert_bytes;
use serde_json::Value;

mod common;

/// One catalog row: language, illegal body, stable code, and the exact text the
/// loss span must cover.
struct Row {
    language: &'static str,
    source: &'static str,
    code: &'static str,
    span_text: &'static str,
    /// JSON path the rejection names, for languages that address structure.
    path: Option<&'static str>,
}

const CATALOG: [Row; 4] = [
    Row {
        language: "mermaid",
        source: "sequenceDiagram\n  participant Alice\n  participant Bob\n  Note over Alice,Bob: shared\n  Alice->>Bob: Hello\n",
        code: "davinci.diagram.unsupported",
        span_text: "Note over Alice,Bob: shared",
        path: None,
    },
    Row {
        language: "dot",
        source: "digraph {\n  a [label=<b>bold</b>]\n}\n",
        code: "davinci.diagram.unsupported",
        span_text: "<b>bold</b>",
        path: None,
    },
    Row {
        language: "plantuml",
        source: "@startuml\nskinparam monochrome true\n@enduml\n",
        code: "davinci.diagram.unsupported",
        span_text: "skinparam monochrome true",
        path: None,
    },
    Row {
        language: "vega-lite",
        source: r#"{"data":{"values":[{"t":1,"v":10}]},"mark":"line","encoding":{"x":{"field":"t","type":"quantitative"},"y":{"field":"v","type":"quantitative"}},"transform":[{"calculate":"1","as":"k"}]}"#,
        code: "davinci.chart.unsupported-transform",
        span_text: r#"[{"calculate":"1","as":"k"}]"#,
        path: Some("$.transform"),
    },
];

#[test]
fn every_catalog_rejection_reports_the_documented_code_and_span() {
    for row in &CATALOG {
        let document = format!("# Catalog\n\n```{}\n{}\n```\n", row.language, row.source);
        let output =
            convert_bytes("markdown", "html", "catalog.md", document.as_bytes().to_vec(), None).expect("convert catalog row");
        let html = output.html.as_deref().expect("html output");
        let report: Value = serde_json::from_str(&output.report_json).expect("parse report");

        assert_eq!(report["status"], "success_with_loss", "{}: {}", row.language, output.report_json);
        assert!(!html.contains("<figure"), "{}: a rejected fence keeps no figure: {html}", row.language);
        let escaped = row.span_text.replace('<', "&lt;").replace('>', "&gt;");
        assert!(html.contains(&escaped), "{}: the rejected text stays visible: {html}", row.language);

        let losses = report["losses"].as_array().expect("losses array");
        assert!(losses.iter().any(|loss| loss["code"] == row.code), "{}: expected {} in {losses:?}", row.language, row.code);

        let span =
            losses.iter().find(|loss| loss["code"] == row.code).and_then(|loss| loss.get("span")).expect("loss span key");
        assert!(!span.is_null(), "{}: the rejection carries an authored span: {losses:?}", row.language);
        let start = span["start"].as_u64().expect("span start") as usize;
        let end = span["end"].as_u64().expect("span end") as usize;
        assert_eq!(&document[start..end], row.span_text, "{}: the span points at the rejected text", row.language);

        let message =
            losses.iter().find(|loss| loss["code"] == row.code).and_then(|loss| loss["message"].as_str()).expect("loss message");
        if let Some(path) = row.path {
            assert!(message.contains(path), "{}: the message names `{path}`: {message}", row.language);
        }
    }
}

#[test]
fn every_catalog_rejection_is_stable_across_two_runs() {
    for row in &CATALOG {
        let document = format!("# Catalog\n\n```{}\n{}\n```\n", row.language, row.source);
        let first = convert_bytes("markdown", "html", "catalog.md", document.as_bytes().to_vec(), None).expect("first convert");
        let second =
            convert_bytes("markdown", "html", "catalog.md", document.as_bytes().to_vec(), None).expect("second convert");
        assert_eq!(first.report_json, second.report_json, "{}: the rejection report must be deterministic", row.language);
    }
}
