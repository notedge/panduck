use panduck_rst::lexer::{tokenize, RstToken, RstTokenType};
use panduck_core::helpers::SourceText;

#[test]
fn test_heading_lexer() {
    let source = SourceText::new("=======\nTitle\n=======");
    let (tokens, diagnostics) = tokenize(source);

    assert!(diagnostics.is_empty());
    assert_eq!(tokens.len(), 5);
    assert_eq!(tokens[0].token_type, RstTokenType::Equals);
    assert_eq!(tokens[1].token_type, RstTokenType::Text);
    assert_eq!(tokens[1].text(&source), "Title");
    assert_eq!(tokens[2].token_type, RstTokenType::Newline);
    assert_eq!(tokens[3].token_type, RstTokenType::Equals);
    assert_eq!(tokens[4].token_type, RstTokenType::EndOfFile);
}

#[test]
fn test_paragraph_lexer() {
    let source = SourceText::new("This is a paragraph.");
    let (tokens, diagnostics) = tokenize(source);

    assert!(diagnostics.is_empty());
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].token_type, RstTokenType::Text);
    assert_eq!(tokens[0].text(&source), "This is a paragraph.");
    assert_eq!(tokens[1].token_type, RstTokenType::EndOfFile);
}

#[test]
fn test_bold_lexer() {
    let source = SourceText::new("**bold text**");
    let (tokens, diagnostics) = tokenize(source);

    assert!(diagnostics.is_empty());
    assert_eq!(tokens.len(), 4);
    assert_eq!(tokens[0].token_type, RstTokenType::Star);
    assert_eq!(tokens[1].token_type, RstTokenType::Text);
    assert_eq!(tokens[1].text(&source), "bold text");
    assert_eq!(tokens[2].token_type, RstTokenType::Star);
    assert_eq!(tokens[3].token_type, RstTokenType::EndOfFile);
}

#[test]
fn test_italic_lexer() {
    let source = SourceText::new("*italic text*");
    let (tokens, diagnostics) = tokenize(source);

    assert!(diagnostics.is_empty());
    assert_eq!(tokens.len(), 4);
    assert_eq!(tokens[0].token_type, RstTokenType::Star);
    assert_eq!(tokens[1].token_type, RstTokenType::Text);
    assert_eq!(tokens[1].text(&source), "italic text");
    assert_eq!(tokens[2].token_type, RstTokenType::Star);
    assert_eq!(tokens[3].token_type, RstTokenType::EndOfFile);
}

#[test]
fn test_code_lexer() {
    let source = SourceText::new("``code``");
    let (tokens, diagnostics) = tokenize(source);

    assert!(diagnostics.is_empty());
    assert_eq!(tokens.len(), 4);
    assert_eq!(tokens[0].token_type, RstTokenType::Backtick);
    assert_eq!(tokens[1].token_type, RstTokenType::Text);
    assert_eq!(tokens[1].text(&source), "code");
    assert_eq!(tokens[2].token_type, RstTokenType::Backtick);
    assert_eq!(tokens[3].token_type, RstTokenType::EndOfFile);
}

#[test]
fn test_link_lexer() {
    let source = SourceText::new("`link text <http://example.com>`_`");
    let (tokens, diagnostics) = tokenize(source);

    assert!(diagnostics.is_empty());
    assert_eq!(tokens.len(), 8);
    assert_eq!(tokens[0].token_type, RstTokenType::Backtick);
    assert_eq!(tokens[1].token_type, RstTokenType::Text);
    assert_eq!(tokens[1].text(&source), "link text ");
    assert_eq!(tokens[2].token_type, RstTokenType::LessThan);
    assert_eq!(tokens[3].token_type, RstTokenType::Text);
    assert_eq!(tokens[3].text(&source), "http://example.com");
    assert_eq!(tokens[4].token_type, RstTokenType::GreaterThan);
    assert_eq!(tokens[5].token_type, RstTokenType::Backtick);
    assert_eq!(tokens[6].token_type, RstTokenType::Underscore);
    assert_eq!(tokens[7].token_type, RstTokenType::EndOfFile);
}

#[test]
fn test_unordered_list_lexer() {
    let source = SourceText::new("- Item 1\n- Item 2");
    let (tokens, diagnostics) = tokenize(source);

    assert!(diagnostics.is_empty());
    assert_eq!(tokens.len(), 7);
    assert_eq!(tokens[0].token_type, RstTokenType::Hyphen);
    assert_eq!(tokens[1].token_type, RstTokenType::Text);
    assert_eq!(tokens[1].text(&source), " Item 1");
    assert_eq!(tokens[2].token_type, RstTokenType::Newline);
    assert_eq!(tokens[3].token_type, RstTokenType::Hyphen);
    assert_eq!(tokens[4].token_type, RstTokenType::Text);
    assert_eq!(tokens[4].text(&source), " Item 2");
    assert_eq!(tokens[5].token_type, RstTokenType::Newline);
    assert_eq!(tokens[6].token_type, RstTokenType::EndOfFile);
}

#[test]
fn test_ordered_list_lexer() {
    let source = SourceText::new("1. Item 1\n2. Item 2");
    let (tokens, diagnostics) = tokenize(source);

    assert!(diagnostics.is_empty());
    assert_eq!(tokens.len(), 7);
    assert_eq!(tokens[0].token_type, RstTokenType::Number);
    assert_eq!(tokens[1].token_type, RstTokenType::Text);
    assert_eq!(tokens[1].text(&source), ". Item 1");
    assert_eq!(tokens[2].token_type, RstTokenType::Newline);
    assert_eq!(tokens[3].token_type, RstTokenType::Number);
    assert_eq!(tokens[4].token_type, RstTokenType::Text);
    assert_eq!(tokens[4].text(&source), ". Item 2");
    assert_eq!(tokens[5].token_type, RstTokenType::Newline);
    assert_eq!(tokens[6].token_type, RstTokenType::EndOfFile);
}