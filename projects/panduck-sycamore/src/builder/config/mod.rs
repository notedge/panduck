use panduck_html::{CodeConfig, ImageConfig, MathConfig};

use super::*;

pub struct SycamoreConfig {
    pub(crate) trust_raw_html: bool,
    pub(crate) image_config: ImageConfig,
    pub(crate) code_config: CodeConfig,
    pub(crate) math_config: MathConfig,
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
    pub fn into_builder(self) -> SycamoreBuilder {
        SycamoreBuilder { config: self, context: Default::default() }
    }
}
