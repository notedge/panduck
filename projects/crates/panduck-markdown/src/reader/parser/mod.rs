#![doc = include_str!("readme.md")]

use crate::ast::MarkdownRoot;
use crate::reader::{MarkdownReadConfig, MarkdownTokenType};
use panduck_types::{AdapterError, Result};

pub struct MarkdownParser<'input> {
    pub(crate) state: panduck_types::parser::ParserState<'input, MarkdownTokenType>,
    pub(crate) config: &'input MarkdownReadConfig,
}

impl<'input> MarkdownParser<'input> {
    pub fn parse(&mut self) -> Result<MarkdownRoot> {
        Err(AdapterError::not_implemented("markdown parser"))
    }
}
