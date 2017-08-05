#![doc = include_str!("readme.md")]

use crate::AdapterError;
use serde::{Deserialize, Serialize};
use std::ops::Range;
use url::Url;

#[derive(Debug)]
pub struct SourceText {
    raw: String,
    url: Option<Url>,
    lines_map: Vec<usize>,
}

impl SourceText {
    pub fn new(raw: impl Into<String>, url: Option<Url>) -> Self {
        let raw = raw.into();
        let mut lines_map = Vec::new();
        lines_map.push(0);
        for (i, c) in raw.char_indices() {
            if c == '\n' {
                lines_map.push(i + 1);
            }
        }
        Self {
            raw,
            url,
            lines_map,
        }
    }

    pub fn get_str(&self, range: Range<usize>) -> Result<&str, AdapterError> {
        match self.raw.get(range.start..range.end) {
            Some(s) => Ok(s),
            None => Err(AdapterError::invalid_range(range.start, range.end - range.start)),
        }
    }

    pub fn get_char(&self, offset: usize) -> Result<char, AdapterError> {
        match self.raw.get(offset..) {
            Some(s) => match s.chars().next() {
                Some(ch) => Ok(ch),
                None => Err(AdapterError::invalid_range(offset, 0)),
            },
            None => Err(AdapterError::invalid_range(offset, 0)),
        }
    }

    pub fn get_location(&self, offset: usize) -> SourceLocation {
        let line_index = match self.lines_map.binary_search(&offset) {
            Ok(index) => index,
            Err(index) => index - 1,
        };
        let line_start_offset = self.lines_map[line_index];
        let line = (line_index + 1) as u32;
        let column = (offset - line_start_offset + 1) as u32;
        SourceLocation {
            line,
            column,
            url: self.url.clone(),
        }
    }

    pub fn utf8_length(&self) -> usize {
        self.raw.len()
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourcePosition {
    pub offset: usize,
    pub length: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceLocation {
    pub line: u32,
    pub column: u32,
    pub url: Option<Url>,
}

impl Default for SourceLocation {
    fn default() -> Self {
        Self {
            line: 1,
            column: 1,
            url: None,
        }
    }
}

impl SourcePosition {
    pub fn add(&self, offset: usize) -> Self {
        Self {
            offset: self.offset + offset,
            length: self.length,
        }
    }
}
