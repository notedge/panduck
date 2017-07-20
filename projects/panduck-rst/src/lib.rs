#![feature(try_trait_v2)]
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
pub use crate::reader::lexer::RstReader;
pub use crate::reader::lexer::tokenize;
pub use crate::reader::token_type::{RstToken, RstTokenType};
pub use crate::reader::parser::ParserState;
pub use crate::reader::parser::parse;
pub use crate::writer::generator::RstWriter;
use panduck_core::helpers::SourceText;
use panduck_core::PanduckDiagnostics;

#[derive(Copy, Clone, Debug)]
pub struct RstReadConfig {
    pub support_math: bool,
}

#[derive(Copy, Clone, Debug)]
pub struct RstWriteConfig {}

pub fn parse_and_generate_rst(rst_input: &str, config: RstReadConfig) -> PanduckDiagnostics<String> {
    let source = SourceText::new(rst_input);
    let lexer = reader::lexer::RstReader::new(config);
    let tokens_result = lexer.tokenize(&source);

    if tokens_result.has_errors() {
        return tokens_result.map(|_| String::new());
    }

    let parser = reader::parser::RstReader::new(config);
    let ast_result = parser.parse(&source, tokens_result.into_value());

    if ast_result.has_errors() {
        return ast_result.map(|_| String::new());
    }

    let generator = writer::generator::RstWriter::new();
    let html = generator.generate(ast_result.into_value());

    PanduckDiagnostics::new().success(html)
}
