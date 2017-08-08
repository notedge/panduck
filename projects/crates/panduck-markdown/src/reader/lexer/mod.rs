#![doc = include_str!("readme.md")]

use crate::reader::token_type::MarkdownTokenType;
use crate::reader::{MarkdownReadConfig, MarkdownToken};
use panduck_types::{AdapterError, Result};

#[derive(Debug)]
pub struct MarkdownLexer<'input> {
    pub(crate) state: panduck_types::lexer::LexerState<'input, MarkdownTokenType>,
    pub(crate) config: &'input MarkdownReadConfig,
}

impl<'input> MarkdownLexer<'input> {
    pub fn tokenize(mut self) -> Result<Vec<MarkdownToken>> {
        Err(AdapterError::not_implemented("markdown lexer"))
    }

    fn next_token(&mut self) {
        let start_offset = self.state.offset();
        if let Some(c) = self.state.current_char() {
            match c {
                '#' => {
                    self.state.advance();
                    self.state.add_token(MarkdownTokenType::Hash, start_offset);
                }
                '*' => {
                    self.state.advance();
                    self.state.add_token(MarkdownTokenType::Star, start_offset);
                }
                '_' => {
                    self.state.advance();
                    self.state
                        .add_token(MarkdownTokenType::Underscore, start_offset);
                }
                '`' => {
                    let mut backtick_count = 0;
                    while self.state.current_char() == Some('`') && backtick_count < 3 {
                        backtick_count += 1;
                        self.state.advance();
                    }
                    if backtick_count == 3 {
                        self.state
                            .add_token(MarkdownTokenType::TripleBacktick, start_offset);
                    } else {
                        self.state
                            .add_token(MarkdownTokenType::Backtick, start_offset);
                    }
                }
                _ => {
                    self.state.advance();
                }
            }
        }
    }
}
