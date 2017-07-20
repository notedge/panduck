#![doc = include_str!("readme.md")]


use crate::ast::{MarkdownBlock, MarkdownBlockCode, MarkdownHeading, MarkdownInline, MarkdownList, MarkdownListItem, MarkdownParagraph, MarkdownRoot};

/// 二进制写入器，用于从实现了 WriteBytesExt trait 的类型中写入数据
///
/// 这是一个泛型结构体，可以包装任何实现了 WriteBytesExt trait 的类型，
/// 提供二进制数据的写入功能。
#[derive(Debug)]
pub struct MarkdownWriter {

}

impl MarkdownWriter {
    pub fn new() -> Self {
        Self {}
    }

    pub fn generate(&self, ast: MarkdownRoot) -> String {
        let mut html = String::new();
        for block in ast.blocks {
            html.push_str(&self.generate_block(block));
        }
        html
    }

    fn generate_block(&self, block: MarkdownBlock) -> String {
        match block {
            MarkdownBlock::Heading(heading) => self.generate_heading(heading),
            MarkdownBlock::Paragraph(paragraph) => self.generate_paragraph(paragraph),
            MarkdownBlock::BlockCode(block_code) => self.generate_block_code(block_code),
            MarkdownBlock::List(list) => self.generate_list(list),
        }
    }

    fn generate_heading(&self, heading: MarkdownHeading) -> String {
        let content = self.generate_inline_content(heading.content);
        format!("<h{}>{}</h{}>\n", heading.level, content, heading.level)
    }

    fn generate_paragraph(&self, paragraph: MarkdownParagraph) -> String {
        let content = self.generate_inline_content(paragraph.content);
        format!("<p>{}</p>\n", content)
    }

    fn generate_block_code(&self, block_code: MarkdownBlockCode) -> String {
        let lang_attr = if let Some(lang) = block_code.lang {
            format!(" class=\"language-{}\"", lang)
        } else {
            String::new()
        };
        format!("<pre><code{}>{}</code></pre>\n", lang_attr, escape_html(&block_code.content))
    }

    fn generate_list(&self, list: MarkdownList) -> String {
        let mut items_html = String::new();
        for item in list.items {
            items_html.push_str(&self.generate_list_item(item));
        }
        format!("<ul>\n{}</ul>\n", items_html)
    }

    fn generate_list_item(&self, item: MarkdownListItem) -> String {
        let content = self.generate_inline_content(item.content);
        format!("<li>{}</li>\n", content)
    }

    fn generate_inline_content(&self, content: Vec<MarkdownInline>) -> String {
        let mut html = String::new();
        for inline in content {
            html.push_str(&self.generate_inline(inline));
        }
        html
    }

    fn generate_inline(&self, inline: MarkdownInline) -> String {
        match inline {
            MarkdownInline::Text(text) => escape_html(&text),
            MarkdownInline::Bold(content) => format!("<strong>{}</strong>", self.generate_inline_content(content)),
            MarkdownInline::Italic(content) => format!("<em>{}</em>", self.generate_inline_content(content)),
            MarkdownInline::Code(code) => format!("<code>{}</code>", escape_html(&code)),
            MarkdownInline::Link(text, url) => format!("<a href=\"{}\">{}</a>", escape_html(&url), escape_html(&text)),
            MarkdownInline::Image(alt_text, url) => format!("<img src=\"{}\" alt=\"{}\">", escape_html(&url), escape_html(&alt_text)),
        }
    }
}

fn escape_html(s: &str) -> String {
    s.replace("&", "&amp;")
     .replace("<", "&lt;")
     .replace(">", "&gt;")
     .replace("\"", "&quot;")
     .replace("'", "&#x27;")
}
