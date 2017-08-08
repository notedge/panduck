#![doc = include_str!("readme.md")]

use crate::ast::OrgRoot;
use crate::reader::OrgReadConfig;
use panduck_types::{AdapterError, Result};

pub struct OrgParser<'input> {
    pub(crate) config: &'input OrgReadConfig,
}

impl<'input> OrgParser<'input> {
    pub fn parse(&mut self) -> Result<OrgRoot> {
        Err(AdapterError::not_implemented("org parser"))
    }
}
