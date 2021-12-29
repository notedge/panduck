use notedown_ast::{nodes::StyleKind, ASTKind, ASTNode, Result};
use pretty::RcDoc;

use crate::HTMLConfig;

mod nodes;
mod text;

pub trait PrettyHTML {
    fn pretty_html(&self, f: &mut PrettyRenderer) -> RcDoc<()>;
}

pub struct PrettyRenderer {
    xhtml: bool,
    max_width: usize,
    config: HTMLConfig,
}

impl Default for PrettyRenderer {
    fn default() -> Self {
        Self { xhtml: false, max_width: 144, config: Default::default() }
    }
}

impl PrettyRenderer {
    pub fn render(&mut self, node: &ASTNode) -> Result<String> {
        let mut buffer = String::with_capacity(1000);
        node.pretty_html(self).render_fmt(self.max_width, &mut buffer)?;
        Ok(buffer.to_owned())
    }
}
