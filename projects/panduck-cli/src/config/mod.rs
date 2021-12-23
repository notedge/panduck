mod parse;

use panduck_html::HTMLConfig;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct PanduckConfig {
    html: HTMLConfig,
}
