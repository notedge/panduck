use dioxus::{
    core::{DioxusElement, Element, LazyNodes, NodeFactory, Scope},
    prelude::{VNode, *},
};
use notedown_ast::{
    nodes::{CodeNode, Delimiter, Header, ListView, Literal, MathNode, StyleKind, StyleNode, TableView, TextSpan},
    ASTKind,
};

use crate::{DioxusBuilder, SycamoreConfig, SycamoreContext};

mod header;

pub trait IntoDioxus {
    fn into_dioxus(self, cx: Scope<DioxusBuilder>) -> Element;
}

impl<T> IntoDioxus for Literal<T>
where
    T: IntoDioxus,
{
    fn into_dioxus(self, cx: Scope<DioxusBuilder>) -> Element {
        self.value.into_dioxus(cx)
    }
}

impl IntoDioxus for ASTKind {
    fn into_dioxus(self, cx: Scope<DioxusBuilder>) -> Element {
        match self {
            Self::Statements(_) => {
                unimplemented!()
            }
            Self::Paragraph(_) => {
                unimplemented!()
            }
            Self::Delimiter(_) => {
                unimplemented!()
            }
            Self::Header(h) => h.into_dioxus(cx),
            Self::TableView(_) => {
                unimplemented!()
            }
            Self::ListView(_) => {
                unimplemented!()
            }
            Self::QuoteNode(_) => {
                unimplemented!()
            }
            Self::CodeNode(_) => {
                unimplemented!()
            }
            Self::MathNode(_) => {
                unimplemented!()
            }
            Self::LinkNode(_) => {
                unimplemented!()
            }
            Self::TextSpan(_) => {
                unimplemented!()
            }
            Self::StyledSpan(_) => {
                unimplemented!()
            }
            Self::Command(_) => {
                unimplemented!()
            }
            Self::Value(_) => {
                unimplemented!()
            }
        }
    }
}
