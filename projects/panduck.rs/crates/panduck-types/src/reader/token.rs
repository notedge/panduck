use crate::helpers::SourcePosition;
use serde::{Deserialize, Serialize};
use std::ops::Range;

/// Lexed token with byte span metadata in the source buffer.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Token<T> {
    /// Classifier value for the lexeme.
    pub token_type: T,
    /// Byte span of the token in the parent [`SourceText`](crate::helpers::SourceText).
    pub position: SourcePosition,
}

impl<T: Copy> Token<T> {
    /// Returns the inclusive-exclusive byte range covered by this token.
    pub fn get_range(&self) -> Range<usize> {
        let start = self.position.offset;
        let end = start + self.position.length;
        start..end
    }
}
