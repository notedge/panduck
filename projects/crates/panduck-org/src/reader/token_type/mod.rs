//! # panduck-org
//! 
//! This is a placeholder readme for the panduck-org project.
#![doc = include_str!("../../../readme.md")]

use panduck_types::lexer::TokenType;
use panduck_types::reader::Token;

pub type OrgToken = Token<OrgTokenType>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OrgTokenType {
    // Org specific tokens
    Heading(usize), // Level from 1 to n
    DrawerStart(String), // Drawer name (e.g. :PROPERTIES:)
    DrawerEnd, // :END:
    Planning(String), // DEADLINE/SCHEDULED
    Keyword(String), // #+KEYWORD
    Timestamp(String), // <2024-05-20>
    // Common markup tokens
    Star,
    Underscore,
    Backtick,
    TripleBacktick,
    OpenBracket,
    CloseBracket,
    OpenParen,
    CloseParen,
    ExclamationMark,
    Hash,
    Minus,
    Plus,
    Dot,
    Number,
    GreaterThan,
    LessThan,
    Text(String),
    Newline,
    Whitespace,
    EndOfFile,
}

impl TokenType for OrgTokenType {
    const END_OF_STREAM: Self = OrgTokenType::EndOfFile;

    fn is_whitespace(&self) -> bool {
        matches!(self, OrgTokenType::Whitespace)
    }

    fn is_ignored(&self) -> bool {
        false
    }
}

impl ToString for OrgTokenType {
    fn to_string(&self) -> String {
        match self {
            OrgTokenType::Heading(level) => format!("Heading({})", level),
            OrgTokenType::DrawerStart(name) => format!("DrawerStart({})", name),
            OrgTokenType::DrawerEnd => ":END:".to_string(),
            OrgTokenType::Planning(keyword) => format!("Planning({})", keyword),
            OrgTokenType::Keyword(keyword) => format!("Keyword({})", keyword),
            OrgTokenType::Timestamp(ts) => format!("Timestamp({})", ts),
            OrgTokenType::Star => "*".to_string(),
            OrgTokenType::Underscore => "_".to_string(),
            OrgTokenType::Backtick => "`".to_string(),
            OrgTokenType::TripleBacktick => "```".to_string(),
            OrgTokenType::OpenBracket => "[".to_string(),
            OrgTokenType::CloseBracket => "]".to_string(),
            OrgTokenType::OpenParen => "(".to_string(),
            OrgTokenType::CloseParen => ")".to_string(),
            OrgTokenType::ExclamationMark => "!".to_string(),
            OrgTokenType::Hash => "#".to_string(),
            OrgTokenType::Minus => "-".to_string(),
            OrgTokenType::Plus => "+".to_string(),
            OrgTokenType::Dot => ".".to_string(),
            OrgTokenType::Number => "Number".to_string(),
            OrgTokenType::GreaterThan => ">".to_string(),
            OrgTokenType::LessThan => "<".to_string(),
            OrgTokenType::Text(text) => format!("Text(\"{}\")", text),
            OrgTokenType::Newline => "\\n".to_string(),
            OrgTokenType::Whitespace => "Whitespace".to_string(),
            OrgTokenType::EndOfFile => "EOF".to_string(),
        }
    }
}