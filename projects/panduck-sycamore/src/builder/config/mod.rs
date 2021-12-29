use panduck_html::ImageConfig;

use super::*;

pub struct SycamoreConfig {
    pub trust_raw_html: bool,
    pub image_config: ImageConfig,
    pub code_config: CodeConfig,
}

impl Default for SycamoreConfig {
    fn default() -> Self {
        Self { trust_raw_html: false, image_config: Default::default() }
    }
}

impl SycamoreConfig {
    pub fn into_builder(self) -> SycamoreBuilder {
        SycamoreBuilder { cfg: self, ctx: Default::default() }
    }
}
