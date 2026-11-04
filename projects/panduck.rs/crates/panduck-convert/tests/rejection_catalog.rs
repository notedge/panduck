//! Cross-entry rejection catalog for the panduck HTML entry.
//!
//! Every expected code, severity, and span comes from
//! `davinci_types::rejections`, so this entry owns no private expectations and
//! the same rows drive the notedown, panduck, and VuTeX entries.

use std::collections::BTreeSet;

use davinci_types::{cases, code_docs};
use panduck_convert::convert_bytes;
use serde_json::Value;

#[test]
fn the_catalog_documents_the_davinci_codes_this_entry_reports() {
    let docs = code_docs();
    let documented: BTreeSet<&str> = docs.iter().map(|doc| doc.code.as_str()).collect();
    let all = cases();
    let used: BTreeSet<&str> = all.iter().map(|case| case.code.as_str()).collect();
    for code in &used {
        assert!(documented.contains(code), "every case code must be documented: `{code}`");
    }
}

#[test]
fn every_catalog_rejection_reports_the_documented_code_severity_and_span() {
    let all = cases();
    assert!(!all.is_empty(), "the catalog must carry cases");

    let mut comparisons = 0;
    for case in &all {
        let document = format!("# Catalog\n\n```{}\n{}\n```\n", case.language, case.source);
        let body_start = document.find(&case.source).unwrap_or_else(|| panic!("`{}` must sit in the fence", case.id));

        let output = convert_bytes("markdown", "html", "catalog.md", document.as_bytes().to_vec(), None)
            .unwrap_or_else(|error| panic!("`{}` must convert with a recorded loss: {error}", case.id));
        let html = output.html.as_deref().expect("html output");
        let report: Value = serde_json::from_str(&output.report_json).expect("parse report");

        assert_eq!(report["status"], "success_with_loss", "`{}`: {}", case.id, output.report_json);
        assert!(!html.contains("<figure"), "`{}`: a rejected fence keeps no figure: {html}", case.id);

        let losses = report["losses"].as_array().expect("losses array");
        let loss = losses
            .iter()
            .find(|loss| loss["code"] == case.code.as_str())
            .unwrap_or_else(|| panic!("`{}`: expected `{}` in {losses:?}", case.id, case.code));

        assert_eq!(loss["severity"], case.severity, "`{}`: severity {} but the catalog says {}", case.id, loss["severity"], case.severity);

        let span = loss.get("span").expect("loss span key");
        assert!(!span.is_null(), "`{}`: the rejection carries an authored span: {losses:?}", case.id);
        let start = span["start"].as_u64().expect("span start") as usize;
        let end = span["end"].as_u64().expect("span end") as usize;
        assert!(start >= body_start, "`{}`: the span must point inside the authored fence body at {body_start}", case.id);
        assert_eq!(&document[start..end], case.span_text, "`{}`: the span must cover the rejected authored text", case.id);
        comparisons += 1;
    }
    assert_eq!(comparisons, all.len(), "every catalog case must be compared");
}

#[test]
fn every_catalog_rejection_is_stable_across_two_runs() {
    for case in cases() {
        let document = format!("# Catalog\n\n```{}\n{}\n```\n", case.language, case.source);
        let first = convert_bytes("markdown", "html", "catalog.md", document.as_bytes().to_vec(), None).expect("first convert");
        let second =
            convert_bytes("markdown", "html", "catalog.md", document.as_bytes().to_vec(), None).expect("second convert");
        assert_eq!(first.report_json, second.report_json, "`{}`: the rejection report must be deterministic", case.id);
    }
}
