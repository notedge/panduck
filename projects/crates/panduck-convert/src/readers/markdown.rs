use std::path::Path;

use notedown_ir::{
    Block, DocumentGraph, DocumentId, Inline, ListItem, LossMarker, SemanticStatus,
};
use oak_core::{Builder, SourceText};
use oak_markdown::{MarkdownBuilder, MarkdownLanguage};
use oak_markdown::ast::{Block as OakBlock, ListItem as OakListItem};
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
    let builder = MarkdownBuilder::new(&language);
    let source = SourceText::new(text);
    let mut cache = oak_core::parser::session::ParseSession::<MarkdownLanguage>::default();
    let output = builder.build(&source, &[], &mut cache);
    let ast = output
        .result
        .map_err(|error| AdapterError::adapter("markdown", error.to_string()))?;
    lower_markdown_ast(&label, ast)
}

fn lower_markdown_ast(label: &str, ast: oak_markdown::MarkdownRoot) -> Result<DocumentGraph> {
    let mut graph = DocumentGraph::new(document_id_for(label));
    for block in ast.blocks {
        lower_block(&mut graph, block);
    }
    if graph.blocks.is_empty() {
        graph.push_loss(LossMarker {
            code: "reader.markdown.empty_document".into(),
            message: "markdown source produced no semantic blocks".into(),
            status: SemanticStatus::Partial,
        });
    }
    Ok(graph)
}

fn lower_block(graph: &mut DocumentGraph, block: OakBlock) {
    match block {
        OakBlock::Heading(heading) => {
            graph.push_block(Block::Section {
                level: heading.level.clamp(1, 6) as u8,
                title: vec![Inline::Text {
                    text: heading.content,
                }],
                children: Vec::new(),
            });
        }
        OakBlock::Paragraph(paragraph) => {
            graph.push_block(Block::Paragraph {
                content: vec![Inline::Text {
                    text: paragraph.content,
                }],
            });
        }
        OakBlock::CodeBlock(code) => {
            graph.push_block(Block::Code {
                language: code.language,
                content: code.content,
            });
        }
        OakBlock::List(list) => {
            let items = list
                .items
                .into_iter()
                .map(|item| ListItem {
                    content: vec![Inline::Text {
                        text: flatten_list_item(&item),
                    }],
                    children: Vec::new(),
                })
                .collect();
            graph.push_block(Block::List {
                ordered: list.is_ordered,
                items,
            });
        }
        OakBlock::Blockquote(quote) => {
            graph.push_block(Block::Quote {
                content: vec![Inline::Text {
                    text: flatten_blocks(&quote.content),
                }],
            });
        }
        OakBlock::HorizontalRule(_) => {
            graph.push_loss(LossMarker {
                code: "reader.markdown.horizontal_rule".into(),
                message: "horizontal rule lowered to loss marker".into(),
                status: SemanticStatus::Lossy,
            });
        }
        other => {
            graph.push_loss(LossMarker {
                code: "reader.markdown.unsupported_block".into(),
                message: format!("unsupported markdown block: {other:?}"),
                status: SemanticStatus::Unsupported,
            });
        }
    }
}

fn flatten_list_item(item: &OakListItem) -> String {
    flatten_blocks(&item.content)
}

fn flatten_blocks(blocks: &[OakBlock]) -> String {
    let mut parts = Vec::new();
    for block in blocks {
        match block {
            OakBlock::Paragraph(paragraph) => parts.push(paragraph.content.clone()),
            OakBlock::Heading(heading) => parts.push(heading.content.clone()),
            OakBlock::CodeBlock(code) => parts.push(code.content.clone()),
            other => parts.push(format!("[{other:?}]")),
        }
    }
    parts.join("\n")
}

fn document_id_for(label: &str) -> DocumentId {
    let mut hash = 1u64;
    for byte in label.bytes() {
        hash = hash * 31 + u64::from(byte);
    }
    DocumentId(hash)
}
