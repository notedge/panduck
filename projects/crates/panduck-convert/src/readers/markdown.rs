use std::path::Path;

use notedown_ir::{
    Block, DocumentGraph, DocumentId, Inline, ListItem, LossMarker, SemanticStatus,
};
use oak_core::parser::session::ParseSession;
use oak_core::{GreenNode, Parser, RedNode, RedTree, SourceText};
use oak_markdown::parser::element_type::MarkdownElementType;
use oak_markdown::{MarkdownLanguage, MarkdownParser};
use panduck_types::{AdapterError, Result};

/// Reads Markdown text from disk into `DocumentGraph`.
pub fn read_markdown(path: impl AsRef<Path>) -> Result<DocumentGraph> {
    let path = path.as_ref();
    let text = std::fs::read_to_string(path).map_err(|source| {
        AdapterError::io(source, Some(path.display().to_string()))
    })?;
    read_markdown_bytes(path.display().to_string(), text)
}

/// Reads Markdown bytes as UTF-8 into `DocumentGraph`.
pub fn read_markdown_bytes(label: impl Into<String>, text: String) -> Result<DocumentGraph> {
    let label = label.into();
    let language = MarkdownLanguage::default();
    let source = SourceText::new(text);
    let mut cache = ParseSession::<MarkdownLanguage>::default();
    let parser = MarkdownParser::new(&language);
    let parsed = parser.parse(&source, &[], &mut cache);
    let green_tree = parsed
        .result
        .map_err(|error| AdapterError::adapter("markdown", error.to_string()))?;
    lower_markdown_tree(&label, green_tree, &source)
}

fn lower_markdown_tree(
    label: &str,
    green_tree: &GreenNode<MarkdownLanguage>,
    source: &SourceText,
) -> Result<DocumentGraph> {
    let red_root = RedNode::new(green_tree, 0);
    let mut graph = DocumentGraph::new(document_id_for(label));
    for child in red_root.children() {
        if let RedTree::Node(node) = child {
            match lower_block_node(node, source) {
                BlockOutcome::Block(block) => {
                    graph.push_block(block);
                }
                BlockOutcome::Loss(marker) => {
                    graph.push_loss(marker);
                }
                BlockOutcome::Skip => {}
            }
        }
    }

    if graph.blocks.is_empty() && graph.coverage.loss.is_empty() {
        graph.push_loss(LossMarker {
            code: "reader.markdown.empty_document".into(),
            message: "markdown source produced no semantic blocks".into(),
            status: SemanticStatus::Partial,
        });
    }
    Ok(graph)
}

enum BlockOutcome {
    Block(Block),
    Loss(LossMarker),
    Skip,
}

fn lower_block_node(node: RedNode<MarkdownLanguage>, source: &SourceText) -> BlockOutcome {
    let kind = node.element_type();
    match kind {
        MarkdownElementType::Heading1
        | MarkdownElementType::Heading2
        | MarkdownElementType::Heading3
        | MarkdownElementType::Heading4
        | MarkdownElementType::Heading5
        | MarkdownElementType::Heading6 => BlockOutcome::Block(Block::Section {
            level: heading_level(kind),
            title: collect_inlines(node, source),
            children: Vec::new(),
        }),
        MarkdownElementType::Paragraph => BlockOutcome::Block(Block::Paragraph {
            content: collect_inlines(node, source),
        }),
        MarkdownElementType::CodeBlock => {
            let (language, content) = extract_code_block(node, source);
            BlockOutcome::Block(Block::Code { language, content })
        }
        MarkdownElementType::List => {
            let (ordered, items) = extract_list(node, source);
            BlockOutcome::Block(Block::List { ordered, items })
        }
        MarkdownElementType::Blockquote => BlockOutcome::Block(Block::Quote {
            content: vec![Inline::Text {
                text: extract_blockquote_text(node, source),
            }],
        }),
        MarkdownElementType::HorizontalRule => BlockOutcome::Loss(LossMarker {
            code: "reader.markdown.horizontal_rule".into(),
            message: "horizontal rule lowered to loss marker".into(),
            status: SemanticStatus::Lossy,
        }),
        _ => BlockOutcome::Loss(LossMarker {
            code: "reader.markdown.unsupported_block".into(),
            message: format!("unsupported markdown block: {kind:?}"),
            status: SemanticStatus::Unsupported,
        }),
    }
}

fn collect_inlines(node: RedNode<MarkdownLanguage>, source: &SourceText) -> Vec<Inline> {
    let mut inlines = Vec::new();
    for child in node.children() {
        match child {
            RedTree::Node(child_node) => {
                if let Some(inline) = lower_inline_node(child_node, source) {
                    push_inline(&mut inlines, inline);
                }
            }
            RedTree::Leaf(_) => {
                let text = child.text(source);
                if !text.is_empty() && !is_markdown_marker(text.as_ref()) {
                    push_inline(&mut inlines, Inline::Text { text: text.into_owned() });
                }
            }
        }
    }
    if inlines.is_empty() {
        let text = node.text(source);
        if !text.trim().is_empty() {
            inlines.push(Inline::Text {
                text: text.into_owned(),
            });
        }
    }
    inlines
}

fn lower_inline_node(node: RedNode<MarkdownLanguage>, source: &SourceText) -> Option<Inline> {
    match node.element_type() {
        MarkdownElementType::Text | MarkdownElementType::HeadingText => Some(Inline::Text {
            text: node.text(source).into_owned(),
        }),
        MarkdownElementType::Strong => Some(Inline::Styled {
            style: "bold".into(),
            children: collect_inlines(node, source),
        }),
        MarkdownElementType::Emphasis => Some(Inline::Styled {
            style: "italic".into(),
            children: collect_inlines(node, source),
        }),
        MarkdownElementType::InlineCode => Some(Inline::InlineCode {
            text: collect_plain_text(node, source),
        }),
        MarkdownElementType::Link => parse_inline_link(node, source),
        MarkdownElementType::Strikethrough => Some(Inline::Text {
            text: collect_plain_text(node, source),
        }),
        _ => None,
    }
}

fn push_inline(inlines: &mut Vec<Inline>, inline: Inline) {
    if let Inline::Text { text: right } = inline {
        if let Some(Inline::Text { text: left }) = inlines.last_mut() {
            left.push_str(&right);
            return;
        }
        inlines.push(Inline::Text { text: right });
        return;
    }
    inlines.push(inline);
}

fn collect_plain_text(node: RedNode<MarkdownLanguage>, source: &SourceText) -> String {
    let mut text = String::new();
    for child in node.children() {
        match child {
            RedTree::Node(child_node) => text.push_str(&collect_plain_text(child_node, source)),
            RedTree::Leaf(_) => text.push_str(&child.text(source)),
        }
    }
    if text.is_empty() {
        text = node.text(source).into_owned();
    }
    text
}

fn parse_inline_link(node: RedNode<MarkdownLanguage>, source: &SourceText) -> Option<Inline> {
    let raw = node.text(source);
    if let Some((text, url)) = split_markdown_link(raw.as_ref()) {
        return Some(Inline::Styled {
            style: "link".into(),
            children: vec![Inline::Text { text }, Inline::Text { text: url }],
        });
    }
    let text = collect_plain_text(node, source);
    if text.is_empty() {
        None
    } else {
        Some(Inline::Text { text })
    }
}

fn split_markdown_link(raw: &str) -> Option<(String, String)> {
    let start = raw.find("](")?;
    let text = raw[..start].trim_start_matches('[').trim().to_string();
    let url = raw[start + 2..]
        .trim_end_matches(')')
        .trim()
        .to_string();
    if text.is_empty() || url.is_empty() {
        return None;
    }
    Some((text, url))
}

fn is_markdown_marker(text: &str) -> bool {
    matches!(text, "*" | "**" | "_" | "__" | "`" | "[" | "]" | "(" | ")" | "![")
}

fn extract_code_block(
    node: RedNode<MarkdownLanguage>,
    source: &SourceText,
) -> (Option<String>, String) {
    let mut language = None;
    let mut content = String::new();
    let mut in_content = false;
    for child in node.children() {
        if let RedTree::Node(child_node) = child {
            match child_node.element_type() {
                MarkdownElementType::CodeLanguage => {
                    language = Some(collect_plain_text(child_node, source).trim().to_string());
                }
                MarkdownElementType::CodeFence => in_content = !in_content,
                MarkdownElementType::Text | MarkdownElementType::Whitespace | MarkdownElementType::Newline
                    if in_content =>
                {
                    content.push_str(&child_node.text(source));
                }
                _ => {}
            }
        }
    }
    (
        language.filter(|value| !value.is_empty()),
        content.trim().to_string(),
    )
}

fn extract_list(
    node: RedNode<MarkdownLanguage>,
    source: &SourceText,
) -> (bool, Vec<ListItem>) {
    let mut ordered = false;
    let mut items = Vec::new();
    for child in node.children() {
        if let RedTree::Node(child_node) = child {
            if child_node.element_type() == MarkdownElementType::ListItem {
                if items.is_empty() {
                    let marker = child_node.text(source);
                    ordered = marker
                        .trim_start()
                        .chars()
                        .next()
                        .map(|ch| ch.is_ascii_digit())
                        .unwrap_or(false);
                }
                items.push(ListItem {
                    content: collect_inlines(child_node, source),
                    children: Vec::new(),
                });
            }
        }
    }
    (ordered, items)
}

fn extract_blockquote_text(node: RedNode<MarkdownLanguage>, source: &SourceText) -> String {
    let mut parts = Vec::new();
    for child in node.children() {
        if let RedTree::Node(child_node) = child {
            if child_node.element_type() == MarkdownElementType::Paragraph {
                parts.push(collect_plain_text(child_node, source));
            }
        }
    }
    parts.join("\n")
}

fn heading_level(kind: MarkdownElementType) -> u8 {
    match kind {
        MarkdownElementType::Heading1 => 1,
        MarkdownElementType::Heading2 => 2,
        MarkdownElementType::Heading3 => 3,
        MarkdownElementType::Heading4 => 4,
        MarkdownElementType::Heading5 => 5,
        MarkdownElementType::Heading6 => 6,
        _ => 1,
    }
}

fn document_id_for(label: &str) -> DocumentId {
    let mut hash = 1u64;
    for byte in label.bytes() {
        hash = hash * 31 + u64::from(byte);
    }
    DocumentId(hash)
}
