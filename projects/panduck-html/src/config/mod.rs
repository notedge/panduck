mod code;
mod image;
mod math;

pub use self::{code::CodeConfig, image::ImageConfig, math::MathConfig};

pub struct HTMLConfig {
    pub trust_raw_html: bool,
    pub image_config: ImageConfig,
    pub code_config: CodeConfig,
    pub math_config: MathConfig,
}

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
