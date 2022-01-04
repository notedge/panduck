use super::*;

pub use self::list::ListConfig;

mod list;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TitleConfig {}

impl Default for TitleConfig {
    fn default() -> Self {
        Self {}
    }
}

impl Default for LaTeXConfig {
    fn default() -> Self {
        Self { width: 144, title_config: Default::default(), list_config: Default::default() }
    }
}

impl LaTeXConfig {
    pub fn into_builder(self) -> LaTeXBuilder {
        LaTeXBuilder { config: self, context: Default::default() }
    }
}
