use notedown_formats::export::pdf::export_pdf_bytes;
use notedown_ir::DocumentGraph;
use panduck_types::Result;

use crate::format_error::map_format_error;

/// Writes `notedown-ir` to PDF bytes.
pub fn write_document_pdf(graph: &DocumentGraph) -> Result<Vec<u8>> { export_pdf_bytes(graph).map_err(map_format_error) }
