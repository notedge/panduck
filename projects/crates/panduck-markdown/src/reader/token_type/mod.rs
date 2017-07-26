use panduck_types::lexer::TokenType;
use panduck_types::reader::Token;

pub type MarkdownToken = Token<MarkdownTokenType>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarkdownTokenType {
    /// `*` or `**` for bold/italic
    Star,
    /// `_` or `__` for bold/italic
    Underscore,
    /// `` ` `` for inline code
    Backtick,
    /// ``` for code block
    TripleBacktick,
    /// `[` for links/images
    OpenBracket,
    /// `]` for links/images
    CloseBracket,
    /// `(` for link/image URLs
    OpenParen,
    /// `)` for link/image URLs
    CloseParen,
    /// `!` for images
    ExclamationMark,
    /// `#` for headings
    Hash,
    /// `-` for unordered list items
    Minus,
    /// `+` for unordered list items
    Plus,
    /// `.` for ordered list items
    Dot,
    /// `1`, `2`, `3`... for ordered list items
    Number,
    /// `>` for blockquotes
    GreaterThan,
    /// Plain text content
    Text,
    /// Newline character
    Newline,
    /// Whitespace character
    Whitespace,
    /// End of file
    EndOfFile,
}

impl TokenType for MarkdownTokenType {
    const END_OF_STREAM: Self = MarkdownTokenType::EndOfFile;

    fn is_whitespace(&self) -> bool {
        matches!(self, MarkdownTokenType::Whitespace)
    }

    fn is_ignored(&self) -> bool {
        false
    }
}
