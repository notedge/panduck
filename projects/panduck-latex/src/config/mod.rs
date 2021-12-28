use notedown_ast::{ASTNode, Result};
use notedown_rt::NoteVM;

use crate::traits::IntoLaTeX;

pub use self::{config::LaTeXConfig, context::LaTeXContext};

mod config;
mod context;

pub struct LaTeXBuilder {
    config: LaTeXConfig,
    context: LaTeXContext,
}

impl LaTeXBuilder {
    pub fn render_latex(&mut self, ast: &ASTNode) -> Result<String> {
        let cfg = &self.config;
        let ctx = &mut self.context;
        let doc = ast.into_latex(cfg, ctx);
        let mut out = String::new();
        doc.render_fmt(cfg.width, &mut out)?;
        Ok(out)
    }
}
