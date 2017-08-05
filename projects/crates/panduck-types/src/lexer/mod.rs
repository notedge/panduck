#![doc = include_str!("readme.md")]

use crate::helpers::{SourcePosition, SourceText};
use crate::{reader::Token, AdapterError, Result};

pub trait TokenType: Copy {
    const END_OF_STREAM: Self;

    fn is_whitespace(&self) -> bool;

    fn is_ignored(&self) -> bool;
}

/// Lexer state for transitional Panduck-owned text adapters.
#[derive(Debug)]
pub struct LexerState<'input, T: TokenType> {
    source: &'input SourceText,
    tokens: Vec<Token<T>>,
    offset: usize,
}

impl<'input, T: TokenType> LexerState<'input, T> {
    pub fn new(input: &'input SourceText) -> Self {
        Self {
            source: input,
            tokens: Vec::new(),
            offset: 0,
        }
    }

    pub fn current_char(&self) -> Option<char> {
        self.source.get_char(self.offset).ok()
    }

    pub fn peek_char(&self) -> Option<char> {
        let step = self.current_char().map_or(0, |c| c.len_utf8());
        self.source.get_char(self.offset + step).ok()
    }

    pub fn advance(&mut self) {
        if let Some(c) = self.current_char() {
            self.offset += c.len_utf8();
        }
    }

    pub fn add_token(&mut self, token_type: T, start_offset: usize) {
        let position = SourcePosition {
            offset: start_offset,
            length: self.offset - start_offset,
        };
        self.tokens.push(Token {
            token_type,
            position,
        });
    }

    pub fn finish(mut self) -> Result<Vec<Token<T>>> {
        let position = SourcePosition {
            offset: self.source.utf8_length(),
            length: 0,
        };
        self.tokens.push(Token {
            token_type: T::END_OF_STREAM,
            position,
        });
        Ok(self.tokens)
    }

    pub fn fail(self, error: AdapterError) -> Result<Vec<Token<T>>> {
        Err(error)
    }

    pub fn offset(&self) -> usize {
        self.offset
    }
}
