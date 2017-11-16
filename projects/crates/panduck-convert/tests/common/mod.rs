use std::fs;

use panduck_convert::{ConvertProjectOutput, PublishedMarkdownProject};
use serde_json::Value;

/// Assert `panduck.report/v1` matches IR loss, on-disk report, and project output fields.
pub fn assert_convert_project_report(
    output: &ConvertProjectOutput,
    published: &PublishedMarkdownProject,
) {
    let report = parse_report_json(&output.report_json);
    let disk = fs::read_to_string(&published.report_path).expect("read panduck.report.json");
    let disk_report = parse_report_json(&disk);
    assert_eq!(report, disk_report, "in-memory and on-disk report must match");

    assert_eq!(report["schema_version"], "panduck.report/v1");
    assert_eq!(report["operation"], "convert_project");

    let losses = report["losses"].as_array().expect("losses array");
    let coverage_losses = report["coverage"]["loss"].as_array().expect("coverage.loss array");
    assert_eq!(output.loss_count, losses.len());
    assert_eq!(losses, coverage_losses, "report losses must mirror graph.coverage.loss");

    let unresolved = report["unresolved_assets"]
        .as_array()
        .expect("unresolved_assets array");
    assert_eq!(
        output.unresolved_assets.len(),
        unresolved.len(),
        "report unresolved_assets must mirror project export"
    );
    for source in &output.unresolved_assets {
        assert!(
            unresolved.iter().any(|value| value.as_str() == Some(source)),
            "missing unresolved asset {source} in report"
        );
    }

    let expected_status = if losses.is_empty() { "success" } else { "success_with_loss" };
    assert_eq!(report["status"].as_str(), Some(expected_status));

    let outputs = report["outputs"].as_array().expect("outputs array");
    assert!(outputs.iter().any(|entry| entry["path"] == "index.md"));
    assert!(outputs.iter().any(|entry| entry["path"] == "panduck.report.json"));

    let published_asset_count = outputs
        .iter()
        .find(|entry| entry["path"] == "assets/")
        .and_then(|entry| entry["published_asset_count"].as_u64())
        .expect("assets output summary");
    assert_eq!(published_asset_count as usize, output.published_assets.len());
    assert_eq!(published.asset_paths.len(), output.published_assets.len());
    assert_eq!(published.chapter_paths.len(), output.published_chapters.len());
}

/// Assert the report lists every expected loss code at least once.
pub fn assert_report_contains_loss_codes(report_json: &str, codes: &[&str]) {
    let report = parse_report_json(report_json);
    let loss_codes = report["losses"]
        .as_array()
        .expect("losses array")
        .iter()
        .filter_map(|loss| loss["code"].as_str())
        .collect::<Vec<_>>();
    for code in codes {
        assert!(
            loss_codes.iter().any(|entry| *entry == *code),
            "expected loss code {code}, got {loss_codes:?}"
        );
    }
}

fn parse_report_json(report_json: &str) -> Value {
    serde_json::from_str(report_json).expect("parse panduck.report.json")
}
