#![deny(missing_docs)]

//! Node-API export surface for Panduck.

use std::collections::BTreeSet;

use napi::bindgen_prelude::*;
use napi_derive::napi;
use panduck_convert::{
    READERS, WRITERS, convert_bytes, convert_file, convert_markdown_project_file,
    convert_to_markdown_project as convert_markdown_project_bytes_impl, empty_diagnostic_envelope, inspect_docx_decode,
    inspect_docx_index, supported_routes, supports_markdown_project_source, supports_route,
};
use panduck_types::AdapterError;

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
        "schema_version": panduck_convert::SCHEMA_REPORT,
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
        "schema_version": panduck_convert::SCHEMA_REPORT,
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
) -> ConvertProjectResponse {
    match convert_markdown_project_file(&from, &input_path, &output_dir, handlers_json.as_deref()) {
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
) -> ConvertProjectResponse {
    match convert_markdown_project_bytes_impl(&from, &label, bytes.to_vec(), &output_dir, handlers_json.as_deref()) {
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
pub fn convert_document(from: String, to: String, input_path: String, handlers_json: Option<String>) -> ConvertResponse {
    match convert_file(&from, &to, &input_path, handlers_json.as_deref()) {
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
) -> ConvertResponse {
    match convert_bytes(&from, &to, &label, bytes.to_vec(), handlers_json.as_deref()) {
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

    let diagnostic = panduck_convert::from_adapter_error(error);
    let mut set = DiagnosticSet::new();
    set.push(diagnostic);
    let envelope = DiagnosticEnvelope::from_set(&set);
    let report_json = serde_json::json!({
        "schema_version": panduck_convert::SCHEMA_REPORT,
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

    let diagnostic = panduck_convert::from_adapter_error(error);
    let mut set = DiagnosticSet::new();
    set.push(diagnostic);
    let envelope = DiagnosticEnvelope::from_set(&set);
    let report_json = serde_json::json!({
        "schema_version": panduck_convert::SCHEMA_REPORT,
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

    let diagnostic = panduck_convert::from_adapter_error(error);
    let mut set = DiagnosticSet::new();
    set.push(diagnostic);
    let envelope = DiagnosticEnvelope::from_set(&set);
    serde_json::json!({
        "schema_version": panduck_convert::SCHEMA_REPORT,
        "operation": "inspect",
        "status": "failed",
        "inputs": [{ "path": label }],
        "pipeline": { "stages": [operation] },
        "diagnostics": envelope,
        "losses": [],
    })
    .to_string()
}
