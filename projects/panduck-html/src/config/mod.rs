use serde::{Deserialize, Serialize};

pub use self::{code::CodeConfig, image::ImageConfig, math::MathConfig};

mod code;
mod image;
mod math;

pub struct HTMLBuilder {
    config: HTMLConfig,
    context: HTMLContext,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HTMLConfig {
    pub trust_raw_html: bool,
    pub image_config: ImageConfig,
    pub code_config: CodeConfig,
    pub math_config: MathConfig,
}

pub struct HTMLContext {}

impl Default for HTMLConfig {
    fn default() -> Self {
        Self {
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
