use std::fs;

use panduck_convert::{ConvertOutput, ConvertProjectOutput, PublishedMarkdownProject};
use serde_json::Value;

/// Assert single-file `convert` report matches IR loss and writer metadata.
pub fn assert_convert_report(output: &ConvertOutput, writer: &str) {
    let report = parse_report_json(&output.report_json);
    assert_eq!(report["schema_version"], "panduck.report/v1");
    assert_eq!(report["operation"], "convert");

    let losses = report["losses"].as_array().expect("losses array");
    let coverage_losses = report["coverage"]["loss"].as_array().expect("coverage.loss array");
    assert_eq!(output.loss_count, losses.len());
    assert_eq!(losses, coverage_losses, "report losses must mirror graph.coverage.loss");

    // status is now the AND of capability readiness and IR coverage completeness.
    // All routes currently report Partial on import/export, so we expect the
    // partial_capability suffix in practice; the important invariant is that
    // it is **not** a failure status.
    let status = report["status"].as_str().expect("status string");
    assert!(
        matches!(status, "success" | "success_with_loss" | "success_with_partial_capability" | "success_with_partial_capability_and_loss"),
        "unexpected report status {status}"
    );

    // capabilities fragment must be present.
    let caps = report["capabilities"].as_object().expect("capabilities object");
    assert!(caps.contains_key("import_format"));
    assert!(caps.contains_key("import_status"));
    assert!(caps.contains_key("export_format"));
    assert!(caps.contains_key("export_status"));

    assert_eq!(report["pipeline"]["writer"], writer);

    let outputs = report["outputs"].as_array().expect("outputs array");
    assert!(
        outputs.iter().any(|entry| entry["format"] == writer && entry["published"] == true),
        "expected published output for format {writer}, got {outputs:?}"
    );
}

pub fn stored_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut archive = Vec::new();
    let mut central = Vec::new();

    for (path, payload) in entries {
        let name = path.as_bytes();
        let crc = crc32(payload);
        let local_offset = archive.len();

        archive.extend_from_slice(b"PK\x03\x04");
        archive.extend_from_slice(&[0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        archive.extend_from_slice(&crc.to_le_bytes());
        archive.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        archive.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        archive.extend_from_slice(&(name.len() as u16).to_le_bytes());
        archive.extend_from_slice(&[0x00, 0x00]);
        archive.extend_from_slice(name);
        archive.extend_from_slice(payload);

        central.extend_from_slice(b"PK\x01\x02");
        // 42 bytes after the signature (APPNOTE central directory total = 46).
        let mut cd_fixed = [0u8; 42];
        cd_fixed[0..2].copy_from_slice(&[0x14, 0x00]);
        cd_fixed[2..4].copy_from_slice(&[0x14, 0x00]);
        cd_fixed[12..16].copy_from_slice(&crc.to_le_bytes());
        cd_fixed[16..20].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        cd_fixed[20..24].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        cd_fixed[24..26].copy_from_slice(&(name.len() as u16).to_le_bytes());
        cd_fixed[38..42].copy_from_slice(&(local_offset as u32).to_le_bytes());
        central.extend_from_slice(&cd_fixed);
        central.extend_from_slice(name);
    }

    let cd_offset = archive.len();
    archive.extend_from_slice(&central);
    let cd_size = central.len();
    archive.extend_from_slice(b"PK\x05\x06");
    archive.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);
    archive.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    archive.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    archive.extend_from_slice(&(cd_size as u32).to_le_bytes());
    archive.extend_from_slice(&(cd_offset as u32).to_le_bytes());
    archive.extend_from_slice(&[0x00, 0x00]);
    archive
}

pub fn docx_zip(document_xml: &[u8]) -> Vec<u8> {
    stored_zip(&[("word/document.xml", document_xml)])
}

pub fn docx_zip_with_footnotes(document_xml: &[u8], footnotes_xml: &[u8]) -> Vec<u8> {
    stored_zip(&[("word/document.xml", document_xml), ("word/footnotes.xml", footnotes_xml)])
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let bit = crc & 1;
            crc >>= 1;
            if bit != 0 {
                crc ^= 0xEDB8_8320;
            }
        }
    }
    crc ^ 0xFFFF_FFFF
}

/// Assert `panduck.report/v1` matches IR loss, on-disk report, and project output fields.
pub fn assert_convert_project_report(output: &ConvertProjectOutput, published: &PublishedMarkdownProject) {
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

    let unresolved = report["unresolved_assets"].as_array().expect("unresolved_assets array");
    assert_eq!(output.unresolved_assets.len(), unresolved.len(), "report unresolved_assets must mirror project export");
    for source in &output.unresolved_assets {
        assert!(unresolved.iter().any(|value| value.as_str() == Some(source)), "missing unresolved asset {source} in report");
    }

    let status = report["status"].as_str().expect("status string");
    assert!(
        matches!(status, "success" | "success_with_loss" | "success_with_partial_capability" | "success_with_partial_capability_and_loss"),
        "unexpected report status {status}"
    );

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
    let loss_codes =
        report["losses"].as_array().expect("losses array").iter().filter_map(|loss| loss["code"].as_str()).collect::<Vec<_>>();
    for code in codes {
        assert!(loss_codes.iter().any(|entry| *entry == *code), "expected loss code {code}, got {loss_codes:?}");
    }
}

fn parse_report_json(report_json: &str) -> Value {
    serde_json::from_str(report_json).expect("parse panduck.report.json")
}
