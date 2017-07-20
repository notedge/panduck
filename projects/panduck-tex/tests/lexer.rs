use panduck_markdown::lexer::{MarkdownReader, MarkdownTokenType};
use panduck_core::helpers::SourceText;

#[test]
fn test_lex_heading() {
    let source = SourceText::new("# Heading 1\n## Heading 2");
    let lexer = MarkdownReader::new(Default::default());
    let tokens = lexer.tokenize(&source).into_value();
    
    assert_eq!(tokens[0].token_type, MarkdownTokenType::Hash);
    assert_eq!(tokens[2].token_type, MarkdownTokenType::Text);
    assert_eq!(tokens[4].token_type, MarkdownTokenType::Newline);
    assert_eq!(tokens[5].token_type, MarkdownTokenType::Hash);
}

#[test]
fn test_lex_bold_italic() {
    let source = SourceText::new("**bold** _italic_");
    let lexer = MarkdownReader::new(Default::default());
    let tokens = lexer.tokenize(&source).into_value();
    
    assert_eq!(tokens[0].token_type, MarkdownTokenType::Star);
    assert_eq!(tokens[1].token_type, MarkdownTokenType::Star);
    assert_eq!(tokens[3].token_type, MarkdownTokenType::Star);
    assert_eq!(tokens[4].token_type, MarkdownTokenType::Star);
    assert_eq!(tokens[6].token_type, MarkdownTokenType::Underscore);
    assert_eq!(tokens[8].token_type, MarkdownTokenType::Underscore);
}

#[test]
fn test_lex_code_block() {
    let source = SourceText::new("```rust\nfn main() {}```");
    let lexer = MarkdownReader::new(Default::default());
    let tokens = lexer.tokenize(&source).into_value();
    
    assert_eq!(tokens[0].token_type, MarkdownTokenType::TripleBacktick);
    assert_eq!(tokens[1].token_type, MarkdownTokenType::Text);
    assert_eq!(tokens[3].token_type, MarkdownTokenType::Text);
    assert_eq!(tokens[4].token_type, MarkdownTokenType::TripleBacktick);
}

#[test]
fn test_lex_list() {
    let source = SourceText::new("- Item 1\n1. Item 2");
    let lexer = MarkdownReader::new(Default::default());
    let tokens = lexer.tokenize(&source).into_value();
    
    assert_eq!(tokens[0].token_type, MarkdownTokenType::Minus);
    assert_eq!(tokens[2].token_type, MarkdownTokenType::Text);
    assert_eq!(tokens[4].token_type, MarkdownTokenType::Number);
    assert_eq!(tokens[5].token_type, MarkdownTokenType::Dot);
}