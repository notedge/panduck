use notedown_ast::ASTNode;
use notedown_error::Result;
use notedown_rt::{document::DocumentMeta, NoteDocument, NoteVM};
use serde::{Deserialize, Serialize};

use panduck_pp::{hard_break, text_ref};

use crate::traits::IntoLaTeX;

pub use self::{
    config::{ListConfig, TitleConfig, USED_PACKAGES},
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
    /// TODO: use [`NoteDocument`]
    pub fn render_article(&mut self, ast: &ASTNode) -> Result<String> {
        //
        let mut article = text_ref("\\documentclass{article}");
        article = article.append(hard_break(1)).append(USED_PACKAGES);
        let cfg = &self.config;
        let ctx = &mut self.context;
        article = article.append(hard_break(1)).append(ast.into_latex(cfg, ctx));
        //
        let mut out = String::new();
        article.render_fmt(cfg.width, &mut out)?;
        Ok(out)
    }

    pub fn render_title(&self, meta: &DocumentMeta) {}

    pub fn render_authors(&self, meta: &DocumentMeta) {}

    pub fn render_date(&self, meta: &DocumentMeta) {}

    pub fn render_ast(&mut self, ast: &ASTNode) -> Result<String> {
        let cfg = &self.config;
        let ctx = &mut self.context;
        let doc = ast.into_latex(cfg, ctx);
        let mut out = String::new();
        doc.render_fmt(cfg.width, &mut out)?;
        Ok(out)
    }
}
