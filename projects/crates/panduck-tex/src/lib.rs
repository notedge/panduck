#![deny(missing_debug_implementations, missing_copy_implementations)]
#![warn(missing_docs, rustdoc::missing_crate_level_docs)]
#![doc = include_str!("../readme.md")]

use panduck_types::{AdapterError, Result};

#[derive(Copy, Clone, Debug)]
pub struct MarkdownReadConfig {
    pub support_math: bool,
}

#[derive(Copy, Clone, Debug)]
pub struct MarkdownWriteConfig {}

pub fn parse_and_generate_markdown(
    _markdown_input: &str,
    _config: MarkdownReadConfig,
) -> Result<String> {
    Err(AdapterError::not_implemented("tex markdown pipeline"))
}
