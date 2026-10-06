use std::fs;
use std::path::Path;

use notedown_formats::import::html::import_html_bytes;
use notedown_ir::DocumentGraph;
use panduck_types::Result;

use crate::format_error::map_format_error;

/// Read an HTML document from a filesystem path into `notedown-ir`.
pub fn read_html(path: impl AsRef<Path>) -> Result<DocumentGraph> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|error| panduck_types::AdapterError::io(error, Some(path.display().to_string())))?;
    read_html_bytes(&path.display().to_string(), bytes)
}

/// Read HTML bytes into `notedown-ir`.
pub fn read_html_bytes(label: &str, bytes: Vec<u8>) -> Result<DocumentGraph> {
    import_html_bytes(label, &bytes).map_err(map_format_error)
}
