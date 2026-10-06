use notedown_formats::export::markdown_project::{export_markdown_project, MarkdownProject};
use notedown_ir::DocumentGraph;
use panduck_types::Result;

use crate::format_error::map_format_error;

/// Builds a Markdown project payload from `notedown-ir`.
pub fn write_markdown_project(graph: &DocumentGraph) -> Result<MarkdownProject> {
    export_markdown_project(graph).map_err(map_format_error)
}
