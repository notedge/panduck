use std::borrow::Borrow;

use notedown_ast::{
    nodes::{Header, *},
    ASTKind, ASTNode, Command, Value,
};
use notedown_rt::NoteDocument;

use panduck_pp::*;

use crate::{LaTeXConfig, LaTeXContext};

pub use self::utils::*;

mod blocks;
mod cmd;
mod code;
mod link;
mod list;
mod math;
mod quote;
mod table;
mod text;
mod utils;

pub trait IntoLaTeX {
    //noinspection RsSelfConvention
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> PrettyPrint<'a>;
}

impl IntoLaTeX for NoteDocument {
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> PrettyPrint<'a> {
        todo!()
    }
}

impl IntoLaTeX for ASTNode {
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> PrettyPrint<'a> {
        self.value.into_latex(cfg, ctx)
    }
}

impl IntoLaTeX for ASTKind {
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> PrettyPrint<'a> {
        match self {
            Self::Statements(s) => {
                PrettyPrint::intersperse(s.iter().map(|x| x.into_latex(cfg, ctx)), hard_break(2))
                    .group()
            }
            Self::Paragraph(s) => {
                PrettyPrint::intersperse(s.iter().map(|x| x.into_latex(cfg, ctx)), nil_or_newline())
                    .group()
            }
            Self::Delimiter(v) => v.into_latex(cfg, ctx),
            Self::Header(v) => v.into_latex(cfg, ctx),
            Self::TableView(v) => v.into_latex(cfg, ctx),
            Self::ListView(v) => v.into_latex(cfg, ctx),
            Self::QuoteNode(v) => v.into_latex(cfg, ctx),
            Self::CodeNode(v) => v.into_latex(cfg, ctx),
            Self::MathNode(v) => v.into_latex(cfg, ctx),
            Self::LinkNode(v) => v.into_latex(cfg, ctx),
            Self::TextSpan(v) => v.into_latex(cfg, ctx),
            Self::StyledSpan(v) => v.into_latex(cfg, ctx),
            Self::Command(v) => v.into_latex(cfg, ctx),
            Self::Value(v) => v.into_latex(cfg, ctx),
        }
    }
}
