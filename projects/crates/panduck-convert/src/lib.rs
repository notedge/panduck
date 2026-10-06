#![deny(missing_docs)]
#![doc = include_str!("readme.md")]

mod format_error;
mod inspect;
mod publish;
mod readers;
mod writers;

use std::fs;
use std::path::Path;

use notedown_ir::DocumentGraph;
use panduck_diagnostic::{diagnostics_from_graph, DiagnosticEnvelope};
use notedown_formats::import::docx::import_docx_bytes;
use notedown_formats::import::doc::{import_doc_bytes};
use notedown_formats::import::pdf::import_pdf_bytes;
use notedown_formats::import::epub::import_epub_bytes;
use notedown_formats::import::html::import_html_bytes;
use panduck_types::{AdapterError, Result};
use serde::Serialize;

pub use inspect::{
    inspect_docx_decode, inspect_docx_decode_bytes, inspect_docx_index, inspect_docx_index_bytes,
    DocxDecodedPart, DocxInspectDecode, DocxInspectIndex,
};
pub use publish::{publish_bytes, publish_markdown_project, publish_text, PublishedMarkdownProject};
pub use readers::{read_doc, read_doc_bytes, read_html, read_html_bytes, read_markdown, read_markdown_bytes, read_notedown, read_notedown_bytes, read_pdf, read_pdf_bytes};
pub use writers::{write_document_docx, write_document_html, write_document_markdown, write_document_pdf, write_markdown_project};

/// Supported conversion route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Route {
    /// Source format name.
    pub from: &'static str,
    /// Target format name.
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
    Route { from: "doc", to: "markdown" },
    Route { from: "doc", to: "html" },
    Route { from: "pdf", to: "markdown" },
    Route { from: "pdf", to: "html" },
    Route { from: "pdf", to: "docx" },
    Route { from: "docx", to: "pdf" },
    Route { from: "markdown", to: "pdf" },
    Route { from: "html", to: "pdf" },
    Route {
        from: "markdown",
        to: "markdown",
    },
    Route {
        from: "markdown",
        to: "html",
    },
    Route {
        from: "notedown",
        to: "markdown",
    },
    Route {
        from: "notedown",
        to: "html",
    },
    Route {
        from: "epub",
        to: "markdown",
    },
    Route {
        from: "epub",
        to: "docx",
    },
    Route {
        from: "epub",
        to: "html",
    },
    Route {
        from: "markdown",
        to: "docx",
    },
    Route {
        from: "docx",
        to: "html",
    },
    Route {
        from: "html",
        to: "markdown",
    },
    Route {
        from: "html",
        to: "html",
    },
    Route {
        from: "html",
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
    /// Serialized `panduck.report/v1` JSON payload.
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
    /// Serialized `panduck.report/v1` JSON payload.
    pub report_json: String,
    /// Number of semantic loss markers recorded in the report.
    pub loss_count: usize,
}

/// Converts bytes from `from` format to `to` format.
pub fn convert_bytes(from: &str, to: &str, label: &str, bytes: Vec<u8>) -> Result<ConvertOutput> {
    let from = from.to_ascii_lowercase();
    let to = to.to_ascii_lowercase();
    if !supports_route(&from, &to) {
        return format_error::fail(AdapterError::unsupported_format(from, format!("convert to {to}")));
    }

    let graph = format_error::propagate(read_source(&from, label, bytes))?;
    let (markdown, html, binary) = format_error::propagate(write_target(&to, &graph))?;
    let loss_count = graph.coverage.loss.len();
    let diagnostic_set = diagnostics_from_graph(&graph);
    if loss_count > 0 {
        panduck_diagnostic::log_diagnostic_set(&diagnostic_set);
    }
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
        html,
        binary,
        report_json,
        loss_count,
    })
}

/// Converts bytes into a Markdown project description without writing disk.
pub fn convert_to_markdown_project_bytes(from: &str, label: &str, bytes: Vec<u8>) -> Result<ConvertProjectOutput> {
    let from = from.to_ascii_lowercase();
    if !supports_markdown_project_source(&from) {
        return format_error::fail(AdapterError::unsupported_format(from, "markdown project"));
    }

    let graph = format_error::propagate(read_source(&from, label, bytes))?;
    let project = format_error::propagate(write_markdown_project(&graph))?;
    Ok(build_convert_project_output(&from, label, &graph, &project))
}

/// Converts bytes and publishes a Markdown project directory.
pub fn convert_to_markdown_project(
    from: &str,
    label: &str,
    bytes: Vec<u8>,
    output_dir: impl AsRef<Path>,
) -> Result<(ConvertProjectOutput, PublishedMarkdownProject)> {
    let from = from.to_ascii_lowercase();
    if !supports_markdown_project_source(&from) {
        return format_error::fail(AdapterError::unsupported_format(from, "markdown project"));
    }

    let graph = format_error::propagate(read_source(&from, label, bytes))?;
    let project = format_error::propagate(write_markdown_project(&graph))?;
    let output = build_convert_project_output(&from, label, &graph, &project);
    let published = publish_markdown_project(output_dir, &project, &output.report_json)?;
    Ok((output, published))
}

fn build_convert_project_output(
    from: &str,
    label: &str,
    graph: &DocumentGraph,
    project: &notedown_formats::export::markdown_project::MarkdownProject,
) -> ConvertProjectOutput {
    let loss_count = graph.coverage.loss.len();
    let diagnostic_set = diagnostics_from_graph(graph);
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
    let report_json = serde_json::json!({
        "schema_version": "panduck.report/v1",
        "operation": "convert_project",
        "status": if graph.coverage.complete { "success" } else { "success_with_loss" },
        "inputs": [{ "path": label, "format": from }],
        "pipeline": { "reader": from, "writer": "markdown-project", "stages": ["read", "ir", "write", "materialize"] },
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
    matches!(
        from.to_ascii_lowercase().as_str(),
        "docx" | "doc" | "pdf" | "epub" | "html" | "markdown" | "notedown"
    )
}

/// Converts a file on disk into a Markdown project directory.
pub fn convert_markdown_project_file(
    from: &str,
    input: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> Result<(ConvertProjectOutput, PublishedMarkdownProject)> {
    let input = input.as_ref();
    let bytes = match fs::read(input) {
        Ok(bytes) => bytes,
        Err(source) => {
            return format_error::fail(AdapterError::io(source, Some(input.display().to_string())));
        }
    };
    convert_to_markdown_project(from, &input.display().to_string(), bytes, output_dir)
}

/// Converts a file on disk.
pub fn convert_file(from: &str, to: &str, input: impl AsRef<Path>) -> Result<ConvertOutput> {
    let input = input.as_ref();
    let bytes = match fs::read(input) {
        Ok(bytes) => bytes,
        Err(source) => {
            return format_error::fail(AdapterError::io(source, Some(input.display().to_string())));
        }
    };
    convert_bytes(from, to, &input.display().to_string(), bytes)
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
