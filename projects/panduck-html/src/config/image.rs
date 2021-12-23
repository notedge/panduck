use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImageConfig {
    pub lazy_loading: bool,
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self { lazy_loading: false }
    }
}

impl HTMLConfig {
    #[inline]
    pub fn get_image_lazy_loading(&mut self) -> &bool {
        &self.image_config.lazy_loading
    }
    #[inline]
    pub fn set_image_lazy_loading(&mut self, config: bool) {
        self.image_config.lazy_loading = config
    }
}
