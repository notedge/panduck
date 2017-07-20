use panduck_markdown::parser::MarkdownReader;
use panduck_markdown::lexer::MarkdownReader as LexerReader;
use panduck_markdown::MarkdownReadConfig;
use panduck_core::helpers::SourceText;
use panduck_markdown::ast::{MarkdownBlock, MarkdownHeading, MarkdownInline, MarkdownParagraph, MarkdownBlockCode, MarkdownList, MarkdownListItem};

fn parse_markdown_to_ast(input: &str) -> Vec<MarkdownBlock> {
    let source = SourceText::new(input);
    let lexer = LexerReader::new(MarkdownReadConfig { support_math: false });
    let tokens = lexer.tokenize(&source).into_value();
    let parser = MarkdownReader::new(MarkdownReadConfig { support_math: false });
    parser.parse(&source, tokens).into_value().blocks
}

#[test]
fn test_parse_heading() {
    let ast = parse_markdown_to_ast("# Heading 1\n## Heading 2\n");
    assert_eq!(ast.len(), 2);
    if let MarkdownBlock::Heading(h1) = &ast[0] {
        assert_eq!(h1.level, 1);
        assert_eq!(h1.content.len(), 1);
        if let MarkdownInline::Text(text) = &h1.content[0] {
            assert_eq!(text, "Heading 1");
        }
    }
    if let MarkdownBlock::Heading(h2) = &ast[1] {
        assert_eq!(h2.level, 2);
        assert_eq!(h2.content.len(), 1);
        if let MarkdownInline::Text(text) = &h2.content[0] {
            assert_eq!(text, "Heading 2");
        }
    }
}

#[test]
fn test_parse_paragraph() {
    let ast = parse_markdown_to_ast("This is a paragraph.\nAnother paragraph.\n");
    assert_eq!(ast.len(), 2);
    if let MarkdownBlock::Paragraph(p1) = &ast[0] {
        assert_eq!(p1.content.len(), 4);
        if let MarkdownInline::Text(text) = &p1.content[0] {
            assert_eq!(text, "This");
        }
    }
}

#[test]
fn test_parse_bold_italic() {
    let ast = parse_markdown_to_ast("**bold** _italic_\n");
    assert_eq!(ast.len(), 1);
    if let MarkdownBlock::Paragraph(p) = &ast[0] {
        assert_eq!(p.content.len(), 3);
        if let MarkdownInline::Bold(bold_content) = &p.content[0] {
            assert_eq!(bold_content.len(), 1);
            if let MarkdownInline::Text(text) = &bold_content[0] {
                assert_eq!(text, "bold");
            }
        }
        if let MarkdownInline::Italic(italic_content) = &p.content[2] {
            assert_eq!(italic_content.len(), 1);
            if let MarkdownInline::Text(text) = &italic_content[0] {
                assert_eq!(text, "italic");
            }
        }
    }
}

#[test]
fn test_parse_code_block() {
    let ast = parse_markdown_to_ast("```rust\nfn main() {}\n```\n");
    assert_eq!(ast.len(), 1);
    if let MarkdownBlock::BlockCode(code) = &ast[0] {
        assert_eq!(code.lang, Some("rust".to_string()));
        assert_eq!(code.content, "fn main() {}\n");
    }
}

#[test]
fn test_parse_list() {
    let ast = parse_markdown_to_ast("- Item 1\n- Item 2\n");
    assert_eq!(ast.len(), 1);
    if let MarkdownBlock::List(list) = &ast[0] {
        assert_eq!(list.items.len(), 2);
        if let MarkdownListItem { content: item1_content } = &list.items[0] {
            assert_eq!(item1_content.len(), 2);
            if let MarkdownInline::Text(text) = &item1_content[0] {
                assert_eq!(text, "Item");
            }
        }
    }
}