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
mod generator;
mod lexer;
mod parser;
mod tex_processor;

pub use crate::ast::MarkdownRoot;
pub use crate::lexer::{LexerState, MarkdownToken, MarkdownTokenType};
pub use crate::parser::MarkdownReader;
pub use crate::generator::MarkdownWriter;
use panduck_core::helpers::SourceText;
use panduck_core::PanduckDiagnostics;

#[derive(Copy, Clone, Debug)]
pub struct MarkdownReadConfig {
    pub support_math: bool,
}

#[derive(Copy, Clone, Debug)]
pub struct MarkdownWriteConfig {}

pub fn parse_and_generate_markdown(markdown_input: &str, config: MarkdownReadConfig) -> PanduckDiagnostics<String> {
    let source = SourceText::new(markdown_input.to_string(), None);
    let lexer = lexer::MarkdownReader::new(config);
    let tokens = lexer.tokenize(&source)?;

    let parser = parser::MarkdownReader::new(config);
    let ast = parser.parse(&source, tokens)?;

    let generator = generator::MarkdownWriter::new();
    let html = generator.generate(ast);

    PanduckDiagnostics::success(html)
}
