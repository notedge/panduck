use panduck_types::helpers::SourceText;


use panduck_types::PanduckDiagnostics;

pub use crate::reader::token_type::{RstToken, RstTokenType};
use panduck_types::reader::{Reader, ReadConfig};

pub struct RstReadConfig;

impl ReadConfig for RstReadConfig {
    type TokenType = RstTokenType;
    type Token = RstToken;
}

pub struct RstReader {
    source: SourceText,
    diagnostics: PanduckDiagnostics<String>,
}

impl Reader for RstReader {
    type Config = RstReadConfig;

    fn new(source: SourceText) -> Self {
        Self {
            source,
            diagnostics: PanduckDiagnostics::new(),
        }
    }

    fn source(&self) -> &SourceText {
        &self.source
    }

    fn source_mut(&mut self) -> &mut SourceText {
        &mut self.source
    }

    fn diagnostics(&self) -> &PanduckDiagnostics<String> {
        &self.diagnostics
    }

    fn diagnostics_mut(&mut self) -> &mut PanduckDiagnostics<String> {
        &mut self.diagnostics
    }

    fn next_token(&mut self) -> RstToken {
        self.skip_whitespace();

        if self.is_eof() {
            return self.token(RstTokenType::EndOfFile);
        }

        let c = self.peek();
        match c {
            '*' => self.token(RstTokenType::Star),
            '_' => self.token(RstTokenType::Underscore),
            '`' => self.token(RstTokenType::Backtick),
            '|' => self.token(RstTokenType::Pipe),
            ':' => {
                self.bump();
                if self.peek() == ':' {
                    self.token(RstTokenType::DoubleColon)
                } else {
                    self.token(RstTokenType::Colon)
                }
            }
            '-' => self.token(RstTokenType::Hyphen),
            '+' => self.token(RstTokenType::Plus),
            '=' => self.token(RstTokenType::Equal),
            '~' => self.token(RstTokenType::Tilde),
            '#' => self.token(RstTokenType::Hash),
            '<' => self.token(RstTokenType::LessThan),
            '>' => self.token(RstTokenType::GreaterThan),
            '[' => self.token(RstTokenType::OpenBracket),
            ']' => self.token(RstTokenType::CloseBracket),
            '(' => self.token(RstTokenType::OpenParen),
            ')' => self.token(RstTokenType::CloseParen),
            _ => self.read_text(),
        }
    }

    fn read_text(&mut self) -> RstToken {
        let start = self.current_pos();
        while !self.is_eof() && !self.peek().is_whitespace() && !self.is_delimiter() {
            self.bump();
        }
        self.token_from(start, RstTokenType::Text)
    }
}

impl RstReader {
    fn is_delimiter(&self) -> bool {
        matches!(
            self.peek(),
            '*' | '_' | '`' | '|' | ':' | '-' | '+' | '=' | '~' | '#' | '<' | '>' | '[' | ']' | '(' | ')'
        )
    }

    fn skip_whitespace(&mut self) {
        while self.peek().is_whitespace() && !self.is_newline() {
            self.bump();
        }
    }

    fn is_newline(&self) -> bool {
        self.peek() == '\n'
    }
}

pub fn tokenize(source: &str) -> (Vec<RstToken>, PanduckDiagnostics<String>) {
    let mut reader = RstReader::new(SourceText::new(source));
    let mut tokens = Vec::new();
    loop {
        let token = reader.next_token();
        if token.is_eof() {
            break;
        }
        tokens.push(token);
    }
    (tokens, reader.diagnostics)
}
