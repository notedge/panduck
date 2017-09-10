#![warn(missing_docs)]
#![doc = include_str!("../readme.md")]

mod footnotes;
mod inspect;
mod numbering;
mod reader;
mod rels;
mod table;
mod xml;

pub use inspect::{
    inspect_docx_decode, inspect_docx_decode_bytes, inspect_docx_index, inspect_docx_index_bytes,
    DocxDecodedPart, DocxInspectDecode, DocxInspectIndex,
};
pub use reader::{read_docx, read_docx_bytes};
