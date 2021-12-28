use super::*;

// static info
#[derive(Debug, Clone)]
pub struct LaTeXConfig {
    pub width: usize,
}

impl Default for LaTeXConfig {
    fn default() -> Self {
        Self { width: 144 }
    }
}

impl LaTeXConfig {
    pub fn into_builder(self) -> LaTeXBuilder {
        LaTeXBuilder { config: self, context: LaTeXContext { vm: Default::default() } }
    }
}
