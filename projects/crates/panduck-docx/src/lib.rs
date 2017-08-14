#![warn(missing_docs)]
//! DOCX import into `notedown-ir::DocumentGraph`.

mod reader;
mod rels;
mod xml;

pub use reader::{read_docx, read_docx_bytes};
