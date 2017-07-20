#![doc = include_str!("readme.md")]

use crate::ast::MarkdownRoot;
use panduck_core::PanduckDiagnostics;

use crate::reader::{MarkdownReadConfig, MarkdownTokenType};

pub struct MarkdownParser<'input> {
    pub(crate) state: panduck_core::parser::ParserState<'input, MarkdownTokenType>,
    pub(crate) config: &'input MarkdownReadConfig,
}

impl<'input> MarkdownParser<'input> {
    pub fn parse(&mut self) -> PanduckDiagnostics<MarkdownRoot> {
        todo!()
    }
}
