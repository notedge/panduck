use katex::{error::Error, render_with_opts, OutputType};
use notedown_ast::nodes::{MathKind, MathNode};
use notedown_error::{NoteError, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KatexConfig {}

impl Default for KatexConfig {
    fn default() -> Self {
        Self {}
    }
}

impl KatexConfig {
    pub fn render_html(&self, math: &MathNode) -> Result<String> {
        match math.kind {
            MathKind::Inline => katex_inline(&math.raw),
            MathKind::Display => katex_display(&math.raw),
            MathKind::BlockInline => katex_inline(&math.raw),
            MathKind::BlockDisplay => katex_display(&math.raw),
        }
    }
}

pub fn katex_inline(input: &str) -> Result<String> {
    let mut cfg = katex::Opts::default();
    cfg.set_output_type(OutputType::Html);
    handler(render_with_opts(input, &cfg))
}

pub fn katex_display(input: &str) -> Result<String> {
    let mut cfg = katex::Opts::default();
    cfg.set_output_type(OutputType::Html);
    cfg.set_display_mode(true);
    handler(render_with_opts(input, &cfg))
}
fn handler<T>(out: katex::error::Result<T, Error>) -> Result<T> {
    match out {
        Ok(o) => Ok(o),
        Err(e) => Err(NoteError::runtime_error(format!("{:?}", e))),
    }
}
