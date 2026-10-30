use notedown_formats::export::markdown::export_markdown;
use notedown_ir::DocumentGraph;
use panduck_types::Result;

use crate::format_error::map_format_error;

/// Serializes a `DocumentGraph` to Markdown text.
pub fn write_document_markdown(graph: &DocumentGraph) -> Result<String> {
    export_markdown(graph).map_err(map_format_error)
}
