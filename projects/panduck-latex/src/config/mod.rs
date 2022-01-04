use notedown_ast::ASTNode;
use notedown_error::Result;
use notedown_rt::{NoteDocument, NoteVM};
use serde::{Deserialize, Serialize};

use crate::traits::IntoLaTeX;

pub use self::{
    config::{ListConfig, TitleConfig},
    context::LaTeXContext,
};

mod config;
mod context;

pub struct LaTeXBuilder {
    config: LaTeXConfig,
    context: LaTeXContext,
}

// static info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaTeXConfig {
    pub width: usize,
    pub title_config: TitleConfig,
    pub list_config: ListConfig,
}

impl LaTeXBuilder {
    pub fn render_ast(&mut self, ast: &ASTNode) -> Result<String> {
        let cfg = &self.config;
        let ctx = &mut self.context;
        let doc = ast.into_latex(cfg, ctx);
        let mut out = String::new();
        doc.render_fmt(cfg.width, &mut out)?;
        Ok(out)
    }
}
