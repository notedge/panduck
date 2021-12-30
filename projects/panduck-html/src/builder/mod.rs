use notedown_ast::{ASTNode, Result};
use serde::{Deserialize, Serialize};

use crate::IntoHTML;

pub use self::{code::CodeConfig, config::*, image::ImageConfig, math::MathConfig};

mod code;
mod config;
mod image;
mod math;

pub struct HTMLBuilder {
    config: HTMLConfig,
    context: HTMLContext,
}

impl HTMLBuilder {
    pub fn render(&mut self, node: &ASTNode) -> Result<String> {
        let mut buffer = String::with_capacity(1000);
        node.into_html(&self.config, &mut self.context).render_fmt(self.config.max_width, &mut buffer)?;
        Ok(buffer.to_owned())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HTMLConfig {
    pub xhtml: XHtml,
    pub indent: usize,
    pub max_width: usize,
    pub trust_raw_html: bool,
    pub image_config: ImageConfig,
    pub code_config: CodeConfig,
    pub math_config: MathConfig,
}

pub struct HTMLContext {}

impl Default for HTMLConfig {
    fn default() -> Self {
        Self {
            xhtml: XHtml::False,
            indent: 4,
            max_width: 100,
            trust_raw_html: true,
            image_config: Default::default(),
            code_config: Default::default(),
            math_config: Default::default(),
        }
    }
}

impl Default for HTMLContext {
    fn default() -> Self {
        Self {}
    }
}

impl HTMLBuilder {
    // pub fn build_html(&mut self, tree: ASTNode) {
    //     tree.into_sycamore(&self.context)
    // }
}

impl HTMLConfig {
    pub fn into_builder(self) -> HTMLBuilder {
        HTMLBuilder { config: Default::default(), context: HTMLContext {} }
    }
}
