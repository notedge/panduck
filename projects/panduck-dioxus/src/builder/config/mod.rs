use panduck_html::{CodeConfig, ImageConfig, MathConfig};

use super::*;

pub struct DioxusConfig {
    pub trust_raw_html: bool,
    pub image_config: ImageConfig,
    pub code_config: CodeConfig,
    pub math_config: MathConfig,
}

impl Default for DioxusConfig {
    fn default() -> Self {
        Self {
            trust_raw_html: false,
            image_config: Default::default(),
            code_config: Default::default(),
            math_config: Default::default(),
        }
    }
}

impl DioxusConfig {
    pub fn into_builder(self) -> DioxusBuilder {
        DioxusBuilder { config: self, context: Default::default() }
    }
}
