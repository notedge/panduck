use std::fs;
use std::path::Path;

use acorn_core::ParseBudget;
use acorn_docx::OpcPackage;
use panduck_types::{AdapterError, Result};

use crate::format_error;

/// Container index summary for DOCX packages.
#[derive(Debug, Clone)]
pub struct DocxInspectIndex {
    pub format: String,
    pub outer: String,
    pub inner: String,
    pub parts: Vec<String>,
}

/// Indexes a DOCX file on disk without reading document semantics.
pub fn inspect_docx_index(path: impl AsRef<Path>) -> Result<DocxInspectIndex> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|source| {
        AdapterError::io(source, Some(path.display().to_string()))
    })?;
    inspect_docx_index_bytes(path.display().to_string(), bytes)
}

/// One decoded OPC member summary.
#[derive(Debug, Clone)]
pub struct DocxDecodedPart {
    pub path: String,
    pub compression_method: u16,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub decoded_size: u64,
}

/// Decode-stage summary for DOCX packages.
#[derive(Debug, Clone)]
pub struct DocxInspectDecode {
    pub format: String,
    pub outer: String,
    pub inner: String,
    pub parts: Vec<DocxDecodedPart>,
}

/// Decodes DOCX package members on disk without projecting document semantics.
pub fn inspect_docx_decode(
    path: impl AsRef<Path>,
    part_filter: Option<&str>,
) -> Result<DocxInspectDecode> {
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
    if looks_like_ole(&bytes) {
        return format_error::fail(AdapterError::not_implemented(
            "legacy .doc OLE inspect is not available yet",
        ));
    }
    if !looks_like_zip(&bytes) {
        return format_error::fail(AdapterError::invalid_input("input is not a ZIP-based DOCX package"));
    }

    let label = label.into();
    let package = OpcPackage::open(label, bytes).map_err(map_opc_error)?;
    let budget = ParseBudget::default();
    let paths = match part_filter {
        Some(path) => vec![acorn_docx::normalize_part_path(path)],
        None => package.part_paths(),
    };

    let mut parts = Vec::new();
    for path in paths {
        let member = package.part(&path).ok_or_else(|| {
            AdapterError::invalid_input(format!("opc part not found: {path}"))
        })?;
        let decoded = package
            .read_part(&path, &budget)
            .map_err(map_opc_error)?;
        parts.push(DocxDecodedPart {
            path,
            compression_method: member.compression_method,
            compressed_size: member.compressed_size,
            uncompressed_size: member.uncompressed_size,
            decoded_size: decoded.len() as u64,
        });
    }

    Ok(DocxInspectDecode {
        format: "docx".into(),
        outer: "zip".into(),
        inner: "opc".into(),
        parts,
    })
}

/// Indexes DOCX bytes without reading document semantics.
pub fn inspect_docx_index_bytes(label: impl Into<String>, bytes: Vec<u8>) -> Result<DocxInspectIndex> {
    if looks_like_ole(&bytes) {
        return format_error::fail(AdapterError::not_implemented(
            "legacy .doc OLE inspect is not available yet",
        ));
    }
    if !looks_like_zip(&bytes) {
        return format_error::fail(AdapterError::invalid_input("input is not a ZIP-based DOCX package"));
    }

    let label = label.into();
    let package = OpcPackage::open(label, bytes).map_err(map_opc_error)?;
    Ok(DocxInspectIndex {
        format: "docx".into(),
        outer: "zip".into(),
        inner: "opc".into(),
        parts: package.part_paths(),
    })
}

fn looks_like_zip(bytes: &[u8]) -> bool {
    bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06")
}

fn looks_like_ole(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1])
}

fn map_opc_error(error: acorn_docx::OpcError) -> AdapterError {
    AdapterError::adapter("docx", error.to_string())
}
