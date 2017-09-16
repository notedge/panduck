use notedown_formats::export::docx::export_docx_bytes;
use notedown_ir::DocumentGraph;
use panduck_types::Result;

use crate::format_error::map_format_error;

/// Write `notedown-ir` to DOCX bytes via `notedown-formats`.
pub fn write_document_docx(graph: &DocumentGraph) -> Result<Vec<u8>> {
    export_docx_bytes(graph).map_err(map_format_error)
}
