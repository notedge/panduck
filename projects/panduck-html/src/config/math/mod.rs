#[cfg(feature = "katex")]
mod katex;

use self::katex::KatexConfig;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MathConfig {
    pub katex_config: KatexConfig,
}

impl Default for MathConfig {
    fn default() -> Self {
        Self { katex_config: Default::default() }
    }
}
