use panduck_rst::lexer::tokenize;
use panduck_rst::parser::parse;
use panduck_core::helpers::SourceText;
use panduck_rst::ast::{RstRoot, RstBlock, RstHeading, RstInline};

#[test]
fn test_heading_parser() {
    let source = SourceText::new("=======\nTitle\n=======");
    let (tokens, _lexer_diagnostics) = tokenize(source.clone());
    let (ast, parser_diagnostics) = parse(tokens, source);

    assert!(parser_diagnostics.is_empty());
    assert_eq!(ast.blocks.len(), 1);

    if let Some(RstBlock::Heading(heading)) = ast.blocks.first() {
        assert_eq!(heading.level, 1);
        assert_eq!(heading.content.len(), 1);
        if let Some(RstInline::Text(text)) = heading.content.first() {
            assert_eq!(text, "Title");
        }
    } else {
        panic!("Expected a heading block");
    }
}

#[test]
fn test_paragraph_parser() {
    let source = SourceText::new("This is a paragraph.");
    let (tokens, _lexer_diagnostics) = tokenize(source.clone());
    let (ast, parser_diagnostics) = parse(tokens, source);

    assert!(parser_diagnostics.is_empty());
    assert_eq!(ast.blocks.len(), 1);

    if let Some(RstBlock::Paragraph(paragraph)) = ast.blocks.first() {
        assert_eq!(paragraph.content.len(), 1);
        if let Some(RstInline::Text(text)) = paragraph.content.first() {
            assert_eq!(text, "This is a paragraph.");
        }
    } else {
        panic!("Expected a paragraph block");
    }
}

#[test]
fn test_bold_parser() {
    let source = SourceText::new("**bold text**");
    let (tokens, _lexer_diagnostics) = tokenize(source.clone());
    let (ast, parser_diagnostics) = parse(tokens, source);

    assert!(parser_diagnostics.is_empty());
    assert_eq!(ast.blocks.len(), 1);

    if let Some(RstBlock::Paragraph(paragraph)) = ast.blocks.first() {
        assert_eq!(paragraph.content.len(), 1);
        if let Some(RstInline::Bold(content)) = paragraph.content.first() {
            assert_eq!(content.len(), 1);
            if let Some(RstInline::Text(text)) = content.first() {
                assert_eq!(text, "bold text");
            }
        }
    } else {
        panic!("Expected a paragraph block with bold inline");
    }
}

#[test]
fn test_italic_parser() {
    let source = SourceText::new("*italic text*");
    let (tokens, _lexer_diagnostics) = tokenize(source.clone());
    let (ast, parser_diagnostics) = parse(tokens, source);

    assert!(parser_diagnostics.is_empty());
    assert_eq!(ast.blocks.len(), 1);

    if let Some(RstBlock::Paragraph(paragraph)) = ast.blocks.first() {
        assert_eq!(paragraph.content.len(), 1);
        if let Some(RstInline::Italic(content)) = paragraph.content.first() {
            assert_eq!(content.len(), 1);
            if let Some(RstInline::Text(text)) = content.first() {
                assert_eq!(text, "italic text");
            }
        }
    } else {
        panic!("Expected a paragraph block with italic inline");
    }
}

#[test]
fn test_code_parser() {
    let source = SourceText::new("``code``");
    let (tokens, _lexer_diagnostics) = tokenize(source.clone());
    let (ast, parser_diagnostics) = parse(tokens, source);

    assert!(parser_diagnostics.is_empty());
    assert_eq!(ast.blocks.len(), 1);

    if let Some(RstBlock::Paragraph(paragraph)) = ast.blocks.first() {
        assert_eq!(paragraph.content.len(), 1);
        if let Some(RstInline::Code(code)) = paragraph.content.first() {
            assert_eq!(code, "code");
        }
    } else {
        panic!("Expected a paragraph block with code inline");
    }
}

#[test]
fn test_link_parser() {
    let source = SourceText::new("`link text <http://example.com>`_`");
    let (tokens, _lexer_diagnostics) = tokenize(source.clone());
    let (ast, parser_diagnostics) = parse(tokens, source);

    assert!(parser_diagnostics.is_empty());
    assert_eq!(ast.blocks.len(), 1);

    if let Some(RstBlock::Paragraph(paragraph)) = ast.blocks.first() {
        assert_eq!(paragraph.content.len(), 1);
        if let Some(RstInline::Link { text, url }) = paragraph.content.first() {
            assert_eq!(url, "http://example.com");
            assert_eq!(text.len(), 1);
            if let Some(RstInline::Text(link_text)) = text.first() {
                assert_eq!(link_text, "link text ");
            }
        }
    } else {
        panic!("Expected a paragraph block with link inline");
    }
}

#[test]
fn test_unordered_list_parser() {
    let source = SourceText::new("- Item 1\n- Item 2");
    let (tokens, _lexer_diagnostics) = tokenize(source.clone());
    let (ast, parser_diagnostics) = parse(tokens, source);

    assert!(parser_diagnostics.is_empty());
    assert_eq!(ast.blocks.len(), 1);

    if let Some(RstBlock::List(RstList::Unordered(items))) = ast.blocks.first() {
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].content.len(), 1);
        if let Some(RstInline::Text(text)) = items[0].content.first() {
            assert_eq!(text, "Item 1");
        }
        assert_eq!(items[1].content.len(), 1);
        if let Some(RstInline::Text(text)) = items[1].content.first() {
            assert_eq!(text, "Item 2");
        }
    } else {
        panic!("Expected an unordered list block");
    }
}

#[test]
fn test_ordered_list_parser() {
    let source = SourceText::new("1. Item 1\n2. Item 2");
    let (tokens, _lexer_diagnostics) = tokenize(source.clone());
    let (ast, parser_diagnostics) = parse(tokens, source);

    assert!(parser_diagnostics.is_empty());
    assert_eq!(ast.blocks.len(), 1);

    if let Some(RstBlock::List(RstList::Ordered(items))) = ast.blocks.first() {
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].content.len(), 1);
        if let Some(RstInline::Text(text)) = items[0].content.first() {
            assert_eq!(text, "Item 1");
        }
        assert_eq!(items[1].content.len(), 1);
        if let Some(RstInline::Text(text)) = items[1].content.first() {
            assert_eq!(text, "Item 2");
        }
    } else {
        panic!("Expected an ordered list block");
    }
}