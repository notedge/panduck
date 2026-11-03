use std::collections::HashMap;

use notedown_formats::export::html::{RenderedDiagram, export_html_with_diagrams};
use notedown_ir::{DocumentGraph, NodeId};
use panduck_types::Result;

use crate::format_error::map_format_error;

/// Serializes a document graph to semantic HTML, embedding host-rendered
/// diagram scenes keyed by the authoring block. Diagram blocks without a
/// rendered scene keep the visible source fallback.
pub fn write_document_html(graph: &DocumentGraph, diagrams: &HashMap<NodeId, RenderedDiagram>) -> Result<String> {
    export_html_with_diagrams(graph, diagrams).map_err(map_format_error)
}
