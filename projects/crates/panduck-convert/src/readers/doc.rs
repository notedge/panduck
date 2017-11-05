use std::path::Path;

use notedown_formats::import::doc::{import_doc, import_doc_bytes};
use notedown_ir::DocumentGraph;
use panduck_types::Result;

use crate::format_error::map_format_error;

/// Reads legacy Word text from disk into `DocumentGraph`.
pub fn read_doc(path: impl AsRef<Path>) -> Result<DocumentGraph> { import_doc(path).map_err(map_format_error) }

/// Reads legacy Word bytes into `DocumentGraph`.
pub fn read_doc_bytes(label: impl Into<String>, bytes: Vec<u8>) -> Result<DocumentGraph> {
    let label = label.into();
    import_doc_bytes(&label, &bytes).map_err(map_format_error)
}
