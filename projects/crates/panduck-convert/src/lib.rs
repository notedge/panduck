#![warn(missing_docs)]
//! Panduck conversion routes over `notedown-ir::DocumentGraph`.

use std::fs;
use std::path::Path;

use notedown_ir::DocumentGraph;
use panduck_docx::read_docx_bytes;
use panduck_markdown::write_document_markdown;
use panduck_types::{AdapterError, Result};
use serde::Serialize;

/// Supported conversion route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Route {
    pub from: &'static str,
    pub to: &'static str,
}

const ROUTES: &[Route] = &[Route {
    from: "docx",
    to: "markdown",
}];

/// Returns whether Panduck can run this conversion today.
pub fn supports_route(from: &str, to: &str) -> bool {
    let from = from.to_ascii_lowercase();
    let to = to.to_ascii_lowercase();
    ROUTES.iter().any(|route| route.from == from && route.to == to)
}

/// Lists wired conversion routes.
pub fn supported_routes() -> Vec<(String, String)> {
    ROUTES
        .iter()
        .map(|route| (route.from.to_string(), route.to.to_string()))
        .collect()
}

/// Conversion output payload and report metadata.
#[derive(Debug, Clone, Serialize)]
pub struct ConvertOutput {
    pub markdown: String,
    pub report_json: String,
    pub loss_count: usize,
}

/// Converts bytes from `from` format to `to` format.
pub fn convert_bytes(from: &str, to: &str, label: &str, bytes: Vec<u8>) -> Result<ConvertOutput> {
    let from = from.to_ascii_lowercase();
    let to = to.to_ascii_lowercase();
    if from == "doc" && to == "markdown" {
        return Err(AdapterError::not_implemented(
            "legacy .doc import requires OLE reader support; use .docx",
        ));
    }
    if !supports_route(&from, &to) {
        return Err(AdapterError::unsupported_format(from, format!("convert to {to}")));
    }

    let graph = read_source(&from, label, bytes)?;
    let markdown = write_document_markdown(&graph)?;
    let loss_count = graph.coverage.loss.len();
    let report_json = serde_json::json!({
        "schema_version": "panduck.report/v1",
        "operation": "convert",
        "status": if graph.coverage.complete { "success" } else { "success_with_loss" },
        "inputs": [{ "path": label, "format": from }],
        "pipeline": { "reader": from, "writer": to, "stages": ["read", "ir", "write"] },
        "losses": graph.coverage.loss,
        "outputs": [{ "format": to, "published": true }],
    })
    .to_string();

    Ok(ConvertOutput {
        markdown,
        report_json,
        loss_count,
    })
}

/// Converts a file on disk.
pub fn convert_file(from: &str, to: &str, input: impl AsRef<Path>) -> Result<ConvertOutput> {
    let input = input.as_ref();
    let bytes = fs::read(input).map_err(|source| {
        AdapterError::io(source, Some(input.display().to_string()))
    })?;
    convert_bytes(from, to, &input.display().to_string(), bytes)
}

fn read_source(from: &str, label: &str, bytes: Vec<u8>) -> Result<DocumentGraph> {
    match from {
        "docx" => read_docx_bytes(label, bytes),
        other => Err(AdapterError::unsupported_format(other, "read")),
    }
}
