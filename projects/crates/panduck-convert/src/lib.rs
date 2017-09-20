#![warn(missing_docs)]
#![doc = include_str!("../readme.md")]

mod format_error;
mod inspect;
mod readers;
mod writers;

use std::fs;
use std::path::Path;

use notedown_ir::DocumentGraph;
use panduck_diagnostic::{diagnostics_from_graph, DiagnosticEnvelope};
use notedown_formats::import::docx::import_docx_bytes;
use notedown_formats::import::epub::import_epub_bytes;
use panduck_types::{AdapterError, Result};
use serde::Serialize;

pub use inspect::{
    inspect_docx_decode, inspect_docx_decode_bytes, inspect_docx_index, inspect_docx_index_bytes,
    DocxDecodedPart, DocxInspectDecode, DocxInspectIndex,
};
pub use readers::{read_markdown, read_markdown_bytes, read_notedown, read_notedown_bytes};
pub use writers::{write_document_docx, write_document_markdown};

/// Supported conversion route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Route {
    pub from: &'static str,
    pub to: &'static str,
}

const ROUTES: &[Route] = &[
    Route {
        from: "docx",
        to: "markdown",
    },
    Route {
        from: "docx",
        to: "docx",
    },
    Route {
        from: "markdown",
        to: "markdown",
    },
    Route {
        from: "notedown",
        to: "markdown",
    },
    Route {
        from: "epub",
        to: "markdown",
    },
    Route {
        from: "markdown",
        to: "docx",
    },
];

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
    /// Text output for text targets such as `markdown`.
    pub markdown: String,
    /// Binary output for container targets such as `docx`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary: Option<Vec<u8>>,
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
    let (markdown, binary) = write_target(&to, &graph)?;
    let loss_count = graph.coverage.loss.len();
    let diagnostic_set = diagnostics_from_graph(&graph);
    let diagnostic_envelope = DiagnosticEnvelope::from_set(&diagnostic_set);
    let report_json = serde_json::json!({
        "schema_version": "panduck.report/v1",
        "operation": "convert",
        "status": if graph.coverage.complete { "success" } else { "success_with_loss" },
        "inputs": [{ "path": label, "format": from }],
        "pipeline": { "reader": from, "writer": to, "stages": ["read", "ir", "write"] },
        "coverage": graph.coverage,
        "losses": graph.coverage.loss,
        "diagnostics": diagnostic_envelope,
        "outputs": [{ "format": to, "published": true }],
    })
    .to_string();

    Ok(ConvertOutput {
        markdown,
        binary,
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
        "docx" => import_docx_bytes(label, &bytes).map_err(crate::format_error::map_format_error),
        "epub" => import_epub_bytes(label, &bytes).map_err(crate::format_error::map_format_error),
        "markdown" => {
            let text = String::from_utf8(bytes).map_err(|error| {
                AdapterError::invalid_input(format!("markdown input is not valid UTF-8: {error}"))
            })?;
            read_markdown_bytes(label, text)
        }
        "notedown" => {
            let text = String::from_utf8(bytes).map_err(|error| {
                AdapterError::invalid_input(format!("notedown input is not valid UTF-8: {error}"))
            })?;
            read_notedown_bytes(label, text)
        }
        other => Err(AdapterError::unsupported_format(other, "read")),
    }
}

fn write_target(to: &str, graph: &DocumentGraph) -> Result<(String, Option<Vec<u8>>)> {
    match to {
        "markdown" => {
            let markdown = write_document_markdown(graph)?;
            Ok((markdown, None))
        }
        "docx" => {
            let binary = write_document_docx(graph)?;
            Ok((String::new(), Some(binary)))
        }
        other => Err(AdapterError::unsupported_format(other, "write")),
    }
}
