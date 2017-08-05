#![doc = include_str!("readme.md")]

use crate::helpers::SourceText;
use crate::reader::Token;

/// Parser state for transitional Panduck-owned text adapters.
#[derive(Debug)]
pub struct ParserState<'input, T> {
    pub source: &'input SourceText,
    pub tokens: Vec<Token<T>>,
}

impl<'input, T> ParserState<'input, T> {
    pub fn new(source: &'input SourceText, tokens: Vec<Token<T>>) -> Self {
        Self { source, tokens }
    }
}
