#![warn(missing_docs)]
//! DOCX import into `notedown-ir::DocumentGraph`.

mod inspect;
mod numbering;
mod reader;
mod rels;
mod xml;

pub use inspect::{inspect_docx_index, inspect_docx_index_bytes, DocxInspectIndex};
pub use reader::{read_docx, read_docx_bytes};
