use notedown_ast::{
    nodes::{Header, *},
    ASTKind, ASTNode,
};
use pretty::RcDoc;

use crate::{LaTeXConfig, LaTeXContext};

pub use self::utils::*;

mod math;
mod text;
mod utils;

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
            Self::Statements(s) => {
                RcDoc::intersperse(s.iter().map(|x| x.into_latex(cfg, ctx)), block_break()).group()
            }
            Self::Paragraph(s) => {
                RcDoc::intersperse(s.iter().map(|x| x.into_latex(cfg, ctx)), new_line()).group()
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
            Self::CodeNode(c) => {
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
        // Assuming the level setter is used, input is legal
        let mut header = match self.level {
            1 => RcDoc::text("\\chapter").append("{"),
            2 => RcDoc::text("\\section").append("{"),
            3 => RcDoc::text("\\subsection").append("{"),
            4 => RcDoc::text("\\subsubsection").append("{"),
            5 => RcDoc::text("\\paragraph").append("{"),
            _ => RcDoc::text("\\subparagraph").append("{"),
        };

        let inner =
            RcDoc::intersperse(self.children.iter().map(|x| x.into_latex(cfg, ctx)), new_line())
                .nest(1)
                .group();
        header = header.append(inner).append("}");
        header = header.append(new_line());
        match &self.id {
            None => {}
            Some(id) => {
                header = header.append("\\label{").append(id).append("}");
            }
        }
        return header;
    }
}
