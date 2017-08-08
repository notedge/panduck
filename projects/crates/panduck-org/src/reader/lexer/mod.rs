#![doc = include_str!("readme.md")]

use crate::reader::token_type::OrgToken;
use crate::reader::{OrgReadConfig, OrgTokenType};
use panduck_types::{AdapterError, Result};

#[derive(Debug)]
pub struct OrgLexer<'input> {
    pub(crate) config: &'input OrgReadConfig,
}

impl<'input> OrgLexer<'input> {
    pub fn tokenize(self) -> Result<Vec<OrgToken>> {
        Err(AdapterError::not_implemented("org lexer"))
    }
}
