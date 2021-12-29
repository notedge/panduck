use serde::{Deserialize, Serialize};

#[cfg(feature = "katex")]
use self::katex::KatexConfig;

#[cfg(feature = "katex")]
mod katex;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MathConfig {
    #[cfg(feature = "katex")]
    pub katex_config: KatexConfig,
}

impl Default for MathConfig {
    fn default() -> Self {
        Self {
            #[cfg(feature = "katex")]
            katex_config: Default::default(),
        }
    }
}
