#![doc = include_str!("readme.md")]

use crate::ast::OrgRoot;
use panduck_core::PanduckDiagnostics;

use crate::reader::{OrgReadConfig, OrgTokenType};

pub struct OrgParser<'input> {
    pub(crate) state: panduck_core::parser::ParserState<'input, OrgTokenType>,
    pub(crate) config: &'input OrgReadConfig,
}

impl<'input> OrgParser<'input> {
    pub fn parse(&mut self) -> PanduckDiagnostics<OrgRoot> {
        todo!()
    }
}