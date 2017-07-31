#![doc = include_str!("readme.md")]

use crate::helpers::SourceText;
use crate::reader::Token;
use crate::PanduckError;
use std::fmt::Debug;

#[derive(Debug)]
pub struct ParserState<'input, T> {
    source: &'input SourceText,
    tokens: Vec<Token<T>>,
    diagnostics: Vec<PanduckError>,
}

impl<'input, T> ParserState<'input, T> {
    pub fn new(source: &'input SourceText, tokens: Vec<Token<T>>) -> Self {
        Self {
            source,
            tokens,
            diagnostics: vec![],
        }
    }
}
