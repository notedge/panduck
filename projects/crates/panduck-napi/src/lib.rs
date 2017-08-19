//! Node-API export surface for Panduck.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use panduck_convert::{convert_file, supported_routes, supports_route};
use panduck_docx::inspect_docx_index;
use panduck_types::AdapterError;

const FORMATS: [&str; 5] = ["markdown", "org", "rst", "tex", "docx"];

/// N-API conversion response.
#[napi(object)]
pub struct ConvertResponse {
    pub exit_code: u32,
    pub markdown: Option<String>,
    pub report_json: String,
}

/// N-API container index response.
#[napi(object)]
pub struct InspectIndexResponse {
    pub format: String,
    pub outer: String,
    pub inner: String,
    pub parts: Vec<String>,
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

/// Converts a document file through the Panduck IR pipeline.
#[napi]
pub fn convert_document(from: String, to: String, input_path: String) -> Result<ConvertResponse> {
    let output = convert_file(&from, &to, &input_path).map_err(map_adapter_error)?;
    Ok(ConvertResponse {
        exit_code: 0,
        markdown: Some(output.markdown),
        report_json: output.report_json,
    })
}

fn map_adapter_error(error: AdapterError) -> Error {
    Error::from_reason(error.to_string())
}
