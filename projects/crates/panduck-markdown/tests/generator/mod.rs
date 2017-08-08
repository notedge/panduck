use panduck_types::AdapterError;
use panduck_markdown::ast::{
    MarkdownBlock, MarkdownBlockCode, MarkdownBold, MarkdownHeading, MarkdownImage, MarkdownInline,
    MarkdownItalic, MarkdownLink, MarkdownList, MarkdownListItem, MarkdownParagraph, MarkdownRoot,
    MarkdownText,
};
use panduck_markdown::writer::MarkdownWriteConfig;

#[test]
fn test_markdown_generation() -> Result<(), AdapterError> {
    let ast = MarkdownRoot {
        blocks: vec![
            MarkdownBlock::Heading(MarkdownHeading {
                level: 1,
                content: vec![
                    MarkdownInline::Text(MarkdownText { text: "Hello, ".to_string() }),
                    MarkdownInline::Bold(MarkdownBold {
                        contents: vec![MarkdownInline::Text(MarkdownText { text: "World!".to_string() })],
                    }),
                ],
            }),
            MarkdownBlock::Paragraph(MarkdownParagraph {
                content: vec![
                    MarkdownInline::Text(MarkdownText { text: "This is a ".to_string() }),
                    MarkdownInline::Italic(MarkdownItalic {
                        contents: vec![MarkdownInline::Text(MarkdownText { text: "paragraph".to_string() })],
                    }),
                    MarkdownInline::Text(MarkdownText { text: " with some ".to_string() }),
                    MarkdownInline::Code("code".to_string()),
                    MarkdownInline::Text(MarkdownText { text: " and a ".to_string() }),
                    MarkdownInline::Link(MarkdownLink {
                        alt: "link".to_string(),
                        url: "https://example.com".to_string(),
                    }),
                    MarkdownInline::Text(MarkdownText { text: " and an ".to_string() }),
                    MarkdownInline::Image(MarkdownImage {
                        alt: "alt text".to_string(),
                        url: "https://example.com/image.png".to_string(),
                    }),
                    MarkdownInline::Text(MarkdownText { text: ".".to_string() }),
                ],
            }),
            MarkdownBlock::BlockCode(MarkdownBlockCode {
                language: "rust".to_string(),
                content: "fn main() {\n    println!(\"Hello, world!\");\n}".to_string(),
            }),
            MarkdownBlock::List(MarkdownList::Unordered(vec![
                MarkdownListItem {
                    content: vec![MarkdownInline::Text(MarkdownText { text: "Item 1".to_string() })],
                },
                MarkdownListItem {
                    content: vec![MarkdownInline::Text(MarkdownText { text: "Item 2".to_string() })],
                },
            ])),
            MarkdownBlock::List(MarkdownList::Ordered(vec![
                MarkdownListItem {
                    content: vec![MarkdownInline::Text(MarkdownText { text: "Ordered Item 1".to_string() })],
                },
                MarkdownListItem {
                    content: vec![MarkdownInline::Text(MarkdownText { text: "Ordered Item 2".to_string() })],
                },
            ])),
        ],
    };

    let mut buffer = String::new();
    let config = MarkdownWriteConfig::default();
    let _ = config.writer(&mut buffer).generate(&ast)?;

    assert_eq!(buffer, include_str!("generated1.md"));
    Ok(())
}

#[test]
fn test_markdown_generator() -> Result<(), AdapterError> {
    let ast = MarkdownRoot {
        blocks: vec![
            MarkdownBlock::Heading(MarkdownHeading {
                level: 1,
                content: vec![MarkdownInline::Text(MarkdownText { text: "Heading 1".to_string() })],
            }),
            MarkdownBlock::Paragraph(MarkdownParagraph {
                content: vec![
                    MarkdownInline::Text(MarkdownText { text: "This is a ".to_string() }),
                    MarkdownInline::Bold(MarkdownBold {
                        contents: vec![MarkdownInline::Text(MarkdownText { text: "bold".to_string() })],
                    }),
                    MarkdownInline::Text(MarkdownText { text: " and ".to_string() }),
                    MarkdownInline::Italic(MarkdownItalic {
                        contents: vec![MarkdownInline::Text(MarkdownText { text: "italic".to_string() })],
                    }),
                    MarkdownInline::Text(MarkdownText { text: " text with ".to_string() }),
                    MarkdownInline::Code("inline code".to_string()),
                    MarkdownInline::Text(MarkdownText { text: ".".to_string() }),
                ],
            }),
            MarkdownBlock::BlockCode(MarkdownBlockCode {
                language: "rust".to_string(),
                content: "fn main() {\n    println!(\"Hello, world!\");\n}".to_string(),
            }),
            MarkdownBlock::List(MarkdownList::Unordered(vec![
                MarkdownListItem {
                    content: vec![MarkdownInline::Text(MarkdownText { text: "Item 1".to_string() })],
                },
                MarkdownListItem {
                    content: vec![MarkdownInline::Text(MarkdownText { text: "Item 2".to_string() })],
                },
            ])),
            MarkdownBlock::List(MarkdownList::Ordered(vec![
                MarkdownListItem {
                    content: vec![MarkdownInline::Text(MarkdownText { text: "Ordered Item 1".to_string() })],
                },
                MarkdownListItem {
                    content: vec![MarkdownInline::Text(MarkdownText { text: "Ordered Item 2".to_string() })],
                },
            ])),
            MarkdownBlock::Paragraph(MarkdownParagraph {
                content: vec![
                    MarkdownInline::Link(MarkdownLink {
                        alt: "Google".to_string(),
                        url: "https://www.google.com".to_string(),
                    }),
                    MarkdownInline::Text(MarkdownText { text: " and ".to_string() }),
                    MarkdownInline::Image(MarkdownImage {
                        alt: "Alt Text".to_string(),
                        url: "https://example.com/image.png".to_string(),
                    }),
                    MarkdownInline::Text(MarkdownText { text: ".".to_string() }),
                ],
            }),
        ],
    };

    let mut output = String::new();
    let config = MarkdownWriteConfig::default();
    let _ = config.writer(&mut output).generate(&ast)?;

    assert_eq!(output, include_str!("generated2.md"));
    Ok(())
}
