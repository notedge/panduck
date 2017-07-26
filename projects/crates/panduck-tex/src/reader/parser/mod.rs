#![doc = include_str!("readme.md")]

use crate::ast::{MarkdownBlock, MarkdownBlockCode, MarkdownHeading, MarkdownInline, MarkdownList, MarkdownListItem, MarkdownParagraph, MarkdownRoot};
use crate::lexer::{MarkdownToken, MarkdownTokenType};
use crate::MarkdownReadConfig;
use panduck_types::helpers::SourceText;
use panduck_types::PanduckDiagnostics;

#[derive(Copy, Clone, Debug)]
pub struct MarkdownReader {
    pub(crate) config: MarkdownReadConfig,
}

struct ParserState<'input> {
    source: &'input SourceText,
    tokens: Vec<MarkdownToken>,
    current_token_index: usize,
    diagnostics: PanduckDiagnostics<()>, // Add diagnostics field
}

impl<'input> ParserState<'input> {
    fn new(source: &'input SourceText, tokens: Vec<MarkdownToken>) -> Self {
        Self {
            source,
            tokens,
            current_token_index: 0,
            diagnostics: PanduckDiagnostics::new(),
        }
    }

    fn current_token(&self) -> Option<&MarkdownToken> {
        self.tokens.get(self.current_token_index)
    }

    fn advance(&mut self) {
        self.current_token_index += 1;
    }

    fn peek_token(&self, offset: usize) -> Option<&MarkdownToken> {
        self.tokens.get(self.current_token_index + offset)
    }

    fn eat_token(&mut self, expected_type: MarkdownTokenType) -> bool {
        if let Some(token) = self.current_token() {
            if token.token_type == expected_type {
                self.advance();
                return true;
            }
        }
        false
    }

    fn parse_document(&mut self) -> MarkdownRoot {
        let mut blocks = Vec::new();
        while self.current_token().is_some() && self.current_token().unwrap().token_type != MarkdownTokenType::EndOfFile {
            if let Some(block) = self.parse_block() {
                blocks.push(block);
            } else {
                // Skip unknown tokens to avoid infinite loops
                self.advance();
            }
        }
        MarkdownRoot { blocks }
    }

    fn parse_block(&mut self) -> Option<MarkdownBlock> {
        let token = self.current_token()?;
        match token.token_type {
            MarkdownTokenType::Hash => self.parse_heading().map(MarkdownBlock::Heading),
            MarkdownTokenType::TripleBacktick => self.parse_block_code().map(MarkdownBlock::BlockCode),
            MarkdownTokenType::Minus | MarkdownTokenType::Plus => self.parse_list().map(MarkdownBlock::List),
            MarkdownTokenType::Number if self.peek_token(1)?.token_type == MarkdownTokenType::Dot => self.parse_list().map(MarkdownBlock::List),
            MarkdownTokenType::Newline | MarkdownTokenType::Whitespace => {
                self.advance();
                None // Skip newlines and whitespace between blocks for now
            }
            _ => self.parse_paragraph().map(MarkdownBlock::Paragraph),
        }
    }

    fn parse_block_code(&mut self) -> Option<MarkdownBlockCode> {
        self.eat_token(MarkdownTokenType::TripleBacktick); // Consume opening triple backtick
        let lang = if let Some(MarkdownTokenType::Text) = self.current_token()?.token_type { 
            let lang = self.source.get_text_slice(self.current_token()?.text_range.clone()).to_string();
            self.advance();
            Some(lang)
        } else { None };
        self.eat_token(MarkdownTokenType::Newline);

        let mut content = String::new();
        while let Some(token) = self.current_token() {
            if token.token_type == MarkdownTokenType::TripleBacktick {
                break;
            }
            content.push_str(&self.source.get_str(token.position.offset..(token.position.offset + token.position.length))?);
            self.advance();
        }
        self.eat_token(MarkdownTokenType::TripleBacktick); // Consume closing triple backtick
        Some(MarkdownBlockCode { language: lang, content })
    }

    fn parse_list(&mut self) -> Option<MarkdownList> {
        let mut items = Vec::new();
        while let Some(token) = self.current_token() {
            match token.token_type {
                MarkdownTokenType::Minus | MarkdownTokenType::Plus => {
                    self.advance(); // Consume list marker
                    self.eat_token(MarkdownTokenType::Whitespace);
                    items.push(self.parse_list_item()?);
                }
                MarkdownTokenType::Number if self.peek_token(1)?.token_type == MarkdownTokenType::Dot => {
                    self.advance(); // Consume number
                    self.advance(); // Consume dot
                    self.eat_token(MarkdownTokenType::Whitespace);
                    items.push(self.parse_list_item()?);
                }
                _ => break,
            }
        }
        Some(MarkdownList { items })
    }

    fn parse_list_item(&mut self) -> Option<MarkdownListItem> {
        let content = self.parse_inline_content_until_newline();
        self.eat_token(MarkdownTokenType::Newline);
        Some(MarkdownListItem { content })
    }

    fn parse_heading(&mut self) -> Option<MarkdownHeading> {
        let mut level = 0;
        while self.eat_token(MarkdownTokenType::Hash) {
            level += 1;
        }

        if level == 0 || level > 6 {
            // Invalid heading level
            return None;
        }

        // Consume optional whitespace after hashes
        self.eat_token(MarkdownTokenType::Whitespace);

        let content = self.parse_inline_content_until_newline();

        // Consume the newline at the end of the heading
        self.eat_token(MarkdownTokenType::Newline);

        Some(MarkdownHeading { level, content })
    }

    fn parse_paragraph(&mut self) -> Option<MarkdownParagraph> {
        let content = self.parse_inline_content_until_newline();
        if content.is_empty() {
            return None;
        }
        // Consume the newline at the end of the paragraph
        self.eat_token(MarkdownTokenType::Newline);
        Some(MarkdownParagraph { content })
    }

    fn parse_inline_content_until_newline(&mut self) -> Vec<MarkdownInline> {
        let mut content = Vec::new();
        while let Some(token) = self.current_token() {
            if token.token_type == MarkdownTokenType::Newline || token.token_type == MarkdownTokenType::EndOfFile {
                break;
            }
            if let Some(inline) = self.parse_inline() {
                content.push(inline);
            } else {
                // If parse_inline returns None, it means it couldn't parse the current token as an inline element.
                // In this case, we should advance to prevent an infinite loop and potentially add a diagnostic.
                self.advance();
            }
        }
        content
    }

    fn parse_inline(&mut self) -> Option<MarkdownInline> {
        let token = self.current_token()?;
        match token.token_type {
            MarkdownTokenType::Text => {
                let text = self.source.get_text_slice(token.position.clone());
                self.advance();
                Some(MarkdownInline::Text(text.to_string()))
            }
            MarkdownTokenType::Star => {
                self.advance(); // Consume the first star
                if self.current_token()?.token_type == MarkdownTokenType::Star {
                    self.advance(); // Consume the second star for bold
                    let inner_content = self.parse_inline_content_until_closing_delimiter(MarkdownTokenType::Star, 2);
                    self.eat_token(MarkdownTokenType::Star); // Consume closing star
                    self.eat_token(MarkdownTokenType::Star); // Consume closing star
                    Some(MarkdownInline::Bold(inner_content))
                } else {
                    let inner_content = self.parse_inline_content_until_closing_delimiter(MarkdownTokenType::Star, 1);
                    self.eat_token(MarkdownTokenType::Star); // Consume closing star
                    Some(MarkdownInline::Italic(inner_content))
                }
            }
            MarkdownTokenType::Underscore => {
                self.advance(); // Consume the first underscore
                if self.current_token()?.token_type == MarkdownTokenType::Underscore {
                    self.advance(); // Consume the second underscore for bold
                    let inner_content = self.parse_inline_content_until_closing_delimiter(MarkdownTokenType::Underscore, 2);
                    self.eat_token(MarkdownTokenType::Underscore); // Consume closing underscore
                    self.eat_token(MarkdownTokenType::Underscore); // Consume closing underscore
                    Some(MarkdownInline::Bold(inner_content))
                } else {
                    let inner_content = self.parse_inline_content_until_closing_delimiter(MarkdownTokenType::Underscore, 1);
                    self.eat_token(MarkdownTokenType::Underscore); // Consume closing underscore
                    Some(MarkdownInline::Italic(inner_content))
                }
            }
            MarkdownTokenType::Backtick => {
                self.advance(); // Consume the backtick
                let start_offset = self.current_token_index;
                while let Some(t) = self.current_token() {
                    if t.token_type == MarkdownTokenType::Backtick {
                        break;
                    }
                    self.advance();
                }
                let end_offset = self.current_token_index;
                let code_tokens = &self.tokens[start_offset..end_offset];
                let code_text = code_tokens.iter().map(|t| self.source.get_str(t.position.offset..(t.position.offset + t.position.length))).collect::<Result<Vec<&str>, PanduckError>>()?.join("");
                self.eat_token(MarkdownTokenType::Backtick); // Consume closing backtick
                Some(MarkdownInline::Code(code_text))
            }
            MarkdownTokenType::ExclamationMark => {
                self.advance(); // Consume '!'
                if self.eat_token(MarkdownTokenType::OpenBracket) {
                    let alt_text_tokens_start = self.current_token_index;
                    while let Some(t) = self.current_token() {
                        if t.token_type == MarkdownTokenType::CloseBracket {
                            break;
                        }
                        self.advance();
                    }
                    let alt_text_tokens_end = self.current_token_index;
                    let alt_text = self.tokens[alt_text_tokens_start..alt_text_tokens_end].iter().map(|t| self.source.get_str(t.position.offset..(t.position.offset + t.position.length))).collect::<Result<Vec<&str>, PanduckError>>()?.join("");
                    self.eat_token(MarkdownTokenType::CloseBracket);
                    if self.eat_token(MarkdownTokenType::OpenParen) {
                        let url_tokens_start = self.current_token_index;
                        while let Some(t) = self.current_token() {
                            if t.token_type == MarkdownTokenType::CloseParen {
                                break;
                            }
                            self.advance();
                        }
                        let url_tokens_end = self.current_token_index;
                        let url = self.tokens[url_tokens_start..url_tokens_end].iter().map(|t| self.source.get_str(t.position.offset..(t.position.offset + t.position.length))).collect::<Result<Vec<&str>, PanduckError>>()?.join("");
                        self.eat_token(MarkdownTokenType::CloseParen);
                        Some(MarkdownInline::Image(alt_text, url))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            MarkdownTokenType::OpenBracket => {
                self.advance(); // Consume '['
                let link_text_tokens_start = self.current_token_index;
                while let Some(t) = self.current_token() {
                    if t.token_type == MarkdownTokenType::CloseBracket {
                        break;
                    }
                    self.advance();
                }
                let link_text_tokens_end = self.current_token_index;
                let link_text = self.tokens[link_text_tokens_start..link_text_tokens_end].iter().map(|t| self.source.get_str(t.position.offset..(t.position.offset + t.position.length))).collect::<Result<Vec<&str>, PanduckError>>()?.join("");
                self.eat_token(MarkdownTokenType::CloseBracket);
                if self.eat_token(MarkdownTokenType::OpenParen) {
                    let url_tokens_start = self.current_token_index;
                    while let Some(t) = self.current_token() {
                        if t.token_type == MarkdownTokenType::CloseParen {
                            break;
                        }
                        self.advance();
                    }
                    let url_tokens_end = self.current_token_index;
                    let url = self.tokens[url_tokens_start..url_tokens_end].iter().map(|t| self.source.get_str(t.position.offset..(t.position.offset + t.position.length))).collect::<Result<Vec<&str>, PanduckError>>()?.join("");
                    self.eat_token(MarkdownTokenType::CloseParen);
                    Some(MarkdownInline::Link(link_text, url))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn parse_inline_content_until_closing_delimiter(&mut self, delimiter_type: MarkdownTokenType, count: usize) -> Vec<MarkdownInline> {
        let mut content = Vec::new();
        let mut delimiter_found_count = 0;
        while let Some(token) = self.current_token() {
            if token.token_type == delimiter_type {
                delimiter_found_count += 1;
                if delimiter_found_count == count {
                    break;
                }
            } else {
                delimiter_found_count = 0;
            }

            if let Some(inline) = self.parse_inline() {
                content.push(inline);
            } else {
                self.advance();
            }
        }
        content
    }
}

impl MarkdownReader {
    pub fn new(config: MarkdownReadConfig) -> Self {
        Self { config }
    }

    pub fn parse(
        &self,
        source: &SourceText,
        tokens: Vec<MarkdownToken>,
    ) -> PanduckDiagnostics<MarkdownRoot> {
        let mut state = ParserState::new(source, tokens);
        let ast = state.parse_document();
        PanduckDiagnostics::success(ast)
    }
}
