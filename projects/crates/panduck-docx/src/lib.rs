#![warn(missing_docs)]
//! DOCX import into `notedown-ir::DocumentGraph`.

mod footnotes;
mod inspect;
mod numbering;
mod reader;
mod rels;
mod xml;

pub use inspect::{
    inspect_docx_decode, inspect_docx_decode_bytes, inspect_docx_index, inspect_docx_index_bytes,
    DocxDecodedPart, DocxInspectDecode, DocxInspectIndex,
};
pub use reader::{read_docx, read_docx_bytes};
