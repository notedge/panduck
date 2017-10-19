#![doc = include_str!("readme.md")]

use crate::helpers::SourceText;
use crate::reader::Token;

/// Parser state for transitional Panduck-owned text adapters.
#[derive(Debug)]
pub struct ParserState<'input, T> {
    /// Source text backing the parse.
    pub source: &'input SourceText,
    /// Token stream produced by the lexer stage.
    pub tokens: Vec<Token<T>>,
}

impl<'input, T> ParserState<'input, T> {
    /// Binds parser state to `source` and a pre-lexed `tokens` vector.
    pub fn new(source: &'input SourceText, tokens: Vec<Token<T>>) -> Self {
        Self { source, tokens }
    }
}
