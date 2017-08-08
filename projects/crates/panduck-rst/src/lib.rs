#![deny(missing_debug_implementations, missing_copy_implementations)]
#![warn(missing_docs, rustdoc::missing_crate_level_docs)]
#![doc = include_str!("../readme.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/oovm/shape-rs/dev/projects/images/Trapezohedron.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/oovm/shape-rs/dev/projects/images/Trapezohedron.svg"
)]

mod ast;
mod reader;
mod writer;

pub use crate::ast::RstRoot;
pub use crate::reader::lexer::tokenize;
pub use crate::reader::token_type::{RstToken, RstTokenType};
pub use crate::reader::parser::{parse, ParserState};
pub use crate::writer::{generate, RstWriter};
use panduck_types::helpers::SourceText;
use panduck_types::Result;

/// Oak reStructuredText lexer/parser surface for this adapter.
pub mod oak {
    pub use oak_rst::{RstLanguage, RstLexer, RstParser, RstRoot};
}

/// Notedown document IR produced by Panduck readers.
pub mod ir {
    pub use notedown_ir::{
        Block, DocumentGraph, DocumentMetadata, IdAllocator, Inline, LossMarker, SemanticStatus,
    };
}

#[derive(Copy, Clone, Debug)]
pub struct RstReadConfig {
    pub support_math: bool,
}

#[derive(Copy, Clone, Debug)]
pub struct RstWriteConfig {}

pub fn parse_and_generate_rst(rst_input: &str, _config: RstReadConfig) -> Result<String> {
    let source = SourceText::new(rst_input, None);
    let tokens = tokenize(rst_input)?;
    let root = parse(tokens, source)?;
    generate(root)
}
