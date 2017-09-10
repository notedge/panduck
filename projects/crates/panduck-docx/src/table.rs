use notedown_ir::{Block, DocumentGraph, Inline, TableRow};

/// Mutable table parse state for `w:tbl`/`w:tr`/`w:tc` walks.
#[derive(Debug, Default)]
pub(crate) struct TableState {
    pub rows: Vec<Vec<Vec<Inline>>>,
    pub current_row: Vec<Vec<Inline>>,
    pub current_cell: Vec<Inline>,
}

pub(crate) fn append_cell_paragraph(cell: &mut Vec<Inline>, content: &[Inline]) {
    if content.is_empty() {
        return;
    }
    if !cell.is_empty() {
        cell.push(Inline::Text { text: "\n".into() });
    }
    cell.extend(content.iter().cloned());
}

pub(crate) fn push_table_block(graph: &mut DocumentGraph, table: TableState) {
    if table.rows.is_empty() {
        return;
    }
    let rows = table
        .rows
        .into_iter()
        .map(|cells| TableRow { cells })
        .collect();
    graph.push_block(Block::Table { rows });
}
