#![deny(missing_docs)]

//! Node-API export surface for Panduck.

use std::collections::BTreeSet;

use document_convert::{
    ConversionBudgets, READERS, WRITERS, convert_bytes_with_budgets, convert_file_with_budgets,
    convert_markdown_project_file_with_budgets,
    convert_to_markdown_project_with_budgets as convert_markdown_project_bytes_impl, empty_diagnostic_envelope,
    inspect_docx_decode, inspect_docx_index, supported_routes, supports_markdown_project_source, supports_route,
};
use document_types::AdapterError;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use oak_core::{Lexer, TokenType, lexer::NoLexerCache, source::SourceText};

/// N-API Markdown project conversion response.
#[napi(object)]
pub struct ConvertProjectResponse {
    /// Process exit code. Zero means success.
    pub exit_code: u32,
    /// Primary Markdown body written to `index.md`.
    pub index_markdown: String,
    /// Project-relative asset paths that were materialized.
    pub published_assets: Vec<String>,
    /// Project-relative chapter paths that were materialized.
    pub published_chapters: Vec<String>,
    /// Image sources left unresolved in the Markdown body.
    pub unresolved_assets: Vec<String>,
    /// Output directory root.
    pub output_dir: String,
    /// Serialized `panduck.report/v0` JSON payload.
    pub report_json: String,
}

/// N-API conversion response.
#[napi(object)]
pub struct ConvertResponse {
    /// Process exit code. Zero means success.
    pub exit_code: u32,
    /// Text output for text targets such as `markdown`.
    pub text: Option<String>,
    /// HTML output for the HTML target.
    pub html: Option<String>,
    /// Binary output for container targets such as `docx`.
    pub binary: Option<Buffer>,
    /// Serialized `panduck.report/v0` JSON payload.
    pub report_json: String,
}

/// One lexical token produced by a Panduck source grammar.
#[napi(object)]
pub struct LexToken {
    /// Grammar token kind, such as `Heading1`, `TagName` or `Eof`.
    pub kind: String,
    /// Source slice covered by this token.
    pub text: String,
    /// Byte offset of the first source character.
    pub start: u32,
    /// Byte offset one past the last source character.
    pub end: u32,
    /// Whether the grammar classifies this token as trivia.
    pub ignored: bool,
}

/// N-API lexical analysis response.
#[napi(object)]
pub struct LexResponse {
    /// False when the grammar reported a fatal lexical error.
    pub success: bool,
    /// Tokens in source order, including grammar trivia.
    pub tokens: Vec<LexToken>,
    /// Lexical diagnostics rendered as text.
    pub diagnostics: Vec<String>,
}

/// N-API container index response.

#[napi(object)]
pub struct InspectIndexResponse {
    /// Process exit code. Zero means success.
    pub exit_code: u32,
    /// Detected container format.
    pub format: String,
    /// Outer container label from Acorn detection.
    pub outer: String,
    /// Inner payload label from Acorn detection.
    pub inner: String,
    /// OPC member paths discovered in the package.
    pub parts: Vec<String>,
    /// Serialized `panduck.report/v0` JSON payload.
    pub report_json: String,
}

/// One decoded OPC member in an inspect decode response.
#[napi(object)]
pub struct InspectDecodedPart {
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

/// N-API container decode response.
#[napi(object)]
pub struct InspectDecodeResponse {
    /// Process exit code. Zero means success.
    pub exit_code: u32,
    /// Detected container format.
    pub format: String,
    /// Outer container label from Acorn detection.
    pub outer: String,
    /// Inner payload label from Acorn detection.
    pub inner: String,
    /// Per-member decode summaries for the package.
    pub parts: Vec<InspectDecodedPart>,
    /// Serialized `panduck.report/v0` JSON payload.
    pub report_json: String,
}

/// Returns the Panduck N-API binding version.
#[napi]
pub fn panduck_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Lists formats wired into this build (reader ∪ writer).
#[napi]
pub fn supported_formats() -> Vec<String> {
    let mut set = BTreeSet::new();
    for f in READERS {
        set.insert(*f);
    }
    for f in WRITERS {
        set.insert(*f);
    }
    set.into_iter().map(|s| s.to_string()).collect()
}

/// Returns whether a format name is recognized by Panduck adapters.
#[napi]
pub fn is_supported_format(name: String) -> bool {
    supported_formats().iter().any(|f| f.eq_ignore_ascii_case(name.as_str()))
}

/// Returns whether a conversion route is wired in this build.
#[napi]
pub fn supports_conversion(from: String, to: String) -> bool {
    supports_route(&from, &to)
}

/// Lexically analyses source text with the grammar Panduck reads it as.
///
/// Supported `language` values are `notedown`, `markdown`, `html` and `xml`.
/// `xml` covers the document parts inside OPC containers. The token stream is
/// the same one the matching reader feeds into its parser, so consumers can
/// build editors, linters or incremental caches on top of it.
#[napi]
pub fn lex_document(language: String, source: String) -> LexResponse {
    match language.to_ascii_lowercase().as_str() {
        "notedown" | "nd" => {
            let config = oak_notedown::NoteLanguage::default();
            lex_tokens::<oak_notedown::NoteLanguage, _>(&source, &oak_notedown::NoteLexer::new(&config))
        }
        "markdown" | "md" => {
            let config = markdown_grammar_language();
            lex_tokens::<oak_markdown::MarkdownLanguage, _>(&source, &oak_markdown::MarkdownLexer::new(&config))
        }
        "html" => {
            let config = oak_html::HtmlLanguage::default();
            lex_tokens::<oak_html::HtmlLanguage, _>(&source, &oak_html::HtmlLexer::new(&config))
        }
        "xml" => {
            let config = oak_xml::XmlLanguage::new();
            lex_tokens::<oak_xml::XmlLanguage, _>(&source, &oak_xml::XmlLexer::new(&config))
        }
        other => LexResponse {
            success: false,
            tokens: Vec::new(),
            diagnostics: vec![format!("lex.unsupported_language: `{other}`")],
        },
    }
}

/// Markdown grammar exposed by the lexer: GFM plus math and footnotes.
///
/// The reader layer applies a narrower parse policy, but lexical analysis is a
/// grammar level contract: a token stream that silently cannot represent
/// CommonMark core constructs (indented code blocks, autolinks, setext
/// headings) would be unusable for editors and linters. What the reader does
/// with those tokens stays a reader concern.
fn markdown_grammar_language() -> oak_markdown::MarkdownLanguage {
    let mut language = oak_markdown::MarkdownLanguage::github_flavored();
    language.allow_math = true;
    language.allow_footnotes = true;
    language
}

fn lex_tokens<L, X>(source: &str, lexer: &X) -> LexResponse
where
    L: oak_core::Language,
    X: Lexer<L>,
{
    let text = SourceText::new(source.to_string());
    let mut cache = NoLexerCache;
    let output = lexer.lex(&text, &[], &mut cache);
    let diagnostics = output.diagnostics.iter().map(|error| error.to_string()).collect::<Vec<_>>();
    match output.result {
        Ok(tokens) => LexResponse {
            success: true,
            tokens: tokens
                .iter()
                .map(|token| LexToken {
                    kind: format!("{:?}", token.kind),
                    text: source[token.span.start..token.span.end].to_string(),
                    start: u32::try_from(token.span.start).unwrap_or(u32::MAX),
                    end: u32::try_from(token.span.end).unwrap_or(u32::MAX),
                    ignored: token.kind.is_ignored(),
                })
                .collect(),
            diagnostics,
        },
        Err(error) => {
            let mut all = diagnostics;
            all.push(error.to_string());
            LexResponse { success: false, tokens: Vec::new(), diagnostics: all }
        }
    }
}

/// One diagram diagnostic, fatal or non-fatal.
#[napi(object)]
pub struct DiagramDiagnostic {
    /// Stable diagnostic code such as `davinci.diagram.unsupported`.
    pub code: String,
    /// `error`, `warning` or `info`.
    pub severity: String,
    /// Human readable message.
    pub message: String,
    /// Source slice the diagnostic points at, empty when it has no span.
    pub span_text: String,
}

/// N-API diagram render response.
#[napi(object)]
pub struct DiagramRenderResponse {
    /// False when the source was rejected or rendering failed.
    pub success: bool,
    /// SVG document for accepted sources.
    pub svg: Option<String>,
    /// Diagnostics in source order.
    pub diagnostics: Vec<DiagramDiagnostic>,
}

/// Renders a diagram source through the matching plotter frontend.
///
/// Supported `language` values are `dot`, `mermaid` and `plantuml`. The result
/// is deterministic SVG with caller scoped ids, which lets consumers embed the
/// same diagram several times in one page. TikZ has no frontend in Panduck yet,
/// so it is reported as unsupported instead of being silently accepted.
#[napi]
pub fn render_diagram(language: String, source: String) -> DiagramRenderResponse {
    match language.to_ascii_lowercase().as_str() {
        "dot" => lower_diagram(plotter_input_dot::dot_to_scene(&source), &source),
        "mermaid" => lower_diagram(plotter_input_mermaid::mermaid_to_scene(&source), &source),
        "plantuml" => lower_diagram(plotter_input_plantuml::plantuml_to_scene(&source), &source),
        other => DiagramRenderResponse {
            success: false,
            svg: None,
            diagnostics: vec![DiagramDiagnostic {
                code: "panduck.render.unsupported_language".to_string(),
                severity: "error".to_string(),
                message: format!("`{other}` has no diagram frontend in Panduck"),
                span_text: String::new(),
            }],
        },
    }
}

fn lower_diagram(
    lowered: std::result::Result<plotter_diagram::LoweredDiagram, plotter_diagram::DiagramError>,
    source: &str,
) -> DiagramRenderResponse {
    let lowered = match lowered {
        Ok(lowered) => lowered,
        Err(error) => {
            let span_text = error
                .span()
                .and_then(|span| source.get(span))
                .unwrap_or_default()
                .to_string();
            return DiagramRenderResponse {
                success: false,
                svg: None,
                diagnostics: vec![DiagramDiagnostic {
                    code: error.code().to_string(),
                    severity: "error".to_string(),
                    message: error.message().to_string(),
                    span_text,
                }],
            };
        }
    };
    let mut diagnostics: Vec<DiagramDiagnostic> =
        lowered.diagnostics.iter().map(diagram_diagnostic).collect();
    match lowered
        .scene
        .to_svg_with_hooks("panduck-diagram", "panduck-diagram", &plotter_types::DIAGRAM_THEME)
    {
        Ok(svg) => DiagramRenderResponse { success: true, svg: Some(svg), diagnostics },
        Err(error) => {
            diagnostics.push(DiagramDiagnostic {
                code: "plotter.scene.svg".to_string(),
                severity: "error".to_string(),
                message: error.to_string(),
                span_text: String::new(),
            });
            DiagramRenderResponse { success: false, svg: None, diagnostics }
        }
    }
}

fn diagram_diagnostic(diagnostic: &boost_diagnostic::Diagnostic) -> DiagramDiagnostic {
    DiagramDiagnostic {
        code: diagnostic.code().as_str().to_string(),
        severity: diagnostic.severity().to_string(),
        message: diagnostic
            .message()
            .fallback()
            .unwrap_or_else(|| diagnostic.message().key())
            .to_string(),
        span_text: String::new(),
    }
}

/// Install a JSON lines file sink on the global logger facade.
#[napi]
pub fn install_logger_log_file(path: String) -> Result<()> {
    boost_log::install_global_file_sink(&path).map_err(|error| Error::from_reason(error.to_string()))
}

/// Lists wired conversion routes as `from:to` strings.
#[napi]
pub fn supported_conversions() -> Vec<String> {
    supported_routes().into_iter().map(|(from, to)| format!("{from}:{to}")).collect()
}

/// Indexes a DOCX OPC package and returns member part paths.
#[napi]
pub fn inspect_index(input_path: String) -> InspectIndexResponse {
    let index = match inspect_docx_index(&input_path) {
        Ok(index) => index,
        Err(error) => {
            return InspectIndexResponse {
                exit_code: 1,
                format: String::new(),
                outer: String::new(),
                inner: String::new(),
                parts: Vec::new(),
                report_json: build_inspect_error_report("index", &input_path, &error),
            };
        }
    };
    let report_json = serde_json::json!({
        "schema_version": document_convert::SCHEMA_REPORT,
        "operation": "inspect",
        "status": "success",
        "inputs": [{ "path": input_path, "format": index.format }],
        "detection": {
            "outer": index.outer,
            "inner": index.inner,
            "format": index.format,
            "confidence": "verified",
        },
        "pipeline": { "stages": ["index"] },
        "parts": index.parts,
        "diagnostics": empty_diagnostic_envelope(),
    })
    .to_string();
    InspectIndexResponse {
        exit_code: 0,
        format: index.format,
        outer: index.outer,
        inner: index.inner,
        parts: index.parts,
        report_json,
    }
}

/// Decodes DOCX OPC members and returns per-part payload sizes.
#[napi]
pub fn inspect_decode(input_path: String, part_path: Option<String>) -> InspectDecodeResponse {
    let filter = part_path.as_deref();
    let decode = match inspect_docx_decode(&input_path, filter) {
        Ok(decode) => decode,
        Err(error) => {
            return InspectDecodeResponse {
                exit_code: 1,
                format: String::new(),
                outer: String::new(),
                inner: String::new(),
                parts: Vec::new(),
                report_json: build_inspect_error_report("decode", &input_path, &error),
            };
        }
    };
    let decoded_parts = decode
        .parts
        .iter()
        .map(|part| InspectDecodedPart {
            path: part.path.clone(),
            compression_method: part.compression_method,
            compressed_size: part.compressed_size.min(u32::MAX as u64) as u32,
            uncompressed_size: part.uncompressed_size.min(u32::MAX as u64) as u32,
            decoded_size: part.decoded_size.min(u32::MAX as u64) as u32,
        })
        .collect::<Vec<_>>();
    let report_json = serde_json::json!({
        "schema_version": document_convert::SCHEMA_REPORT,
        "operation": "inspect",
        "status": "success",
        "inputs": [{ "path": input_path, "format": decode.format }],
        "detection": {
            "outer": decode.outer,
            "inner": decode.inner,
            "format": decode.format,
            "confidence": "verified",
        },
        "pipeline": { "stages": ["decode"] },
        "decoded_parts": decode.parts.iter().map(|part| serde_json::json!({
            "path": part.path,
            "compression_method": part.compression_method,
            "compressed_size": part.compressed_size,
            "uncompressed_size": part.uncompressed_size,
            "decoded_size": part.decoded_size,
        })).collect::<Vec<_>>(),
        "diagnostics": empty_diagnostic_envelope(),
    })
    .to_string();
    InspectDecodeResponse {
        exit_code: 0,
        format: decode.format,
        outer: decode.outer,
        inner: decode.inner,
        parts: decoded_parts,
        report_json,
    }
}

/// Returns whether a source format can be converted into a Markdown project.
#[napi]
pub fn supports_markdown_project(from: String) -> bool {
    supports_markdown_project_source(&from)
}

/// Converts a document file into a Markdown project directory.
///
/// **Never throws** — see `convert_bytes_memory` for rationale.
///
/// `handlers_json` is an optional JSON-serialized plan descriptor from the
/// TS-side `ConversionPlan`. Only its `transforms` array is executed — policy
/// handler closures stay in TS and cannot cross the wire.
#[napi]
pub fn convert_markdown_project(
    from: String,
    input_path: String,
    output_dir: String,
    handlers_json: Option<String>,
    budgets_json: Option<String>,
) -> ConvertProjectResponse {
    let budgets = ConversionBudgets::from_json(budgets_json.as_deref());
    match convert_markdown_project_file_with_budgets(&from, &input_path, &output_dir, handlers_json.as_deref(), Some(&budgets))
    {
        Ok((output, published)) => ConvertProjectResponse {
            exit_code: 0,
            index_markdown: output.index_markdown,
            published_assets: output.published_assets,
            published_chapters: output.published_chapters,
            unresolved_assets: output.unresolved_assets,
            output_dir: published.output_dir.display().to_string(),
            report_json: output.report_json,
        },
        Err(adapter_error) => build_project_error_report(&from, &input_path, &output_dir, &adapter_error),
    }
}

/// Converts in-memory bytes and publishes a Markdown project directory.
///
/// This is the byte-input counterpart of `convert_markdown_project` so the
/// TS-side `ConversionPlan.publish` strategy can execute without a temp input
/// file. **Never throws** — failures come back as `exit_code: 1` with a
/// structured report envelope.
#[napi]
pub fn convert_markdown_project_bytes(
    from: String,
    label: String,
    bytes: Buffer,
    output_dir: String,
    handlers_json: Option<String>,
    budgets_json: Option<String>,
) -> ConvertProjectResponse {
    let budgets = ConversionBudgets::from_json(budgets_json.as_deref());
    match convert_markdown_project_bytes_impl(
        &from,
        &label,
        bytes.to_vec(),
        &output_dir,
        handlers_json.as_deref(),
        Some(&budgets),
    ) {
        Ok((output, published)) => ConvertProjectResponse {
            exit_code: 0,
            index_markdown: output.index_markdown,
            published_assets: output.published_assets,
            published_chapters: output.published_chapters,
            unresolved_assets: output.unresolved_assets,
            output_dir: published.output_dir.display().to_string(),
            report_json: output.report_json,
        },
        Err(adapter_error) => build_project_error_report(&from, &label, &output_dir, &adapter_error),
    }
}

/// Converts a document file through the Panduck IR pipeline.
///
/// **Never throws** — failures are reported via `exit_code: 1` and a report
/// JSON that carries the structured `DiagnosticEnvelope`. This avoids the
/// `Error::from_reason("[code=...] ...")` string-construction path where
/// structured diagnostic identity would be collapsed back into free-form text.
///
/// `handlers_json` is an optional JSON-serialized plan descriptor from the
/// TS-side `ConversionPlan`. Only its `transforms` array is executed — policy
/// handler closures stay in TS and cannot cross the wire.
#[napi]
pub fn convert_document(
    from: String,
    to: String,
    input_path: String,
    handlers_json: Option<String>,
    budgets_json: Option<String>,
) -> ConvertResponse {
    let budgets = ConversionBudgets::from_json(budgets_json.as_deref());
    match convert_file_with_budgets(&from, &to, &input_path, handlers_json.as_deref(), Some(&budgets)) {
        Ok(output) => ConvertResponse {
            exit_code: 0,
            text: if output.markdown.is_empty() { None } else { Some(output.markdown) },
            html: output.html,
            binary: output.binary.map(Buffer::from),
            report_json: output.report_json,
        },
        Err(adapter_error) => build_convert_error_report(&from, &to, &input_path, &adapter_error),
    }
}

/// Converts in-memory bytes through the Panduck IR pipeline.
///
/// **Never throws** — see `convert_document` for rationale. The failure path
/// always returns `exit_code: 1` with a report JSON that carries the full
/// structured diagnostic identity (code, typed args, origin, severity) so
/// downstream TS consumers never need to parse free-form error strings.
///
/// `handlers_json` is an optional JSON-serialized plan descriptor from the
/// TS-side `ConversionPlan`. Only its `transforms` array is executed — policy
/// handler closures stay in TS and cannot cross the wire.
#[napi]
pub fn convert_bytes_memory(
    from: String,
    to: String,
    label: String,
    bytes: Buffer,
    handlers_json: Option<String>,
    budgets_json: Option<String>,
) -> ConvertResponse {
    let budgets = ConversionBudgets::from_json(budgets_json.as_deref());
    match convert_bytes_with_budgets(&from, &to, &label, bytes.to_vec(), handlers_json.as_deref(), Some(&budgets)) {
        Ok(output) => ConvertResponse {
            exit_code: 0,
            text: if output.markdown.is_empty() { None } else { Some(output.markdown) },
            html: output.html,
            binary: output.binary.map(Buffer::from),
            report_json: output.report_json,
        },
        Err(adapter_error) => build_convert_error_report(&from, &to, &label, &adapter_error),
    }
}

/// Build a failure `ConvertResponse` where the report JSON carries a proper
/// `DiagnosticEnvelope` wrapping the structured `AdapterError::Diagnostic`.
/// This is the only legitimate path for convert failures — `map_adapter_error`
/// is still kept for legacy N-API entry points that genuinely use
/// `Result<ConvertResponse>`, but every new convert entry must route through
/// this helper instead of throwing.
fn build_convert_error_report(from: &str, to: &str, label: &str, error: &AdapterError) -> ConvertResponse {
    use boost_diagnostic::{DiagnosticEnvelope, DiagnosticSet};

    let diagnostic = document_convert::from_adapter_error(error);
    let mut set = DiagnosticSet::new();
    set.push(diagnostic);
    let envelope = DiagnosticEnvelope::from_set(&set);
    let report_json = serde_json::json!({
        "schema_version": document_convert::SCHEMA_REPORT,
        "operation": "convert",
        "status": "failed",
        "inputs": [{ "path": label, "format": from }],
        "pipeline": { "reader": from, "writer": to, "stages": ["read", "ir", "write"] },
        "diagnostics": envelope,
        "losses": [],
        "outputs": [],
    })
    .to_string();
    ConvertResponse { exit_code: 1, text: None, html: None, binary: None, report_json }
}

fn build_project_error_report(from: &str, input_path: &str, output_dir: &str, error: &AdapterError) -> ConvertProjectResponse {
    use boost_diagnostic::{DiagnosticEnvelope, DiagnosticSet};

    let diagnostic = document_convert::from_adapter_error(error);
    let mut set = DiagnosticSet::new();
    set.push(diagnostic);
    let envelope = DiagnosticEnvelope::from_set(&set);
    let report_json = serde_json::json!({
        "schema_version": document_convert::SCHEMA_REPORT,
        "operation": "convert",
        "status": "failed",
        "inputs": [{ "path": input_path, "format": from }],
        "pipeline": {
            "reader": from,
            "writer": "markdown_project",
            "stages": ["read", "ir", "markdown_project"],
            "output_dir": output_dir,
        },
        "diagnostics": envelope,
        "losses": [],
        "outputs": [],
    })
    .to_string();
    ConvertProjectResponse {
        exit_code: 1,
        index_markdown: String::new(),
        published_assets: Vec::new(),
        published_chapters: Vec::new(),
        unresolved_assets: Vec::new(),
        output_dir: String::new(),
        report_json,
    }
}

fn build_inspect_error_report(operation: &str, label: &str, error: &AdapterError) -> String {
    use boost_diagnostic::{DiagnosticEnvelope, DiagnosticSet};

    let diagnostic = document_convert::from_adapter_error(error);
    let mut set = DiagnosticSet::new();
    set.push(diagnostic);
    let envelope = DiagnosticEnvelope::from_set(&set);
    serde_json::json!({
        "schema_version": document_convert::SCHEMA_REPORT,
        "operation": "inspect",
        "status": "failed",
        "inputs": [{ "path": label }],
        "pipeline": { "stages": [operation] },
        "diagnostics": envelope,
        "losses": [],
    })
    .to_string()
}
