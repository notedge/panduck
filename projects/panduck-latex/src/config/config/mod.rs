use super::*;

pub use self::list::ListConfig;

mod list;

impl Default for LaTeXConfig {
    fn default() -> Self {
        Self { width: 144, list_config: Default::default() }
    }
}

impl LaTeXConfig {
    pub fn into_builder(self) -> LaTeXBuilder {
        LaTeXBuilder { config: self, context: LaTeXContext { vm: Default::default() } }
    }
}
