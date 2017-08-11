use std::fmt::Write as _;

use notedown_ir::{Block, DocumentGraph, Inline};
use panduck_types::{AdapterError, Result};

/// Serializes a `DocumentGraph` to GitHub-flavored Markdown text.
pub fn write_document_markdown(graph: &DocumentGraph) -> Result<String> {
    let mut output = String::new();
    for node in &graph.blocks {
        write_block(&mut output, &node.block)?;
    }
    Ok(output)
}

fn write_block(out: &mut String, block: &Block) -> Result<()> {
    match block {
        Block::Section { level, title, children: _ } => {
            let level = (*level).clamp(1, 6);
            for _ in 0..level {
                out.push('#');
            }
            out.push(' ');
            write_inlines(out, title)?;
            out.push_str("\n\n");
        }
        Block::Paragraph { content } => {
            write_inlines(out, content)?;
            out.push_str("\n\n");
        }
        Block::Code { language, content } => {
            out.push_str("```");
            if let Some(language) = language {
                out.push_str(language);
            }
            out.push('\n');
            out.push_str(content);
            out.push_str("\n```\n\n");
        }
        Block::Quote { content } => {
            let mut line = String::new();
            write_inlines(&mut line, content)?;
            for part in line.lines() {
                out.push_str("> ");
                out.push_str(part);
                out.push('\n');
            }
            out.push('\n');
        }
        Block::List { ordered, items } => {
            for (index, item) in items.iter().enumerate() {
                if *ordered {
                    write!(out, "{}. ", index + 1)
                        .map_err(|error| AdapterError::adapter("markdown", error.to_string()))?;
                } else {
                    out.push_str("- ");
                }
                write_inlines(out, &item.content)?;
                out.push('\n');
            }
            out.push('\n');
        }
        Block::Table { .. } | Block::Math { .. } | Block::Opaque { .. } => {
            return Err(AdapterError::unsupported_format(
                "markdown",
                "block type is not supported by the IR markdown writer yet",
            ));
        }
    }
    Ok(())
}

fn write_inlines(out: &mut String, inlines: &[Inline]) -> Result<()> {
    for inline in inlines {
        write_inline(out, inline)?;
    }
    Ok(())
}

fn write_inline(out: &mut String, inline: &Inline) -> Result<()> {
    match inline {
        Inline::Text { text } => {
            out.push_str(text);
        }
        Inline::InlineCode { text } => {
            out.push('`');
            out.push_str(text);
            out.push('`');
        }
        Inline::Styled { style, children } => {
            let wrapper = match style.as_str() {
                "bold" | "strong" => ("**", "**"),
                "italic" | "emphasis" => ("*", "*"),
                _ => ("", ""),
            };
            out.push_str(wrapper.0);
            write_inlines(out, children)?;
            out.push_str(wrapper.1);
        }
        Inline::InlineMath { content, .. } => {
            out.push('$');
            out.push_str(content);
            out.push('$');
        }
        Inline::Reference { display, .. } => {
            out.push_str(display);
        }
    }
    Ok(())
}
