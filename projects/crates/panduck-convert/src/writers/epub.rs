use notedown_formats::export::epub::export_epub_bytes;
use notedown_ir::DocumentGraph;
use panduck_types::Result;

use crate::format_error::map_format_error;

/// Write `notedown-ir` to EPUB bytes via `notedown-formats`.
pub fn write_document_epub(graph: &DocumentGraph) -> Result<Vec<u8>> {
    export_epub_bytes(graph).map_err(map_format_error)
}
