use crate::helpers::SourcePosition;
use serde::{Deserialize, Serialize};
use std::ops::Range;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Token<T> {
    pub token_type: T,
    pub position: SourcePosition,
}

impl<T: Copy> Token<T> {
    pub fn get_range(&self) -> Range<usize> {
        let start = self.position.offset;
        let end = start + self.position.length;
        start..end
    }
}
