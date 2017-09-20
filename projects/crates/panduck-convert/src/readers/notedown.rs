use std::path::Path;

use notedown_formats::import::notedown::{import_notedown, import_notedown_bytes};
use notedown_ir::DocumentGraph;
use panduck_types::Result;

use crate::format_error::map_format_error;

/// Reads Notedown text from disk into `DocumentGraph`.
pub fn read_notedown(path: impl AsRef<Path>) -> Result<DocumentGraph> {
    import_notedown(path).map_err(map_format_error)
}

/// Reads Notedown bytes as UTF-8 into `DocumentGraph`.
pub fn read_notedown_bytes(label: impl Into<String>, text: String) -> Result<DocumentGraph> {
    let label = label.into();
    import_notedown_bytes(&label, &text).map_err(map_format_error)
}
