use notedown_formats::export::html::export_html;
use notedown_ir::DocumentGraph;
use panduck_types::Result;

use crate::format_error::map_format_error;

/// Serializes a document graph to semantic HTML.
pub fn write_document_html(graph: &DocumentGraph) -> Result<String> {
    export_html(graph).map_err(map_format_error)
}
