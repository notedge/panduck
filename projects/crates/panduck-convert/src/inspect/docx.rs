use std::{fs, path::Path};

use notedown_formats::{InspectBudget, inspect_docx_bytes};
use panduck_types::{AdapterError, Result};

use crate::format_error;

/// Container index summary for DOCX packages.
#[derive(Debug, Clone)]
pub struct DocxInspectIndex {
    /// Detected container format, typically `docx`.
    pub format: String,
    /// Outer container label from the format layer.
    pub outer: String,
    /// Inner payload label from the format layer.
    pub inner: String,
    /// OPC member paths discovered in the package.
    pub parts: Vec<String>,
}

/// Indexes a DOCX file on disk without reading document semantics.
pub fn inspect_docx_index(path: impl AsRef<Path>) -> Result<DocxInspectIndex> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|source| AdapterError::io(source, Some(path.display().to_string())))?;
    inspect_docx_index_bytes(path.display().to_string(), bytes)
}

/// One decoded OPC member summary.
#[derive(Debug, Clone)]
pub struct DocxDecodedPart {
    /// OPC member path inside the package.
    pub path: String,
    /// ZIP compression method identifier.
    pub compression_method: u16,
    /// Compressed byte length stored in the archive.
    pub compressed_size: u64,
    /// Uncompressed byte length declared in the archive.
    pub uncompressed_size: u64,
    /// Decoded payload byte length after inflation.
    pub decoded_size: u64,
}

/// Decode-stage summary for DOCX packages.
#[derive(Debug, Clone)]
pub struct DocxInspectDecode {
    /// Detected container format, typically `docx`.
    pub format: String,
    /// Outer container label from the format layer.
    pub outer: String,
    /// Inner payload label from the format layer.
    pub inner: String,
    /// Per-member decode summaries for the package.
    pub parts: Vec<DocxDecodedPart>,
}

/// Decodes DOCX package members on disk without projecting document semantics.
pub fn inspect_docx_decode(path: impl AsRef<Path>, part_filter: Option<&str>) -> Result<DocxInspectDecode> {
    let path = path.as_ref();
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(source) => {
            return format_error::fail(AdapterError::io(source, Some(path.display().to_string())));
        }
    };
    format_error::propagate(inspect_docx_decode_bytes(path.display().to_string(), bytes, part_filter))
}

/// Decodes DOCX package members without projecting document semantics.
pub fn inspect_docx_decode_bytes(
    label: impl Into<String>,
    bytes: Vec<u8>,
    part_filter: Option<&str>,
) -> Result<DocxInspectDecode> {
    let label = label.into();
    let inspect = inspect_docx_bytes(&label, &bytes, part_filter, true, InspectBudget::default())
        .map_err(format_error::map_format_error)?;
    let parts = inspect
        .parts
        .into_iter()
        .map(|part| DocxDecodedPart {
            path: part.path,
            compression_method: part.compression_method,
            compressed_size: part.compressed_size,
            uncompressed_size: part.uncompressed_size,
            decoded_size: part.decoded_size.unwrap_or_default(),
        })
        .collect();
    Ok(DocxInspectDecode { format: inspect.format, outer: inspect.outer, inner: inspect.inner, parts })
}

/// Indexes DOCX bytes without reading document semantics.
pub fn inspect_docx_index_bytes(label: impl Into<String>, bytes: Vec<u8>) -> Result<DocxInspectIndex> {
    let label = label.into();
    let inspect = inspect_docx_bytes(&label, &bytes, None, false, InspectBudget::default())
        .map_err(format_error::map_format_error)?;
    Ok(DocxInspectIndex {
        format: inspect.format,
        outer: inspect.outer,
        inner: inspect.inner,
        parts: inspect.parts.into_iter().map(|part| part.path).collect(),
    })
}
