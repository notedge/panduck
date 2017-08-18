use std::fs;
use std::path::Path;

use acorn_core::ParseBudget;
use acorn_docx::OpcPackage;
use notedown_ir::{DocumentGraph, LossMarker, SemanticStatus};
use panduck_types::{AdapterError, Result};

use crate::rels::read_document_relationships;
use crate::xml::{new_graph, parse_document_xml};

const DOCUMENT_XML: &str = "word/document.xml";

/// Reads a DOCX file from disk into `DocumentGraph`.
pub fn read_docx(path: impl AsRef<Path>) -> Result<DocumentGraph> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|source| {
        AdapterError::io(source, Some(path.display().to_string()))
    })?;
    read_docx_bytes(path.display().to_string(), bytes)
}

/// Reads DOCX bytes into `DocumentGraph`.
pub fn read_docx_bytes(label: impl Into<String>, bytes: Vec<u8>) -> Result<DocumentGraph> {
    if looks_like_ole(&bytes) {
        return Err(AdapterError::not_implemented(
            "legacy .doc OLE reader is not available yet; save as .docx",
        ));
    }
    if !looks_like_zip(&bytes) {
        return Err(AdapterError::invalid_input("input is not a ZIP-based DOCX package"));
    }

    let label = label.into();
    let package = OpcPackage::open(label.clone(), bytes).map_err(map_opc_error)?;
    let budget = ParseBudget::default();
    let xml = package
        .read_part(DOCUMENT_XML, &budget)
        .map_err(map_opc_error)?;

    let rels = read_document_relationships(&package, &budget)?;
    let mut graph = new_graph(&label);
    parse_document_xml(&xml, &rels, &mut graph)?;
    graph.push_loss(LossMarker {
        code: "reader.docx.partial_coverage".into(),
        message: "DOCX import currently maps paragraphs, heading styles, run bold/italic, hyperlinks, and embedded images".into(),
        status: SemanticStatus::Partial,
    });
    Ok(graph)
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
