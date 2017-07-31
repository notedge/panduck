#![doc = include_str!("readme.md")]

use crate::PanduckError;
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
    pub fn new(raw: String, url: Option<Url>) -> Self {
        let mut lines_map = Vec::new();
        lines_map.push(0); // The first line starts at offset 0
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

    pub fn get_str(&self, range: Range<usize>) -> Result<&str, PanduckError> {
        match self.raw.get(range.start..range.end) {
            Some(s) => Ok(s),
            None => Err(PanduckError::invalid_range(range.end, self.raw.len())),
        }
    }

    pub fn get_char(&self, offset: usize) -> Result<char, PanduckError> {
        match self.raw.get(offset..) {
            Some(s) => match s.chars().next() {
                Some(s) => Ok(s),
                None => Err(PanduckError::invalid_range(offset, self.raw.len())),
            },
            None => Err(PanduckError::invalid_range(offset, self.raw.len())),
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

/// 源代码位置信息，表示代码在源文件中的位置
///
/// 该结构体用于跟踪源代码的位置信息，包括行号、列号等。
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourcePosition {
    /// 字节偏移量，从 0 开始计数
    ///
    /// 表示从文件开始到当前位置的字节偏移量。
    pub offset: usize,
    /// 长度，表示该位置所覆盖的字节数
    ///
    /// 通常用于表示标记或符号的长度。
    pub length: usize,
}

/// 源代码位置，包含文件 URL 和位置信息
///
/// 该结构体扩展了 SourcePosition，增加了文件 URL 信息，
/// 可以表示代码在特定文件中的位置。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceLocation {
    /// 行号，从 1 开始计数
    ///
    /// 表示当前位置所在的行号。
    pub line: u32,
    /// 列号，从 1 开始计数
    ///
    /// 表示当前位置所在的列号。
    pub column: u32,
    /// 源文件的 URL，可选
    ///
    /// 如果存在，表示包含该代码的文件的 URL 或路径。
    /// 可以是文件系统路径或网络 URL。
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