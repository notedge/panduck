use notedown_ast::{
    nodes::{Header, *},
    ASTKind, ASTNode,
};
use pretty::RcDoc;

use crate::{LaTeXConfig, LaTeXContext};

mod math;
mod text;

pub trait IntoLaTeX {
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> RcDoc<'a, ()>;
}

impl IntoLaTeX for ASTNode {
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> RcDoc<'a, ()> {
        self.value.into_latex(cfg, ctx)
    }
}

impl IntoLaTeX for ASTKind {
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> RcDoc<'a, ()> {
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
            Self::Header(v) => v.into_latex(cfg, ctx),
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
            Self::MathNode(s) => s.into_latex(cfg, ctx),
            Self::LinkNode(_) => {
                unimplemented!()
            }
            Self::TextSpan(s) => s.into_latex(cfg, ctx),
            Self::StyledSpan(s) => s.into_latex(cfg, ctx),
            Self::Command(_) => {
                unimplemented!()
            }
            Self::Value(_) => {
                unimplemented!()
            }
        }
    }
}

impl IntoLaTeX for Header {
    fn into_latex<'a>(&'a self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> RcDoc<'a, ()> {
        let _ = (cfg, ctx);
        todo!()
    }
}
