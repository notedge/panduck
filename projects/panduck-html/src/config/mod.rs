mod code;
mod image;
mod math;

pub use self::{code::CodeConfig, image::ImageConfig, math::MathConfig};

use serde::{Deserialize, Serialize};

pub struct HTMLBuilder {
    config: HTMLConfig,
    context: HTMLContext,
}

#[derive(Serialize, Deserialize)]
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

impl HTMLConfig {
    pub fn into_builder(self) -> HTMLBuilder {
        HTMLBuilder { config: Default::default(), context: HTMLContext {} }
    }
}
