use panduck_types::lexer::TokenType;
use panduck_types::reader::Token;

pub type RstToken = Token<RstTokenType>;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RstTokenType {
    Star,
    Underscore,
    Backtick,
    Pipe,
    Colon,
    DoubleColon,
    Hyphen,
    Plus,
    Equal,
    Tilde,
    Hash,
    LessThan,
    GreaterThan,
    OpenBracket,
    CloseBracket,
    OpenParen,
    CloseParen,
    Text,
    Newline,
    EndOfFile,
}

impl TokenType for RstTokenType {
    const END_OF_STREAM: Self = RstTokenType::EndOfFile;

    fn is_whitespace(&self) -> bool {
        matches!(self, RstTokenType::Newline)
    }

    fn is_ignored(&self) -> bool {
        false
    }
}

pub fn token_text<'a>(token: &RstToken, source: &'a panduck_types::helpers::SourceText) -> &'a str {
    let range = token.get_range();
    source.get_str(range).unwrap_or("")
}

pub fn is_eof(token: &RstToken) -> bool {
    matches!(token.token_type, RstTokenType::EndOfFile)
}

pub fn is_newline(token: &RstToken) -> bool {
    matches!(token.token_type, RstTokenType::Newline)
}
