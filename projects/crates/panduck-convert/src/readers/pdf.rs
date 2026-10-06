use std::path::Path;

use notedown_formats::import::pdf::{import_pdf, import_pdf_bytes};
use notedown_ir::DocumentGraph;
use panduck_types::Result;

use crate::format_error::map_format_error;

/// Reads PDF text from disk into `DocumentGraph`.
pub fn read_pdf(path: impl AsRef<Path>) -> Result<DocumentGraph> { import_pdf(path).map_err(map_format_error) }

/// Reads PDF bytes into `DocumentGraph`.
pub fn read_pdf_bytes(label: impl Into<String>, bytes: Vec<u8>) -> Result<DocumentGraph> {
    let label = label.into();
    import_pdf_bytes(&label, &bytes).map_err(map_format_error)
}
