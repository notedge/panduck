use super::*;

pub use self::list::ListConfig;

mod list;

pub const USED_PACKAGES: &str = r#"
\usepackage[utf8]{inputenc}
\usepackage{blindtext}
\usepackage{listings} % used for `Codeblock`
\usepackage{hyperref} % used for `SmarkLink`
\usepackage{csquotes} % used for `QuoteBlock`
"#;

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
