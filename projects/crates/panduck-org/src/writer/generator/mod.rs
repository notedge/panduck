#![doc = include_str!("readme.md")]

use crate::ast::OrgRoot;
use crate::writer::OrgWriteConfig;
use panduck_types::{AdapterError, Result, TextWriter};
use std::fmt::Write;

#[derive(Debug)]
pub struct OrgWriter<'input, W> {
    writer: TextWriter<W>,
    config: &'input OrgWriteConfig,
}

impl OrgWriteConfig {
    pub fn writer<W: Write>(&self, writer: W) -> OrgWriter<W> {
        OrgWriter {
            writer: TextWriter::new(writer),
            config: self,
        }
    }
}

impl<'input, W: Write> OrgWriter<'input, W> {
    pub fn generate(self, _ast: &OrgRoot) -> Result<W> {
        Err(AdapterError::not_implemented("org writer"))
    }
}
