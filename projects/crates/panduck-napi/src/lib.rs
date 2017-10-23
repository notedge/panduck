#![deny(missing_docs)]

//! Node-API export surface for Panduck.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use panduck_convert::{
    convert_file, inspect_docx_decode, inspect_docx_index, supported_routes, supports_route,
};
use panduck_types::AdapterError;

const FORMATS: [&str; 6] = ["markdown", "org", "rst", "tex", "docx", "epub"];

/// N-API conversion response.
#[napi(object)]
pub struct ConvertResponse {
    /// Process exit code. Zero means success.
    pub exit_code: u32,
    /// Text output for text targets such as `markdown`.
    pub markdown: Option<String>,
    /// Binary output for container targets such as `docx`.
    pub binary: Option<Buffer>,
    /// Serialized `panduck.report/v1` JSON payload.
    pub report_json: String,
}

/// N-API container index response.
#[napi(object)]
pub struct InspectIndexResponse {
    /// Detected container format.
    pub format: String,
    /// Outer container label from Acorn detection.
    pub outer: String,
    /// Inner payload label from Acorn detection.
    pub inner: String,
    /// OPC member paths discovered in the package.
    pub parts: Vec<String>,
    /// Serialized `panduck.report/v1` JSON payload.
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
    /// Detected container format.
    pub format: String,
    /// Outer container label from Acorn detection.
    pub outer: String,
    /// Inner payload label from Acorn detection.
    pub inner: String,
    /// Per-member decode summaries for the package.
    pub parts: Vec<InspectDecodedPart>,
    /// Serialized `panduck.report/v1` JSON payload.
    pub report_json: String,
}

/// Returns the Panduck N-API binding version.
#[napi]
pub fn panduck_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Lists format adapters wired into this workspace.
#[napi]
pub fn supported_formats() -> Vec<String> {
    FORMATS.iter().map(|name| (*name).to_string()).collect()
}

/// Returns whether a format name is recognized by Panduck adapters.
#[napi]
pub fn is_supported_format(name: String) -> bool {
    FORMATS
        .iter()
        .any(|format| format.eq_ignore_ascii_case(name.as_str()))
}

/// Returns whether a conversion route is wired in this build.
#[napi]
pub fn supports_conversion(from: String, to: String) -> bool {
    supports_route(&from, &to)
}

/// Install a JSON lines file sink on the global logger facade.
#[napi]
pub fn install_logger_log_file(path: String) -> Result<()> {
    logger::install_global_file_sink(&path).map_err(|error| Error::from_reason(error.to_string()))
}

/// Deprecated alias for [`install_logger_log_file`].
#[deprecated(since = "0.1.0", note = "use `install_logger_log_file` instead")]
#[napi]
pub fn install_console_log_file(path: String) -> Result<()> {
    install_logger_log_file(path)
}

/// Lists wired conversion routes as `from:to` strings.
#[napi]
pub fn supported_conversions() -> Vec<String> {
    supported_routes()
        .into_iter()
        .map(|(from, to)| format!("{from}:{to}"))
        .collect()
}

/// Indexes a DOCX OPC package and returns member part paths.
#[napi]
pub fn inspect_index(input_path: String) -> Result<InspectIndexResponse> {
    let index = inspect_docx_index(&input_path).map_err(map_adapter_error)?;
    let report_json = serde_json::json!({
        "schema_version": "panduck.report/v1",
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
    })
    .to_string();
    Ok(InspectIndexResponse {
        format: index.format,
        outer: index.outer,
        inner: index.inner,
        parts: index.parts,
        report_json,
    })
}

/// Decodes DOCX OPC members and returns per-part payload sizes.
#[napi]
pub fn inspect_decode(input_path: String, part_path: Option<String>) -> Result<InspectDecodeResponse> {
    let filter = part_path.as_deref();
    let decode = inspect_docx_decode(&input_path, filter).map_err(map_adapter_error)?;
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
        "schema_version": "panduck.report/v1",
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
    })
    .to_string();
    Ok(InspectDecodeResponse {
        format: decode.format,
        outer: decode.outer,
        inner: decode.inner,
        parts: decoded_parts,
        report_json,
    })
}

/// Converts a document file through the Panduck IR pipeline.
#[napi]
pub fn convert_document(from: String, to: String, input_path: String) -> Result<ConvertResponse> {
    let output = convert_file(&from, &to, &input_path).map_err(map_adapter_error)?;
    Ok(ConvertResponse {
        exit_code: 0,
        markdown: if output.markdown.is_empty() {
            None
        } else {
            Some(output.markdown)
        },
        binary: output.binary.map(Buffer::from),
        report_json: output.report_json,
    })
}

fn map_adapter_error(error: AdapterError) -> Error {
    Error::from_reason(error.to_string())
}
