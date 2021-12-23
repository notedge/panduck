mod parse;

use panduck_html::HTMLConfig;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PanduckConfig {
    pub verbose: u32,
    pub html: HTMLConfig,
}

impl Default for PanduckConfig {
    fn default() -> Self {
        Self { verbose: 0, html: Default::default() }
    }
}
