//! Orthogonal conversion primitives exported through Node-API.
//!
//! The crate boundary stays data-in/data-out: every entry point takes bytes or
//! serialized IR and returns bytes, serialized IR, losses and diagnostics. File
//! discovery, output placement, exit-code mapping, CLI wording and report
//! assembly live in the TypeScript CLI. Nothing here reads or writes paths.

use document_convert::{
    ConversionBudgets, SCHEMA_REPORT, apply_transform_passes, from_adapter_error, inspect_docx_decode_bytes,
    inspect_docx_index_bytes, read_markdown_bytes, read_notedown_bytes, read_tex_bytes, read_typst_bytes,
    supports_markdown_project_source, write_document_docx, write_document_epub, write_document_html,
    write_document_markdown, write_document_pdf, write_markdown_project,
};
use document_formats::import::{
    doc::import_doc_bytes, docx::import_docx_bytes, epub::import_epub_bytes, html::import_html_bytes, pdf::import_pdf_bytes,
};
use document_formats::{FormatError, from_format_error};
use document_ir::DocumentGraph;
use document_types::{AdapterError, Result as AdapterResult};
use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::structure_guard;

/// N-API container index probe response.
#[napi(object)]
pub struct ProbeIndexResponse {
    /// True when the container was indexed.
    pub success: bool,
    /// Detected container format.
    pub format: String,
    /// Outer container label from Acorn detection.
    pub outer: String,
    /// Inner payload label from Acorn detection.
    pub inner: String,
    /// OPC member paths discovered in the package.
    pub parts: Vec<String>,
    /// Serialized diagnostic envelope.
    pub diagnostics_json: String,
}

/// One decoded OPC member in a probe decode response.
#[napi(object)]
pub struct ProbeDecodedPart {
    /// OPC member path inside the package.
    pub path: String,
    /// ZIP compression method identifier.
    pub compression_method: u16,
    /// Compressed byte length stored in the archive.
    pub compressed_size: u32,
    /// Uncompressed byte length declared in the archive.
    pub uncompressed_size: u32,
    /// Decoded payload byte length after inflation.
    pub decoded_size: u32,
}

/// N-API container decode probe response.
#[napi(object)]
pub struct ProbeDecodeResponse {
    /// True when the container was decoded.
    pub success: bool,
    /// Detected container format.
    pub format: String,
    /// Outer container label from Acorn detection.
    pub outer: String,
    /// Inner payload label from Acorn detection.
    pub inner: String,
    /// Per-member decode summaries for the package.
    pub parts: Vec<ProbeDecodedPart>,
    /// Serialized diagnostic envelope.
    pub diagnostics_json: String,
}

/// N-API import response.
#[napi(object)]
pub struct ImportResponse {
    /// True when the bytes were read into IR.
    pub success: bool,
    /// Serialized `document.ir` graph, absent on failure.
    pub graph_json: Option<String>,
    /// Serialized coverage records for the imported graph.
    pub coverage_json: String,
    /// Serialized loss records with source spans.
    pub losses_json: String,
    /// Serialized diagnostic envelope for the import stage.
    pub diagnostics_json: String,
}

/// N-API IR transform response.
#[napi(object)]
pub struct TransformResponse {
    /// True when the graph was parsed and transformed.
    pub success: bool,
    /// Serialized `document.ir` graph after the passes ran.
    pub graph_json: String,
    /// Serialized transform pass reports for the plan descriptor.
    pub passes_json: String,
    /// Serialized diagnostic envelope for the transform stage.
    pub diagnostics_json: String,
}

/// N-API export response.
#[napi(object)]
pub struct ExportResponse {
    /// True when the graph was written to the requested target.
    pub success: bool,
    /// Text output for text targets such as `markdown`.
    pub text: Option<String>,
    /// HTML output for the HTML target.
    pub html: Option<String>,
    /// Binary output for container targets such as `docx`.
    pub binary: Option<Buffer>,
    /// Serialized coverage records after the export stage.
    pub coverage_json: String,
    /// Serialized loss records after the export stage.
    pub losses_json: String,
    /// Serialized diagnostic envelope for the export stage.
    pub diagnostics_json: String,
}

/// One Markdown project chapter.
#[napi(object)]
pub struct ProjectChapter {
    /// Project-relative chapter path.
    pub path: String,
    /// Markdown body for this chapter.
    pub markdown: String,
}

/// One materialized Markdown project asset.
#[napi(object)]
pub struct ProjectAsset {
    /// Project-relative asset path.
    pub path: String,
    /// Raw asset bytes.
    pub bytes: Buffer,
}

/// N-API Markdown project export response.
#[napi(object)]
pub struct ProjectResponse {
    /// True when the graph was exported as a Markdown project.
    pub success: bool,
    /// Primary Markdown body for `index.md`.
    pub index_markdown: String,
    /// Additional chapter files.
    pub chapters: Vec<ProjectChapter>,
    /// Materialized asset files.
    pub assets: Vec<ProjectAsset>,
    /// Image sources that could not be materialized.
    pub unresolved_assets: Vec<String>,
    /// Serialized coverage records after the export stage.
    pub coverage_json: String,
    /// Serialized loss records after the export stage.
    pub losses_json: String,
    /// Serialized diagnostic envelope for the export stage.
    pub diagnostics_json: String,
}

/// Indexes a DOCX OPC package from in-memory bytes.
#[napi]
pub fn probe_container_index(label: String, bytes: Buffer) -> ProbeIndexResponse {
    match inspect_docx_index_bytes(label, bytes.to_vec()) {
        Ok(index) => ProbeIndexResponse {
            success: true,
            format: index.format,
            outer: index.outer,
            inner: index.inner,
            parts: index.parts,
            diagnostics_json: empty_envelope(),
        },
        Err(error) => ProbeIndexResponse {
            success: false,
            format: String::new(),
            outer: String::new(),
            inner: String::new(),
            parts: Vec::new(),
            diagnostics_json: error_envelope(&error),
        },
    }
}

/// Decodes DOCX OPC members from in-memory bytes.
#[napi]
pub fn probe_container_decode(label: String, bytes: Buffer, part_path: Option<String>) -> ProbeDecodeResponse {
    let filter = part_path.as_deref();
    match inspect_docx_decode_bytes(label, bytes.to_vec(), filter) {
        Ok(decode) => ProbeDecodeResponse {
            success: true,
            format: decode.format,
            outer: decode.outer,
            inner: decode.inner,
            parts: decode
                .parts
                .iter()
                .map(|part| ProbeDecodedPart {
                    path: part.path.clone(),
                    compression_method: part.compression_method,
                    compressed_size: part.compressed_size.min(u32::MAX as u64) as u32,
                    uncompressed_size: part.uncompressed_size.min(u32::MAX as u64) as u32,
                    decoded_size: part.decoded_size.min(u32::MAX as u64) as u32,
                })
                .collect(),
            diagnostics_json: empty_envelope(),
        },
        Err(error) => ProbeDecodeResponse {
            success: false,
            format: String::new(),
            outer: String::new(),
            inner: String::new(),
            parts: Vec::new(),
            diagnostics_json: error_envelope(&error),
        },
    }
}

/// Reads bytes of `from` into a serialized `document.ir` graph.
///
/// Applies container budgets before the reader and the built-in structural
/// nesting guard before any text grammar sees the payload. Failures are
/// returned through `success: false` plus a structured diagnostic envelope,
/// never as exit codes or free-form CLI wording.
#[napi]
pub fn import_document(from: String, label: String, bytes: Buffer, budgets_json: Option<String>) -> ImportResponse {
    let from = from.to_ascii_lowercase();
    if !supports_markdown_project_source(&from) {
        return import_error(AdapterError::unsupported_format(from.as_str(), "read"), true);
    }
    let budgets = ConversionBudgets::from_json(budgets_json.as_deref());
    if let Err(error) = budgets.enforce_container(&from, &label, bytes.as_ref()) {
        // The budget layer already logged the adapter error.
        return import_error(error, false);
    }
    match read_source(&from, &label, bytes.as_ref()) {
        Ok(graph) => ImportResponse {
            success: true,
            graph_json: serde_json::to_string(&graph).ok(),
            coverage_json: serde_json::to_string(&graph.coverage).unwrap_or_else(|_| "null".to_string()),
            losses_json: losses_json(&graph),
            diagnostics_json: graph_envelope(&graph),
        },
        Err(error) => import_error(error, true),
    }
}

/// Runs the transform passes described by `transforms_json` over serialized IR.
///
/// `transforms_json` is the optional `transforms` array of the TS-side
/// `ConversionPlan`. Policy handler closures never cross the wire.
#[napi]
pub fn transform_document(graph_json: String, transforms_json: Option<String>) -> TransformResponse {
    let mut graph: DocumentGraph = match serde_json::from_str(&graph_json) {
        Ok(graph) => graph,
        Err(error) => {
            let error = AdapterError::invalid_input(format!("graph is not a document graph: {error}"));
            document_diagnostic::log_adapter_error(&error);
            return TransformResponse {
                success: false,
                graph_json,
                passes_json: "[]".to_string(),
                diagnostics_json: error_envelope(&error),
            };
        }
    };
    let outcome = apply_transform_passes(&mut graph, transforms_json.as_deref());
    let mut set = boost_diagnostic::DiagnosticSet::new();
    for diagnostic in &outcome.diagnostics {
        set.push(diagnostic.clone());
    }
    TransformResponse {
        success: true,
        graph_json: serde_json::to_string(&graph).unwrap_or(graph_json),
        passes_json: serde_json::to_string(&outcome.passes).unwrap_or_else(|_| "[]".to_string()),
        diagnostics_json: envelope_json(&set),
    }
}

/// Writes a serialized graph to one target format.
///
/// HTML export additionally lowers diagrams and charts and records their losses
/// on the graph, so the returned losses and diagnostics reflect the bytes the
/// caller is about to publish.
#[napi]
pub fn export_document(to: String, graph_json: String) -> ExportResponse {
    let to = to.to_ascii_lowercase();
    let mut graph: DocumentGraph = match serde_json::from_str(&graph_json) {
        Ok(graph) => graph,
        Err(error) => {
            let error = AdapterError::invalid_input(format!("graph is not a document graph: {error}"));
            document_diagnostic::log_adapter_error(&error);
            return export_error(error);
        }
    };
    crate::assets::install_math_fonts();
    match write_target(&to, &mut graph) {
        Ok((text, html, binary)) => {
            log_losses(&graph);
            ExportResponse {
                success: true,
                text: if text.is_empty() { None } else { Some(text) },
                html,
                binary: binary.map(Buffer::from),
                coverage_json: serde_json::to_string(&graph.coverage).unwrap_or_else(|_| "null".to_string()),
                losses_json: losses_json(&graph),
                diagnostics_json: graph_envelope(&graph),
            }
        }
        Err(error) => export_error(error),
    }
}

/// Exports a serialized graph as a Markdown project payload.
///
/// Assets are content-addressed with a BLAKE3 prefix and duplicate payloads
/// collapse to one entry. The returned bytes are not written to disk.
#[napi]
pub fn export_markdown_project(graph_json: String) -> ProjectResponse {
    let graph: DocumentGraph = match serde_json::from_str(&graph_json) {
        Ok(graph) => graph,
        Err(error) => {
            let error = AdapterError::invalid_input(format!("graph is not a document graph: {error}"));
            document_diagnostic::log_adapter_error(&error);
            return project_error(error);
        }
    };
    let mut project = match write_markdown_project(&graph) {
        Ok(project) => project,
        Err(error) => return project_error(error),
    };
    dedupe_assets::dedupe_markdown_project(&mut project);
    log_losses(&graph);
    ProjectResponse {
        success: true,
        index_markdown: project.index_markdown,
        chapters: project
            .chapters
            .iter()
            .map(|chapter| ProjectChapter {
                path: chapter.relative_path.clone(),
                markdown: chapter.markdown.clone(),
            })
            .collect(),
        assets: project
            .assets
            .iter()
            .map(|asset| ProjectAsset {
                path: asset.relative_path.clone(),
                bytes: Buffer::from(asset.bytes.clone()),
            })
            .collect(),
        unresolved_assets: project.unresolved_asset_sources,
        coverage_json: serde_json::to_string(&graph.coverage).unwrap_or_else(|_| "null".to_string()),
        losses_json: losses_json(&graph),
        diagnostics_json: graph_envelope(&graph),
    }
}

fn read_source(from: &str, label: &str, bytes: &[u8]) -> AdapterResult<DocumentGraph> {
    structure_guard::enforce(from, bytes)?;
    match from {
        "docx" => import_docx_bytes(label, bytes).map_err(map_format_error),
        "doc" => import_doc_bytes(label, bytes).map_err(map_format_error),
        "pdf" => import_pdf_bytes(label, bytes).map_err(map_format_error),
        "epub" => import_epub_bytes(label, bytes).map_err(map_format_error),
        "html" => import_html_bytes(label, bytes).map_err(map_format_error),
        "markdown" => {
            let text = utf8("markdown", bytes)?;
            read_markdown_bytes(label, text)
        }
        "notedown" => {
            let text = utf8("notedown", bytes)?;
            read_notedown_bytes(label, text)
        }
        "tex" => {
            let text = utf8("tex", bytes)?;
            read_tex_bytes(label, text)
        }
        "typst" => {
            let text = utf8("typst", bytes)?;
            read_typst_bytes(label, text)
        }
        other => fail(AdapterError::unsupported_format(other, "read")),
    }
}

fn utf8(from: &str, bytes: &[u8]) -> AdapterResult<String> {
    String::from_utf8(bytes.to_vec())
        .map_err(|error| AdapterError::invalid_input(format!("{from} input is not valid UTF-8: {error}")))
}

fn write_target(to: &str, graph: &mut DocumentGraph) -> AdapterResult<(String, Option<String>, Option<Vec<u8>>)> {
    match to {
        "markdown" => Ok((write_document_markdown(graph)?, None, None)),
        "docx" => Ok((String::new(), None, Some(write_document_docx(graph)?))),
        "pdf" => Ok((String::new(), None, Some(write_document_pdf(graph)?))),
        "html" => {
            let diagrams = document_convert::diagram::render_diagrams(graph);
            for (loss, span) in diagrams.losses.into_iter().zip(diagrams.loss_spans) {
                graph.push_loss_at(loss, span);
            }
            let charts = document_convert::chart::render_charts(graph);
            for (loss, span) in charts.losses.into_iter().zip(charts.loss_spans) {
                graph.push_loss_at(loss, span);
            }
            let mut scenes = diagrams.scenes;
            scenes.extend(charts.scenes);
            Ok((String::new(), Some(write_document_html(graph, &scenes)?), None))
        }
        "epub" => Ok((String::new(), None, Some(write_document_epub(graph)?))),
        other => fail(AdapterError::unsupported_format(other, "write")),
    }
}

fn import_error(error: AdapterError, log: bool) -> ImportResponse {
    if log {
        document_diagnostic::log_adapter_error(&error);
    }
    ImportResponse {
        success: false,
        graph_json: None,
        coverage_json: "null".to_string(),
        losses_json: "[]".to_string(),
        diagnostics_json: error_envelope(&error),
    }
}

fn export_error(error: AdapterError) -> ExportResponse {
    document_diagnostic::log_adapter_error(&error);
    ExportResponse {
        success: false,
        text: None,
        html: None,
        binary: None,
        coverage_json: "null".to_string(),
        losses_json: "[]".to_string(),
        diagnostics_json: error_envelope(&error),
    }
}

fn project_error(error: AdapterError) -> ProjectResponse {
    document_diagnostic::log_adapter_error(&error);
    ProjectResponse {
        success: false,
        index_markdown: String::new(),
        chapters: Vec::new(),
        assets: Vec::new(),
        unresolved_assets: Vec::new(),
        coverage_json: "null".to_string(),
        losses_json: "[]".to_string(),
        diagnostics_json: error_envelope(&error),
    }
}

fn fail<T>(error: AdapterError) -> AdapterResult<T> {
    document_diagnostic::log_adapter_error(&error);
    Err(error)
}

fn map_format_error(error: FormatError) -> AdapterError {
    AdapterError::diagnostic(from_format_error(&error))
}

fn losses_json(graph: &DocumentGraph) -> String {
    let losses: Vec<serde_json::Value> = graph
        .coverage
        .loss
        .iter()
        .enumerate()
        .map(|(index, loss)| {
            let span = graph.loss_span(index);
            serde_json::json!({
                "code": loss.code,
                "message": loss.message,
                "status": loss.status,
                "severity": document_diagnostic::loss_severity(loss.status).to_string(),
                "span": match span {
                    Some(span) => serde_json::json!({ "start": span.start, "end": span.end }),
                    None => serde_json::Value::Null,
                },
            })
        })
        .collect();
    serde_json::Value::Array(losses).to_string()
}

fn graph_envelope(graph: &DocumentGraph) -> String {
    envelope_json(&document_diagnostic::diagnostics_from_graph(graph))
}

/// Mirrors the upstream sink rule: loss-bearing conversions reach the installed
/// JSON-lines logger, loss-free ones stay quiet.
fn log_losses(graph: &DocumentGraph) {
    if graph.coverage.loss.is_empty() {
        return;
    }
    document_diagnostic::log_diagnostic_set(&document_diagnostic::diagnostics_from_graph(graph));
}

fn error_envelope(error: &AdapterError) -> String {
    let mut set = boost_diagnostic::DiagnosticSet::new();
    set.push(from_adapter_error(error));
    envelope_json(&set)
}

fn empty_envelope() -> String {
    envelope_json(&boost_diagnostic::DiagnosticSet::new())
}

fn envelope_json(set: &boost_diagnostic::DiagnosticSet) -> String {
    serde_json::to_string(&document_convert::DiagnosticEnvelope::from_set(set))
        .unwrap_or_else(|_| format!("{{\"schema_version\":\"{SCHEMA_REPORT}\",\"diagnostics\":[]}}"))
}

mod dedupe_assets {
    //! Content-addressable asset deduplication for Markdown projects.
    //!
    //! Mirrors `document-convert`'s private `dedupe_assets` module. It stays here
    //! because the primitive boundary must return the same asset identity the
    //! aggregate path produced without writing to disk upstream.

    use std::collections::HashMap;

    use document_formats::export::markdown_project::{MarkdownProject, MarkdownProjectAsset};

    const FINGERPRINT_PREFIX_LEN: usize = 16;

    pub fn dedupe_markdown_project(project: &mut MarkdownProject) -> usize {
        let mut fingerprint_to_path: HashMap<blake3::Hash, String> = HashMap::new();
        let mut old_to_new: HashMap<String, String> = HashMap::new();
        let mut rewritten_assets: Vec<_> = Vec::new();

        for asset in project.assets.drain(..) {
            let fingerprint = blake3_hash(&asset.bytes);
            let is_first = !fingerprint_to_path.contains_key(&fingerprint);
            let canonical_path = fingerprint_to_path
                .entry(fingerprint)
                .or_insert_with(|| content_address_path(&asset.relative_path, &fingerprint))
                .clone();

            old_to_new.insert(asset.relative_path, canonical_path.clone());

            if is_first {
                rewritten_assets.push(MarkdownProjectAsset {
                    relative_path: canonical_path,
                    bytes: asset.bytes,
                    asset_id: asset.asset_id,
                });
            }
        }

        project.assets = rewritten_assets;

        rewrite_markdown_refs(&mut project.index_markdown, &old_to_new);
        for chapter in &mut project.chapters {
            rewrite_markdown_refs(&mut chapter.markdown, &old_to_new);
        }

        fingerprint_to_path.len()
    }

    fn blake3_hash(bytes: &[u8]) -> blake3::Hash {
        let mut hasher = blake3::Hasher::new();
        hasher.update(bytes);
        hasher.finalize()
    }

    fn content_address_path(old_path: &str, fingerprint: &blake3::Hash) -> String {
        let ext = old_path.rsplit('.').next().filter(|e| !e.is_empty() && e.len() <= 8).unwrap_or("bin");
        let hex = fingerprint.to_hex();
        let prefix = hex.as_str().split_at(FINGERPRINT_PREFIX_LEN).0;
        format!("assets/{prefix}.{ext}")
    }

    fn rewrite_markdown_refs(body: &mut String, old_to_new: &HashMap<String, String>) {
        if old_to_new.is_empty() {
            return;
        }

        let mut replaced = body.clone();
        for (old, new) in old_to_new {
            let patterns = [format!("]({old})"), format!("](./{old})"), format!("]({old} "), format!("](./{old} ")];
            for pattern in &patterns {
                let replacement = format!("]({new})");
                let replacement_with_space = format!("]({new} ");
                replaced = replaced.replace(pattern, if pattern.ends_with(' ') { &replacement_with_space } else { &replacement });
            }
        }
        *body = replaced;
    }
}
