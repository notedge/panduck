use std::fmt::Write;

mod generator;
mod ir;

pub use self::generator::MarkdownWriter;
pub use self::ir::write_document_markdown;

/// 二进制写入器，用于从实现了 WriteBytesExt trait 的类型中写入数据
///
/// 这是一个泛型结构体，可以包装任何实现了 WriteBytesExt trait 的类型，
/// 提供二进制数据的写入功能。
#[derive(Copy, Clone, Debug)]
pub struct MarkdownWriteConfig {
    pub support_math: bool,
}

impl Default for MarkdownWriteConfig {
    fn default() -> Self {
        Self {
            support_math: false,
        }
    }
}
