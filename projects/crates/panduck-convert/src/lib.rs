#![deny(missing_docs)]
#![doc = include_str!("readme.md")]

mod dedupe_assets;
mod format_error;
mod inspect;
mod publish;
mod readers;
pub mod transform;
mod writers;

use std::{fs, path::Path};

use boost_diagnostic::{
    Diagnostic, DiagnosticCode, DiagnosticOrigin, DiagnosticSeverity, Message,
};
use notedown_formats::import::{
    doc::import_doc_bytes, docx::import_docx_bytes, epub::import_epub_bytes, html::import_html_bytes, pdf::import_pdf_bytes,
};
use notedown_ir::DocumentGraph;
use panduck_diagnostic::diagnostics_from_graph;
use panduck_types::{AdapterError, Result};
use serde::Serialize;

/// Re-export of `panduck-diagnostic` envelope type so downstream NAPI consumers
/// can build diagnostics-aligned reports without depending on the diagnostic
/// crate directly.
pub use panduck_diagnostic::{DiagnosticEnvelope, SCHEMA_REPORT, from_adapter_error};

/// Build an empty diagnostic envelope for operations that completed cleanly.
pub fn empty_diagnostic_envelope() -> DiagnosticEnvelope {
    use boost_diagnostic::DiagnosticSet;
    DiagnosticEnvelope::from_set(&DiagnosticSet::new())
}

pub use inspect::{
    DocxDecodedPart, DocxInspectDecode, DocxInspectIndex, inspect_docx_decode, inspect_docx_decode_bytes, inspect_docx_index,
    inspect_docx_index_bytes,
};
pub use publish::{PublishedMarkdownProject, publish_bytes, publish_markdown_project, publish_text};
pub use transform::{TransformOutcome, TransformPassReport, TransformPassStatus, apply_transform_passes};
pub use readers::{
    read_doc, read_doc_bytes, read_html, read_html_bytes, read_markdown, read_markdown_bytes, read_notedown,
    read_notedown_bytes, read_pdf, read_pdf_bytes,
};
pub use writers::{
    write_document_docx, write_document_html, write_document_markdown, write_document_pdf, write_markdown_project,
};

/// Reader formats wired into this build (lowercase).
pub const READERS: &[&str] = &["docx", "doc", "pdf", "epub", "html", "markdown", "notedown"];

/// Writer formats wired into this build (lowercase).
pub const WRITERS: &[&str] = &["markdown", "docx", "pdf", "html"];

/// Whether Panduck has a reader for `from` and a writer for `to`.
///
/// Pure capability check — a route is supported iff both sides are wired
/// with real implementations. No runtime registry, no partial status, no
/// transitional "planned" markers.
pub fn supports_route(from: &str, to: &str) -> bool {
    let from = from.to_ascii_lowercase();
    let to = to.to_ascii_lowercase();
    READERS.contains(&from.as_str()) && WRITERS.contains(&to.as_str())
}

/// Lists wired conversion routes: every (reader, writer) pair.
pub fn supported_routes() -> Vec<(String, String)> {
    let mut routes = Vec::with_capacity(READERS.len() * WRITERS.len());
    for reader in READERS {
        for writer in WRITERS {
            routes.push((reader.to_string(), writer.to_string()));
        }
    }
    routes
}

/// Markdown project conversion output.
#[derive(Debug, Clone, Serialize)]
pub struct ConvertProjectOutput {
    /// Primary Markdown body (`index.md` content).
    pub index_markdown: String,
    /// Project-relative asset paths that were materialized.
    pub published_assets: Vec<String>,
    /// Project-relative chapter paths that were materialized.
    pub published_chapters: Vec<String>,
    /// Image sources left unresolved in the Markdown body.
    pub unresolved_assets: Vec<String>,
    /// Serialized `panduck.report/v0` JSON payload.
    pub report_json: String,
    /// Number of semantic loss markers recorded in the report.
    pub loss_count: usize,
}

/// Conversion output payload and report metadata.
#[derive(Debug, Clone, Serialize)]
pub struct ConvertOutput {
    /// Text output for text targets such as `markdown`.
    pub markdown: String,
    /// HTML output for the `html` target.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub html: Option<String>,
    /// Binary output for container targets such as `docx`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary: Option<Vec<u8>>,
    /// Serialized `panduck.report/v0` JSON payload.
    pub report_json: String,
    /// Number of semantic loss markers recorded in the report.
    pub loss_count: usize,
}

/// Converts bytes from `from` format to `to` format.
///
/// `handlers_json` is an optional JSON-serialized plan descriptor from the
/// TS-side `ConversionPlan`. When provided, its handlerKeys and step structure are
/// forwarded into the report `pipeline.handlers` field so downstream consumers
/// can see which custom `.replace()` handlers were registered for this run.
/// The Rust IR pipeline does NOT execute handler closures (they are TS-only),
/// but the parameter channel is fully open — this is the wire-level bridge
/// that lets a future Rust-native handler dispatch host consume the same
/// descriptor.
pub fn convert_bytes(
    from: &str,
    to: &str,
    label: &str,
    bytes: Vec<u8>,
    handlers_json: Option<&str>,
) -> Result<ConvertOutput> {
    let from = from.to_ascii_lowercase();
    let to = to.to_ascii_lowercase();
    if !supports_route(&from, &to) {
        return format_error::fail(AdapterError::unsupported_format(from, format!("convert to {to}")));
    }

    let mut graph = format_error::propagate(read_source(&from, label, bytes))?;
    let transform_outcome = apply_transform_passes(&mut graph, handlers_json);
    let (markdown, html, binary) = format_error::propagate(write_target(&to, &graph))?;
    let loss_count = graph.coverage.loss.len();
    let mut diagnostic_set = diagnostics_from_graph(&graph);
    for diagnostic in &transform_outcome.diagnostics {
        diagnostic_set.push(diagnostic.clone());
    }

    // Handler registry bridge — parse the TS-side descriptor, forward keys
    // into the report pipeline, and emit an info diagnostic when present.
    let (pipeline_handlers, handler_keys) = parse_handlers_descriptor(handlers_json);
    if !handler_keys.is_empty() {
        diagnostic_set.push(build_handler_info_diagnostic(&handler_keys));
    }

    if loss_count > 0 {
        panduck_diagnostic::log_diagnostic_set(&diagnostic_set);
    }
    let diagnostic_envelope = DiagnosticEnvelope::from_set(&diagnostic_set);
    let mut pipeline = serde_json::json!({ "reader": from, "writer": to, "stages": ["read", "ir", "write"] });
    if !transform_outcome.is_empty() {
        pipeline["transforms"] = serde_json::to_value(&transform_outcome.passes).unwrap_or_default();
    }
    if let Some(handlers_val) = pipeline_handlers {
        pipeline["handlers"] = handlers_val;
    }
    let report_json = serde_json::json!({
        "schema_version": panduck_diagnostic::SCHEMA_REPORT,
        "operation": "convert",
        "status": if loss_count == 0 { "success" } else { "success_with_loss" },
        "inputs": [{ "path": label, "format": from }],
        "pipeline": pipeline,
        "coverage": graph.coverage,
        "losses": graph.coverage.loss,
        "diagnostics": diagnostic_envelope,
        "outputs": [{ "format": to, "published": true }],
    })
    .to_string();

    Ok(ConvertOutput { markdown, html, binary, report_json, loss_count })
}

/// Converts bytes into a Markdown project description without writing disk.
pub fn convert_to_markdown_project_bytes(
    from: &str,
    label: &str,
    bytes: Vec<u8>,
    handlers_json: Option<&str>,
) -> Result<ConvertProjectOutput> {
    let from = from.to_ascii_lowercase();
    if !supports_markdown_project_source(&from) {
        return format_error::fail(AdapterError::unsupported_format(from, "markdown project"));
    }

    let mut graph = format_error::propagate(read_source(&from, label, bytes))?;
    let transform_outcome = apply_transform_passes(&mut graph, handlers_json);
    let mut project = format_error::propagate(write_markdown_project(&graph))?;
    dedupe_assets::dedupe_markdown_project(&mut project);
    Ok(build_convert_project_output(&from, label, &graph, &project, &transform_outcome, handlers_json))
}

/// Converts bytes and publishes a Markdown project directory.
pub fn convert_to_markdown_project(
    from: &str,
    label: &str,
    bytes: Vec<u8>,
    output_dir: impl AsRef<Path>,
    handlers_json: Option<&str>,
) -> Result<(ConvertProjectOutput, PublishedMarkdownProject)> {
    let from = from.to_ascii_lowercase();
    if !supports_markdown_project_source(&from) {
        return format_error::fail(AdapterError::unsupported_format(from, "markdown project"));
    }

    let mut graph = format_error::propagate(read_source(&from, label, bytes))?;
    let transform_outcome = apply_transform_passes(&mut graph, handlers_json);
    let mut project = format_error::propagate(write_markdown_project(&graph))?;
    dedupe_assets::dedupe_markdown_project(&mut project);
    let output = build_convert_project_output(&from, label, &graph, &project, &transform_outcome, handlers_json);
    let published = publish_markdown_project(output_dir, &project, &output.report_json)?;
    Ok((output, published))
}

fn build_convert_project_output(
    from: &str,
    label: &str,
    graph: &DocumentGraph,
    project: &notedown_formats::export::markdown_project::MarkdownProject,
    transform_outcome: &TransformOutcome,
    handlers_json: Option<&str>,
) -> ConvertProjectOutput {
    let loss_count = graph.coverage.loss.len();
    let mut diagnostic_set = diagnostics_from_graph(graph);
    for diagnostic in &transform_outcome.diagnostics {
        diagnostic_set.push(diagnostic.clone());
    }

    // Handler registry bridge — same pattern as convert_bytes.
    let (pipeline_handlers, handler_keys) = parse_handlers_descriptor(handlers_json);
    if !handler_keys.is_empty() {
        diagnostic_set.push(build_handler_info_diagnostic(&handler_keys));
    }

    if loss_count > 0 {
        panduck_diagnostic::log_diagnostic_set(&diagnostic_set);
    }
    let diagnostic_envelope = DiagnosticEnvelope::from_set(&diagnostic_set);
    let published_assets: Vec<String> = project.assets.iter().map(|asset| asset.relative_path.clone()).collect();
    let published_chapters: Vec<String> = project.chapters.iter().map(|chapter| chapter.relative_path.clone()).collect();
    let mut outputs = vec![
        serde_json::json!({ "path": "index.md", "format": "markdown", "published": true }),
        serde_json::json!({ "path": "assets/", "format": "assets", "published_asset_count": published_assets.len() }),
    ];
    for chapter in &published_chapters {
        outputs.push(serde_json::json!({ "path": chapter, "format": "markdown", "published": true }));
    }
    outputs.push(serde_json::json!({ "path": "panduck.report.json", "format": "report", "published": true }));
    let mut pipeline = serde_json::json!({ "reader": from, "writer": "markdown-project", "stages": ["read", "ir", "write", "materialize"] });
    if !transform_outcome.is_empty() {
        pipeline["transforms"] = serde_json::to_value(&transform_outcome.passes).unwrap_or_default();
    }
    if let Some(handlers_val) = pipeline_handlers {
        pipeline["handlers"] = handlers_val;
    }
    let report_json = serde_json::json!({
        "schema_version": panduck_diagnostic::SCHEMA_REPORT,
        "operation": "convert_project",
        "status": if loss_count == 0 { "success" } else { "success_with_loss" },
        "inputs": [{ "path": label, "format": from }],
        "pipeline": pipeline,
        "coverage": graph.coverage,
        "losses": graph.coverage.loss,
        "diagnostics": diagnostic_envelope,
        "outputs": outputs,
        "unresolved_assets": project.unresolved_asset_sources,
    })
    .to_string();

    ConvertProjectOutput {
        index_markdown: project.index_markdown.clone(),
        published_assets,
        published_chapters,
        unresolved_assets: project.unresolved_asset_sources.clone(),
        report_json,
        loss_count,
    }
}

/// Returns whether Panduck can emit a Markdown project from this source format.
pub fn supports_markdown_project_source(from: &str) -> bool {
    matches!(from.to_ascii_lowercase().as_str(), "docx" | "doc" | "pdf" | "epub" | "html" | "markdown" | "notedown")
}

/// Converts a file on disk into a Markdown project directory.
pub fn convert_markdown_project_file(
    from: &str,
    input: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
    handlers_json: Option<&str>,
) -> Result<(ConvertProjectOutput, PublishedMarkdownProject)> {
    let input = input.as_ref();
    let bytes = match fs::read(input) {
        Ok(bytes) => bytes,
        Err(source) => {
            return format_error::fail(AdapterError::io(source, Some(input.display().to_string())));
        }
    };
    convert_to_markdown_project(from, &input.display().to_string(), bytes, output_dir, handlers_json)
}

/// Converts a file on disk.
pub fn convert_file(
    from: &str,
    to: &str,
    input: impl AsRef<Path>,
    handlers_json: Option<&str>,
) -> Result<ConvertOutput> {
    let input = input.as_ref();
    let bytes = match fs::read(input) {
        Ok(bytes) => bytes,
        Err(source) => {
            return format_error::fail(AdapterError::io(source, Some(input.display().to_string())));
        }
    };
    convert_bytes(from, to, &input.display().to_string(), bytes, handlers_json)
}

fn read_source(from: &str, label: &str, bytes: Vec<u8>) -> Result<DocumentGraph> {
    match from {
        "docx" => import_docx_bytes(label, &bytes).map_err(crate::format_error::map_format_error),
        "doc" => import_doc_bytes(label, &bytes).map_err(crate::format_error::map_format_error),
        "pdf" => import_pdf_bytes(label, &bytes).map_err(crate::format_error::map_format_error),
        "epub" => import_epub_bytes(label, &bytes).map_err(crate::format_error::map_format_error),
        "html" => import_html_bytes(label, &bytes).map_err(crate::format_error::map_format_error),
        "markdown" => {
            let text = match String::from_utf8(bytes) {
                Ok(text) => text,
                Err(error) => {
                    return format_error::fail(AdapterError::invalid_input(format!(
                        "markdown input is not valid UTF-8: {error}"
                    )));
                }
            };
            read_markdown_bytes(label, text)
        }
        "notedown" => {
            let text = match String::from_utf8(bytes) {
                Ok(text) => text,
                Err(error) => {
                    return format_error::fail(AdapterError::invalid_input(format!(
                        "notedown input is not valid UTF-8: {error}"
                    )));
                }
            };
            read_notedown_bytes(label, text)
        }
        other => format_error::fail(AdapterError::unsupported_format(other, "read")),
    }
}

fn write_target(to: &str, graph: &DocumentGraph) -> Result<(String, Option<String>, Option<Vec<u8>>)> {
    match to {
        "markdown" => {
            let markdown = write_document_markdown(graph)?;
            Ok((markdown, None, None))
        }
        "docx" => {
            let binary = write_document_docx(graph)?;
            Ok((String::new(), None, Some(binary)))
        }
        "pdf" => {
            let binary = write_document_pdf(graph)?;
            Ok((String::new(), None, Some(binary)))
        }
        "html" => {
            let html = write_document_html(graph)?;
            Ok((String::new(), Some(html), None))
        }
        other => format_error::fail(AdapterError::unsupported_format(other, "write")),
    }
}

// ---------------------------------------------------------------------------
// Handler registry bridge — JSON descriptor from the TS-side plan
// ---------------------------------------------------------------------------

/// Parse the optional JSON-serialized RuleDescriptor from the TS side.
///
/// Returns `(pipeline_value, handler_keys)`. The pipeline value is the full
/// descriptor object ready to be embedded into `report.pipeline.handlers`.
/// `handler_keys` is the extracted string array. Either side can be empty
/// without affecting the other.
///
/// Silently ignores malformed input — a corrupt descriptor must never cause
/// the main conversion pipeline to fail.
fn parse_handlers_descriptor(handlers_json: Option<&str>) -> (Option<serde_json::Value>, Vec<String>) {
    let Some(json_str) = handlers_json else {
        return (None, Vec::new());
    };
    if json_str.trim().is_empty() {
        return (None, Vec::new());
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) else {
        return (None, Vec::new());
    };
    let handler_keys = value
        .get("handlerKeys")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let pipeline_val = serde_json::json!({
        "descriptor": value,
        "handler_count": handler_keys.len(),
    });
    (Some(pipeline_val), handler_keys)
}

/// Build an info-level diagnostic announcing a handler registry was received.
fn build_handler_info_diagnostic(handler_keys: &[String]) -> Diagnostic {
    let count = handler_keys.len();
    let keys_display = handler_keys
        .iter()
        .map(|k| format!("\"{k}\""))
        .collect::<Vec<_>>()
        .join(", ");
    Diagnostic::new(
        DiagnosticCode::new("panduck.convert.handlers-received"),
        DiagnosticSeverity::Info,
        DiagnosticOrigin::new("panduck", "convert"),
        Message::new("panduck.convert.handlers-received")
            .with_fallback(format!(
                "Handler registry received: {count} handler(s) [{keys_display}] — Rust IR lowering will execute custom replace() callbacks when dispatch host is wired"
            )),
    )
}
