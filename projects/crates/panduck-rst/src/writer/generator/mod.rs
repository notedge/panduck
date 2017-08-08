use panduck_types::{AdapterError, Result};

use crate::ast::RstRoot;

#[derive(Copy, Clone, Debug, Default)]
pub struct RstWriter;

impl RstWriter {
    pub fn new() -> Self {
        Self
    }

    pub fn generate(&self, _root: RstRoot) -> Result<String> {
        Err(AdapterError::not_implemented("rst writer"))
    }
}

pub fn generate(_root: RstRoot) -> Result<String> {
    Err(AdapterError::not_implemented("rst writer"))
}
