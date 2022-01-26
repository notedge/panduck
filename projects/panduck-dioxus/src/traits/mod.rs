use dioxus::core::LazyNodes;
use notedown_ast::{
    nodes::{CodeNode, Delimiter, Header, ListView, Literal, MathNode, StyleKind, StyleNode, TableView, TextSpan},
    ASTKind,
};

use crate::{SycamoreConfig, SycamoreContext};

pub trait IntoDioxus {
    fn into_dioxus<'a, 'b>(self, cfg: &SycamoreConfig, ctx: &mut SycamoreContext) -> LazyNodes<'a, 'b>;
}

impl<T> IntoDioxus for Literal<T>
where
    T: IntoDioxus,
{
    fn into_dioxus<'a, 'b>(self, cfg: &SycamoreConfig, ctx: &mut SycamoreContext) -> LazyNodes<'a, 'b> {
        self.value.into_dioxus(cfg, ctx)
    }
}

impl IntoDioxus for ASTKind {
    fn into_dioxus<'a, 'b>(self, cfg: &SycamoreConfig, ctx: &mut SycamoreContext) -> LazyNodes<'a, 'b> {
        todo!()
    }
}
