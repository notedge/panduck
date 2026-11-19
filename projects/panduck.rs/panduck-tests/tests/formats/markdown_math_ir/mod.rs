//! `$...$` / `$$...$$` must reach the IR as math, not as text.

use document_formats::import::markdown::import_markdown_bytes;
use document_ir::{Block, Inline, MathDisplay, MathExpression};

#[test]
fn inline_math_is_an_inline_math_node_with_a_script_ast() {
    let graph = import_markdown_bytes("inline.md", "Inline: $E=mc^2$ end.").expect("markdown imports");
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("expected a paragraph, got {:?}", graph.blocks[0].block)
    };
    assert!(matches!(content.first(), Some(Inline::Text { text }) if text == "Inline: "));
    let Some(Inline::InlineMath { formula }) = content.get(1)
    else {
        panic!("expected InlineMath at index 1, got {:?}", content.get(1))
    };
    assert_eq!(formula.source, "E=mc^2");
    assert_eq!(formula.display, MathDisplay::Inline);
    let MathExpression::Sequence(items) = &formula.expression
    else {
        panic!("expected a sequence, got {:?}", formula.expression)
    };
    assert!(
        items.iter().any(|item| matches!(item, MathExpression::Script { superscript: Some(_), .. })),
        "the superscript must survive lowering: {items:?}"
    );
}

#[test]
fn display_math_is_a_math_block() {
    let graph = import_markdown_bytes("block.md", "$$\nE=mc^2\n$$").expect("markdown imports");
    let Block::Math { formula } = &graph.blocks[0].block
    else {
        panic!("expected Block::Math, got {:?}", graph.blocks[0].block)
    };
    assert_eq!(formula.display, MathDisplay::Display);
    assert!(matches!(formula.expression, MathExpression::Sequence(_)));
}

#[test]
fn backslash_parenthesis_is_not_markdown_math() {
    let graph = import_markdown_bytes("paren.md", r"Paren: \(E=mc^2\) end.").expect("markdown imports");
    let Block::Paragraph { content } = &graph.blocks[0].block
    else {
        panic!("expected a paragraph, got {:?}", graph.blocks[0].block)
    };
    assert!(!content.iter().any(|inline| matches!(inline, Inline::InlineMath { .. })));
    assert!(graph.coverage.loss.is_empty());
}
