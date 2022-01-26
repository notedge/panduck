use panduck_html::{CodeConfig, ImageConfig, MathConfig};

use super::*;

pub struct SycamoreConfig {
    pub trust_raw_html: bool,
    pub image_config: ImageConfig,
    pub code_config: CodeConfig,
    pub math_config: MathConfig,
}

impl Default for SycamoreConfig {
    fn default() -> Self {
        Self {
            trust_raw_html: false,
            image_config: Default::default(),
            code_config: Default::default(),
            math_config: Default::default(),
        }
    }
}

impl SycamoreConfig {
    pub fn into_builder(self) -> DioxusBuilder {
        DioxusBuilder { config: self, context: Default::default() }
    }
}
