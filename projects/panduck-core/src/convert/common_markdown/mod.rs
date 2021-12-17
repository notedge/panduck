mod code;
mod html;
mod list;
mod table;
pub use self::list::{block_quote, node_item, node_list};

use crate::{ExtensionHandler, ExtensionRegistrar, Result, ToNotedown};
use comrak::{
    arena_tree::Children,
    nodes::{Ast, AstNode, NodeValue},
    parse_document, Arena, ComrakExtensionOptions, ComrakOptions, ComrakParseOptions,
};
use notedown_ast::{ASTKind, ASTNode, ASTNodes};
use std::{cell::RefCell, collections::BTreeSet, iter::FromIterator};

pub fn register_common_markdown(r: &mut ExtensionRegistrar) {
    let ext = vec!["markdown", "md"];
    let new = ExtensionHandler {
        name: String::from("markdown"),
        try_extension: BTreeSet::from_iter(ext.into_iter().map(String::from)),
        parser: parse_common_markdown,
    };
    r.insert(new)
}

pub fn parse_common_markdown(input: &str) -> Result<ASTNode> {
    let arena = Arena::new();
    let options = ComrakOptions {
        extension: ComrakExtensionOptions {
            strikethrough: true,
            tagfilter: true,
            table: true,
            autolink: true,
            tasklist: true,
            superscript: true,
            header_ids: None,
            footnotes: true,
            description_lists: true,
            front_matter_delimiter: Some(String::from("---")),
        },
        parse: ComrakParseOptions { smart: true, default_info_string: None },
        render: Default::default(),
    };
    let parsed = parse_document(&arena, input, &options);
    return Ok(parsed.into_notedown());
}

impl<'a> ToNotedown for &'a AstNode<'a> {
    fn into_notedown(self) -> ASTNode {
        let node: NodeValue = self.data.borrow().value.to_owned();
        match node {
            NodeValue::Document => ASTKind::statements(self.children().into_notedown_list(), None),
            NodeValue::FrontMatter(v) => {
                let front = String::from_utf8_lossy(&v);
                ASTKind::code_block(front, "front-matter", None)
            }
            NodeValue::BlockQuote => block_quote(self),
            NodeValue::List(v) => node_list(v, self.children().into_notedown_list()),
            NodeValue::Item(v) => node_item(v, self.children().into_notedown_list()),
            NodeValue::DescriptionList => ASTNode::default(),
            NodeValue::DescriptionItem(v) => v.into_notedown(),
            NodeValue::DescriptionTerm => {
                todo!()
            }
            NodeValue::DescriptionDetails => {
                todo!()
            }
            NodeValue::CodeBlock(v) => v.into_notedown(),
            NodeValue::HtmlBlock(v) => v.into_notedown(),
            NodeValue::Paragraph => ASTKind::paragraph(self.children().into_notedown_list(), None),
            NodeValue::Heading(v) => ASTKind::header(self.children().into_notedown_list(), v.level as u8, None),
            NodeValue::ThematicBreak => ASTKind::hr(None),
            NodeValue::FootnoteDefinition(_) => {
                todo!()
            }
            NodeValue::Table(v) => v.into_notedown(),
            NodeValue::TableRow(_) => {
                todo!()
            }
            NodeValue::TableCell => {
                todo!()
            }
            NodeValue::Text(v) => ASTKind::text(String::from_utf8_lossy(&v), None),
            NodeValue::TaskItem(_) => {
                todo!()
            }
            NodeValue::SoftBreak => ASTKind::soft_break(None),
            NodeValue::LineBreak => ASTKind::hard_break(None),
            NodeValue::Code(v) => v.into_notedown(),
            NodeValue::HtmlInline(v) => ASTKind::raw_html_inline(String::from_utf8_lossy(&v), None),
            NodeValue::Emph => ASTKind::emphasis(self.children().into_notedown_list(), None),
            NodeValue::Strong => ASTKind::strong(self.children().into_notedown_list(), None),
            NodeValue::Strikethrough => ASTKind::delete(self.children().into_notedown_list(), None),
            NodeValue::Superscript => ASTKind::superscript(self.children().into_notedown_list(), None),
            NodeValue::Link(v) => {
                let url = String::from_utf8_lossy(&v.url);
                match v.title.is_empty() {
                    true => ASTKind::hyper_link(url, None),
                    false => {
                        let text = String::from_utf8_lossy(&v.title);
                        ASTKind::hyper_link_text(url, text, None)
                    }
                }
            }
            NodeValue::Image(v) => {
                let url = String::from_utf8_lossy(&v.url);
                match v.title.is_empty() {
                    true => ASTKind::image_link(url, None),
                    false => {
                        let alt = String::from_utf8_lossy(&v.title);
                        ASTKind::image_link_alt(url, alt, None)
                    }
                }
            }
            NodeValue::FootnoteReference(v) => {
                let _foot = String::from_utf8_lossy(&v);
                todo!()
            }
        }
    }
}

impl ToNotedown for Children<'_, RefCell<Ast>> {
    fn into_notedown(self) -> ASTNode {
        todo!()
    }

    fn into_notedown_list(self) -> ASTNodes {
        let mut children = vec![];
        for node in self {
            children.push(node.into_notedown())
        }
        return children;
    }
}
