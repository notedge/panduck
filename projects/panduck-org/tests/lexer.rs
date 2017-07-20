use crate::reader::lexer::{Lexer, Token};

#[test]
fn test_empty_input() {
    let mut lexer = Lexer::new("");
    assert_eq!(lexer.next_token(), Token::Eof);
}

#[test]
fn test_heading_token() {
    let mut lexer = Lexer::new("* Heading");
    assert_eq!(lexer.next_token(), Token::Heading(1));
    assert_eq!(lexer.next_token(), Token::Text("Heading".to_string()));
    assert_eq!(lexer.next_token(), Token::Eof);
}

#[test]
fn test_text_token() {
    let mut lexer = Lexer::new("Some text");
    assert_eq!(lexer.next_token(), Token::Text("Some text".to_string()));
    assert_eq!(lexer.next_token(), Token::Eof);
}