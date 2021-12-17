#[cfg(feature = "katex")]
mod katex;

use self::katex::KatexConfig;

pub struct MathConfig {
    pub katex_config: KatexConfig,
}

impl Default for MathConfig {
    fn default() -> Self {
        Self { katex_config: Default::default() }
    }
}
