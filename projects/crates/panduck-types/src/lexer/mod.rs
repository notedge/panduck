#![doc = include_str!("readme.md")]

use crate::helpers::{SourcePosition, SourceText};
use crate::{reader::Token, AdapterError, Result};

/// Token classification contract for [`LexerState`].
pub trait TokenType: Copy {
    /// Sentinel token emitted once at end-of-stream.
    const END_OF_STREAM: Self;

    /// Whether the token should be treated as whitespace during lexing.
    fn is_whitespace(&self) -> bool;

    /// Whether the token should be skipped by higher-level consumers.
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
    /// Creates lexer state at the start of `input`.
    pub fn new(input: &'input SourceText) -> Self {
        Self {
            source: input,
            tokens: Vec::new(),
            offset: 0,
        }
    }

    /// Returns the scalar at the current offset, if in range.
    pub fn current_char(&self) -> Option<char> {
        self.source.get_char(self.offset).ok()
    }

    /// Returns the scalar after the current code point, if in range.
    pub fn peek_char(&self) -> Option<char> {
        let step = self.current_char().map_or(0, |c| c.len_utf8());
        self.source.get_char(self.offset + step).ok()
    }

    /// Advances the cursor by one UTF-8 scalar.
    pub fn advance(&mut self) {
        if let Some(c) = self.current_char() {
            self.offset += c.len_utf8();
        }
    }

    /// Records a token spanning `start_offset` through the current cursor.
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

    /// Appends an end-of-stream token and returns the collected token vector.
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

    /// Short-circuits lexing with a fatal adapter error.
    pub fn fail(self, error: AdapterError) -> Result<Vec<Token<T>>> {
        Err(error)
    }

    /// Returns the current byte offset in the source text.
    pub fn offset(&self) -> usize {
        self.offset
    }
}
