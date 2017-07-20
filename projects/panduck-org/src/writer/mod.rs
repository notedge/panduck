use std::fmt::Write;

pub mod generator;
pub use panduck_markdown::writer::{MarkdownWriter, MarkdownWriteConfig};

/// 二进制写入器，用于从实现了 WriteBytesExt trait 的类型中写入数据
///
/// 这是一个泛型结构体，可以包装任何实现了 WriteBytesExt trait 的类型，
/// 提供二进制数据的写入功能。
#[derive(Copy, Clone, Debug)]
pub struct OrgWriteConfig {
    pub markdown_config: MarkdownWriteConfig,
}

impl Default for OrgWriteConfig {
    fn default() -> Self {
        Self {
            markdown_config: MarkdownWriteConfig::default(),
        }
    }
}