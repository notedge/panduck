use panduck_core::lexer::TokenType;
use panduck_core::reader::Token;

pub type RstToken = Token<RstTokenType>;

#[derive(Debug, PartialEq, Clone)]
pub enum RstTokenType {
    // Structural tokens
    Section,
    Title,
    Subtitle,
    Paragraph,
    LiteralBlock,
    BlockQuote,
    BulletList,
    EnumeratedList,
    DefinitionList,
    FieldList,
    OptionList,
    LineBlock,
    GridTable,
    SimpleTable,
    Transition,

    // Inline tokens
    Emphasis,
    Strong,
    Literal,
    Reference,
    FootnoteReference,
    SubstitutionReference,
    InlineInternalTarget,
    Uri,
    Email,

    // Special characters/delimiters
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

    // Generic tokens
    Text,
    Newline,
    Indent,
    Dedent,
    EndOfFile,
}

impl TokenType for RstTokenType {
    fn eof() -> Self {
        RstTokenType::EndOfFile
    }

    fn is_eof(&self) -> bool {
        matches!(self, RstTokenType::EndOfFile)
    }

    fn is_whitespace(&self) -> bool {
        matches!(self, RstTokenType::Newline | RstTokenType::Indent | RstTokenType::Dedent)
    }
}
