mod blocks;
mod command;
mod link;
mod list;
mod table;
mod text;

use crate::{
    builder::SycamoreBuilder,
    shared::{error_inline, push_nodes},
};
use notedown_ast::{
    nodes::{CodeNode, Delimiter, Header, ListView, Literal, MathNode, StyleKind, StyleNode, TableView, TextNode},
    ASTKind,
};
use sycamore::generic_node::GenericNode;

pub trait IntoSycamore<G: GenericNode> {
    fn into_sycamore(self, context: &SycamoreBuilder) -> G;
}

impl<T, G> IntoSycamore<G> for Literal<T>
where
    T: IntoSycamore<G>,
    G: GenericNode,
{
    fn into_sycamore(self, ctx: &SycamoreBuilder) -> G {
        self.value.into_sycamore(ctx)
    }
}

impl<G> IntoSycamore<G> for ASTKind
where
    G: GenericNode,
{
    fn into_sycamore(self, ctx: &SycamoreBuilder) -> G {
        match self {
            Self::Statements(children) => {
                let root: G = GenericNode::element("div");
                root.set_class_name("notedown");
                push_nodes(&root, children, ctx);
                return root;
            }
            Self::Paragraph(children) => {
                let p = GenericNode::element("p");
                push_nodes(&p, children, ctx);
                return p;
            }
            Self::Header(inner) => inner.into_sycamore(ctx),
            Self::Delimiter(inner) => inner.into_sycamore(ctx),
            Self::TableView(inner) => inner.into_sycamore(ctx),
            Self::ListView(inner) => inner.into_sycamore(ctx),
            Self::CodeNode(inner) => inner.into_sycamore(ctx),
            Self::MathNode(inner) => inner.into_sycamore(ctx),
            Self::LinkNode(inner) => inner.into_sycamore(ctx),
            Self::TextSpan(inner) => inner.into_sycamore(ctx),
            Self::StyledSpan(inner) => inner.into_sycamore(ctx),
            Self::Command(inner) => inner.into_sycamore(ctx),
            Self::Value(inner) => inner.into_sycamore(ctx),
        }
    }
}
