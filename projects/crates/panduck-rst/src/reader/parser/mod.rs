use panduck_types::helpers::SourceText;
use panduck_types::{AdapterError, Result};

use crate::ast::RstRoot;
use crate::reader::token_type::RstToken;

#[derive(Copy, Clone, Debug)]
pub struct ParserState;

pub fn parse(_tokens: Vec<RstToken>, _source: SourceText) -> Result<RstRoot> {
    Err(AdapterError::not_implemented("rst parser"))
}
