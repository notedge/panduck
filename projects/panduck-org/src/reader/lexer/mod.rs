#![doc = include_str!("readme.md")]

use crate::reader::token_type::OrgTokenType;
use crate::reader::{OrgReadConfig, OrgToken};
use panduck_core::PanduckDiagnostics;

#[derive(Debug)]
pub struct OrgLexer<'input> {
    pub(crate) state: panduck_core::lexer::LexerState<'input, OrgTokenType>,
    pub(crate) config: &'input OrgReadConfig,
}

impl<'input> OrgLexer<'input> {
    pub fn tokenize(mut self) -> PanduckDiagnostics<Vec<OrgToken>> {
        while self.state.current_char().is_some() {
            self.next_token();
        }
        self.state.success()
    }

    fn next_token(&mut self) {
        let start_offset = self.state.offset();
        if let Some(c) = self.state.current_char() {
            match c {
                '*' => {
                    let mut star_count = 0;
                    while self.state.current_char() == Some('*') {
                        star_count += 1;
                        self.state.advance();
                    }
                    // Check if it's a heading (starts at the beginning of a line)
                    if self.state.offset() - star_count == 0 || self.state.get_char_from_offset(self.state.offset() - star_count - 1) == Some('\n') {
                        self.state.add_token(OrgTokenType::Heading(star_count), start_offset);
                    } else {
                        // It's not a heading, treat as regular star for emphasis
                        for _ in 0..star_count {
                            self.state.add_token(OrgTokenType::Star, start_offset);
                        }
                    }
                }
                ':' => {
                    self.state.advance(); // Consume ':'
                    let mut identifier = String::new();
                    while let Some(c) = self.state.current_char() {
                        if c.is_ascii_alphanumeric() || c == '_' {
                            identifier.push(c);
                            self.state.advance();
                        } else {
                            break;
                        }
                    }
                    if self.state.current_char() == Some(':') {
                        self.state.advance(); // Consume ending ':'
                        if identifier.to_uppercase() == "END" {
                            self.state.add_token(OrgTokenType::DrawerEnd, start_offset);
                        } else {
                            self.state.add_token(OrgTokenType::DrawerStart(identifier), start_offset);
                        }
                    } else {
                        // Not a drawer, treat as text
                        self.state.add_token(OrgTokenType::Text(format!(":{}", identifier)), start_offset);
                    }
                }
                '#' => {
                    self.state.advance(); // Consume #
                    if self.state.current_char() == Some('+') {
                        self.state.advance(); // Consume +
                        let mut keyword_name = String::new();
                        while let Some(c) = self.state.current_char() {
                            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                                keyword_name.push(c);
                                self.state.advance();
                            } else {
                                break;
                            }
                        }
                        self.state.add_token(OrgTokenType::Keyword(keyword_name), start_offset);
                    } else {
                        self.state.add_token(OrgTokenType::Hash, start_offset);
                    }
                }
                '-' => {
                    // ... existing code ...
                }
                '`' => {
                    // ... existing code ...
                }
                '[' => {
                    // ... existing code ...
                }
                ']' => {
                    // ... existing code ...
                }
                '(' => {
                    // ... existing code ...
                }
                ')' => {
                    // ... existing code ...
                }
                '!' => {
                    // ... existing code ...
                }
                '-' => {
                    // ... existing code ...
                }
                '+' => {
                    // ... existing code ...
                }
                '.' => {
                    // ... existing code ...
                }
                '>' => {
                    // ... existing code ...
                }
                '\n' => {
                    // ... existing code ...
                }
                ' ' | '\t' => {
                    // ... existing code ...
                }
                _ if c.is_ascii_digit() => {
                    // ... existing code ...
                }
                '<' => {
                    self.state.advance();
                    let mut timestamp_content = String::new();
                    while let Some(c) = self.state.current_char() {
                        if c != '>' && c != '\n' {
                            timestamp_content.push(c);
                            self.state.advance();
                        } else {
                            break;
                        }
                    }
                    if self.state.current_char() == Some('>') {
                        self.state.advance();
                        self.state.add_token(OrgTokenType::Timestamp(timestamp_content), start_offset);
                    } else {
                        // Not a valid timestamp, treat as text
                        self.state.add_token(OrgTokenType::Text(format!("<{}", timestamp_content)), start_offset);
                    }
                }
                'D' => {
                    // 检查是否为 DEADLINE:
                    let deadline_str = "DEADLINE:";
                    let mut is_deadline = true;
                    for i in 0..deadline_str.len() {
                        if self.peek_char_n(i) != Some(deadline_str.chars().nth(i).unwrap()) {
                            is_deadline = false;
                            break;
                        }
                    }
                    if is_deadline {
                        for _ in 0..deadline_str.len() {
                            self.state.advance();
                        }
                        self.state.add_token(OrgTokenType::Planning("DEADLINE".to_string()), start_offset);
                    } else {
                        self.read_text(start_offset);
                    }
                }
                'S' => {
                    // 检查是否为 SCHEDULED:
                    let scheduled_str = "SCHEDULED:";
                    let mut is_scheduled = true;
                    for i in 0..scheduled_str.len() {
                        if self.peek_char_n(i) != Some(scheduled_str.chars().nth(i).unwrap()) {
                            is_scheduled = false;
                            break;
                        }
                    }
                    if is_scheduled {
                        for _ in 0..scheduled_str.len() {
                            self.state.advance();
                        }
                        self.state.add_token(OrgTokenType::Planning("SCHEDULED".to_string()), start_offset);
                    } else {
                        self.read_text(start_offset);
                    }
                }
                _ => {
                    self.read_text(start_offset);
                }
            }
        } else {
            // End of file, handled by success method
        }
    }

    fn peek_char_n(&self, n: usize) -> Option<char> {
        self.state.input[self.state.offset()..]
            .chars()
            .nth(n)
    }

    fn read_text(&mut self, start_offset: usize) {
        while let Some(c) = self.state.current_char() {
            match c {
                '#' | '*' | '_' | '`' | '[' | ']' | '(' | ')' | '!' | '-' | '+' | '.' | '>'
                | '\n' | ' ' | '\t' => break,
                _ if c.is_ascii_digit() => {
                    // If it's a digit, check if it's part of an ordered list marker
                    // This is a simplification; a full parser would handle this more robustly
                    let next_char_is_dot = self.state.peek_char() == Some('.');
                    if next_char_is_dot && self.state.offset() == start_offset {
                        // It's a potential ordered list marker, break to let next_token handle it
                        break;
                    } else if self.state.offset() == start_offset && c.is_ascii_digit() {
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
            self.state.add_token(OrgTokenType::Text, start_offset);
        }
    }
}