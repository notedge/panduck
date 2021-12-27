use crate::{LaTeXConfig, LaTeXContext};
use notedown_ast::{
    nodes::{Header, *},
    ASTKind, ASTNode,
};
use pretty::RcDoc;

mod text;

pub trait IntoLaTeX {
    fn into_latex(self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> RcDoc<()>;
}

impl IntoLaTeX for ASTNode {
    fn into_latex(self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> RcDoc<()> {
        self.value.into_latex(cfg, ctx)
    }
}

impl IntoLaTeX for ASTKind {
    fn into_latex(self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> RcDoc<()> {
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
            Self::MathNode(_) => {
                unimplemented!()
            }
            Self::LinkNode(_) => {
                unimplemented!()
            }
            Self::TextSpan(s) => {
                unimplemented!()
            }
            Self::StyledSpan(s) => {
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

impl IntoLaTeX for Header {
    fn into_latex(self, cfg: &LaTeXConfig, ctx: &mut LaTeXContext) -> RcDoc<()> {
        let _ = (cfg, ctx);
        todo!()
    }
}
