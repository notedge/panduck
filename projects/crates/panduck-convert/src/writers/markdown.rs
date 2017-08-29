use std::fmt::Write as _;

use notedown_ir::{Block, DocumentGraph, Inline, TableRow};
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
        Block::Table { rows } => {
            write_table(out, rows)?;
            out.push('\n');
        }
        Block::Math { .. } | Block::Opaque { .. } => {
            return Err(AdapterError::unsupported_format(
                "markdown",
                "block type is not supported by the IR markdown writer yet",
            ));
        }
    }
    Ok(())
}

fn write_table(out: &mut String, rows: &[TableRow]) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    for (index, row) in rows.iter().enumerate() {
        out.push('|');
        for cell in &row.cells {
            out.push(' ');
            write_table_cell(out, cell)?;
            out.push_str(" |");
        }
        out.push('\n');
        if index == 0 {
            out.push('|');
            for _ in &row.cells {
                out.push_str(" --- |");
            }
            out.push('\n');
        }
    }
    Ok(())
}

fn write_table_cell(out: &mut String, inlines: &[Inline]) -> Result<()> {
    let text = inlines
        .iter()
        .map(inline_plain_text)
        .collect::<String>()
        .replace('|', "\\|")
        .replace('\n', " ");
    out.push_str(&text);
    Ok(())
}

fn write_markdown_image(out: &mut String, children: &[Inline]) -> Result<()> {
    if children.len() >= 2 {
        let alt = inline_plain_text(&children[0]);
        let url = inline_plain_text(&children[1]);
        out.push_str("![");
        out.push_str(&alt);
        out.push_str("](");
        out.push_str(&url);
        out.push(')');
        return Ok(());
    }
    write_inlines(out, children)
}

fn write_markdown_link(out: &mut String, children: &[Inline]) -> Result<()> {
    if children.len() >= 2 {
        let display = inline_plain_text(&children[0]);
        let url = inline_plain_text(&children[1]);
        out.push('[');
        out.push_str(&display);
        out.push_str("](");
        out.push_str(&url);
        out.push(')');
        return Ok(());
    }
    write_inlines(out, children)
}

fn inline_plain_text(inline: &Inline) -> String {
    match inline {
        Inline::Text { text } => text.clone(),
        Inline::InlineCode { text } => text.clone(),
        Inline::Styled { children, .. } => children.iter().map(inline_plain_text).collect(),
        Inline::InlineMath { content, .. } => content.clone(),
        Inline::Reference { display, .. } => display.clone(),
    }
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
            if style == "link" {
                return write_markdown_link(out, children);
            }
            if style == "image" {
                return write_markdown_image(out, children);
            }
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
