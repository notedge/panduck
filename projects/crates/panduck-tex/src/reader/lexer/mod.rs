#![doc = include_str!("readme.md")]

use crate::{MarkdownReadConfig, MarkdownReader};
use panduck_types::helpers::SourceText;

mod token_type;
pub use self::token_type::{MarkdownToken, MarkdownTokenType};
use panduck_types::PanduckDiagnostics;

#[derive(Debug)]
pub struct LexerState<'input> {
    state: panduck_types::lexer::LexerState<'input, MarkdownTokenType>,
    config: &'input MarkdownReadConfig,
}

impl<'input> LexerState<'input> {
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
                    self.state.add_token(MarkdownTokenType::Underscore, start_offset);
                }
                '`' => {
                    let mut backtick_count = 0;
                    while self.state.current_char() == Some('`') && backtick_count < 3 {
                        backtick_count += 1;
                        self.state.advance();
                    }
                    if backtick_count == 3 {
                        self.state.add_token(MarkdownTokenType::TripleBacktick, start_offset);
                    } else {
                        self.state.add_token(MarkdownTokenType::Backtick, start_offset);
                    }
                }
                '[' => {
                    self.state.advance();
                    self.state.add_token(MarkdownTokenType::OpenBracket, start_offset);
                }
                ']' => {
                    self.state.advance();
                    self.state.add_token(MarkdownTokenType::CloseBracket, start_offset);
                }
                '(' => {
                    self.state.advance();
                    self.state.add_token(MarkdownTokenType::OpenParen, start_offset);
                }
                ')' => {
                    self.state.advance();
                    self.state.add_token(MarkdownTokenType::CloseParen, start_offset);
                }
                '!' => {
                    self.state.advance();
                    self.state.add_token(MarkdownTokenType::ExclamationMark, start_offset);
                }
                '-' => {
                    self.state.advance();
                    self.state.add_token(MarkdownTokenType::Minus, start_offset);
                }
                '+' => {
                    self.state.advance();
                    self.state.add_token(MarkdownTokenType::Plus, start_offset);
                }
                '.' => {
                    self.state.advance();
                    self.state.add_token(MarkdownTokenType::Dot, start_offset);
                }
                '>' => {
                    self.state.advance();
                    self.state.add_token(MarkdownTokenType::GreaterThan, start_offset);
                }
                '\n' => {
                    self.state.advance();
                    self.state
                        .add_token(MarkdownTokenType::Newline, start_offset);
                }
                ' ' | '\t' => {
                    while let Some(ws_char) = self.state.current_char() {
                        if ws_char.is_whitespace() && ws_char != '\n' {
                            self.state.advance();
                        } else {
                            break;
                        }
                    }
                    self.state
                        .add_token(MarkdownTokenType::Whitespace, start_offset);
                }
                _ if c.is_ascii_digit() => {
                    while let Some(d) = self.state.current_char() {
                        if d.is_ascii_digit() {
                            self.state.advance();
                        } else {
                            break;
                        }
                    }
                    self.state.add_token(MarkdownTokenType::Number, start_offset);
                }
                _ => {
                    self.read_text(start_offset);
                }
            }
        } else {
            // End of file, handled by success method
        }
    }

    fn read_text(&mut self, start_offset: usize) {
        while let Some(c) = self.state.current_char() {
            match c {
                '#' | '*' | '_' | '`' | '[' | ']' | '(' | ')' | '!' | '-' | '+' | '.' | '>' | '\n' | ' ' | '\t' => break,
                _ if c.is_ascii_digit() => {
                    // If it's a digit, check if it's part of an ordered list marker
                    // This is a simplification; a full parser would handle this more robustly
                    let next_char_is_dot = self.state.peek_char() == Some('.');
                    if next_char_is_dot && self.state.offset() == start_offset {
                        // It's a potential ordered list marker, break to let next_token handle it
                        break;
                    } else if self.state.offset() == start_offset && c.is_ascii_digit() {
                        // If it's the start of a number that's not an ordered list, treat as text
                        self.state.advance();
                    } else if c.is_ascii_digit() {
                        self.state.advance();
                    } else {
                        break;
                    }
                }
                _ => self.state.advance(),
            }
        }
        if self.state.offset() > start_offset {
            self.state.add_token(MarkdownTokenType::Text, start_offset);
        }
    }
}

impl MarkdownReader {
    pub fn tokenize(&self, input: &SourceText) -> PanduckDiagnostics<Vec<MarkdownToken>> {
        let mut state = LexerState {
            state: panduck_types::lexer::LexerState::new(input),
            config: &self.config,
        };

        while state.state.current_char().is_some() {
            state.next_token();
        }

        state.state.success()
    }
}
