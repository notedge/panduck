use panduck_types::helpers::SourceText;
use panduck_types::{AdapterError, Result};

pub use crate::reader::token_type::{RstToken, RstTokenType};

#[derive(Copy, Clone, Debug)]
pub struct RstReadConfig;

pub fn tokenize(source: impl Into<String>) -> Result<Vec<RstToken>> {
    Err(AdapterError::not_implemented("rst lexer"))
}
