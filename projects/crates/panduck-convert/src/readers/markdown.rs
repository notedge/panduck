use std::path::Path;

use notedown_formats::import::markdown::{import_markdown, import_markdown_bytes};
use notedown_ir::DocumentGraph;
use panduck_types::Result;

use crate::format_error::map_format_error;

/// Reads Markdown text from disk into `DocumentGraph`.
pub fn read_markdown(path: impl AsRef<Path>) -> Result<DocumentGraph> {
    import_markdown(path).map_err(map_format_error)
}

/// Reads Markdown bytes as UTF-8 into `DocumentGraph`.
pub fn read_markdown_bytes(label: impl Into<String>, text: String) -> Result<DocumentGraph> {
    let label = label.into();
    import_markdown_bytes(&label, &text).map_err(map_format_error)
}
