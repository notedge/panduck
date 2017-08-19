use std::fs;
use std::path::Path;

use acorn_docx::OpcPackage;
use panduck_types::{AdapterError, Result};

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

/// Indexes DOCX bytes without reading document semantics.
pub fn inspect_docx_index_bytes(label: impl Into<String>, bytes: Vec<u8>) -> Result<DocxInspectIndex> {
    if looks_like_ole(&bytes) {
        return Err(AdapterError::not_implemented(
            "legacy .doc OLE inspect is not available yet",
        ));
    }
    if !looks_like_zip(&bytes) {
        return Err(AdapterError::invalid_input("input is not a ZIP-based DOCX package"));
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
