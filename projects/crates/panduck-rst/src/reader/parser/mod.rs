use panduck_types::helpers::SourceText;
use panduck_types::PanduckDiagnostics;

use crate::ast::{RstBlock, RstBlockCode, RstHeading, RstInline, RstList, RstListItem, RstParagraph, RstRoot};
use crate::reader::token_type::{RstToken, RstTokenType};
use panduck_types::parser::{Parser, ParserConfig};

pub struct RstParserConfig;

impl ParserConfig for RstParserConfig {
    type Block = RstBlock;
    type Inline = RstInline;
    type Root = RstRoot;
    type Token = RstToken;
    type TokenType = RstTokenType;
}

pub struct ParserState {
    tokens: Vec<RstToken>,
    current: usize,
    source: SourceText,
    diagnostics: PanduckDiagnostics<String>,
}

impl Parser for ParserState {
    type Config = RstParserConfig;

    fn new(tokens: Vec<RstToken>, source: SourceText) -> Self {
        Self {
            tokens,
            current: 0,
            source,
            diagnostics: PanduckDiagnostics::new(),
        }
    }

    fn current_token(&self) -> &RstToken {
        &self.tokens[self.current]
    }

    fn peek_token(&self) -> &RstToken {
        self.tokens
            .get(self.current + 1)
            .unwrap_or(&self.tokens[self.tokens.len() - 1])
    }

    fn advance(&mut self) {
        self.current += 1;
    }

    fn diagnostics(&self) -> &PanduckDiagnostics<String> {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut PanduckDiagnostics<String> {
        &mut self.diagnostics
    }

    fn parse_root(&mut self) -> RstRoot {
        let mut blocks = Vec::new();
        while !self.current_token().is_eof() {
            if let Some(block) = self.parse_block() {
                blocks.push(block);
            }
        }
        RstRoot { blocks }
    }

    fn parse_block(&mut self) -> Option<RstBlock> {
        match self.current_token().token_type {
            RstTokenType::Hash => Some(RstBlock::Heading(self.parse_heading())),
            RstTokenType::Text => Some(RstBlock::Paragraph(self.parse_paragraph())),
            // Add more block types as needed
            _ => {
                self.advance();
                None
            }
        }
    }

    fn parse_inline_content_until_newline(&mut self) -> Vec<RstInline> {
        let mut content = Vec::new();
        while !self.current_token().is_eof() && !self.current_token().is_newline() {
            if let Some(inline) = self.parse_inline() {
                content.push(inline);
            }
        }
        self.advance(); // Consume the newline
        content
    }

    fn parse_inline(&mut self) -> Option<RstInline> {
        match self.current_token().token_type {
            RstTokenType::Text => {
                let text = self.current_token().text(&self.source).to_string();
                self.advance();
                Some(RstInline::Text(text))
            }
            // Add more inline types as needed
            _ => {
                self.advance();
                None
            }
        }
    }
}

impl ParserState {
    fn parse_heading(&mut self) -> RstHeading {
        self.advance(); // Consume the #
        let content = self.parse_inline_content_until_newline();
        RstHeading { level: 1, content }
    }

    fn parse_paragraph(&mut self) -> RstParagraph {
        let content = self.parse_inline_content_until_newline();
        RstParagraph { content }
    }

    // Add more parsing methods for other RST elements
}

pub fn parse(tokens: Vec<RstToken>, source: SourceText) -> (RstRoot, PanduckDiagnostics<String>) {
    let mut parser = ParserState::new(tokens, source);
    let ast = parser.parse_root();
    (ast, parser.diagnostics)
}
