#![doc = include_str!("readme.md")]

use crate::PanduckError;

/// 文本写入器，用于写入格式化的文本数据
///
/// 这个结构体提供了文本输出的格式化功能，包括缩进管理、
/// 自动换行和自定义缩进文本支持。
///
/// # 类型参数
///
/// * `W` - 底层的写入器类型，必须实现 `Write` trait
///
/// # 示例
///
/// ```rust
/// use gaia_types::writer::TextWriter;
/// use std::io::Cursor;
///
/// let mut buffer = Cursor::new(Vec::new());
/// let mut writer = TextWriter::new(buffer, "    ");
///
/// writer.write_line("function main() {").unwrap();
/// writer.indent();
/// writer.write_line("return 42;").unwrap();
/// writer.dedent();
/// writer.write_line("}").unwrap();
///
/// let result = String::from_utf8(writer.finish().into_inner()).unwrap();
/// assert_eq!(result, "function main() {\n    return 42;\n}\n");
/// ```
#[derive(Debug)]
pub struct TextWriter<W> {
    /// 底层的写入器
    writer: W,
    /// 当前的缩进级别
    indent_level: u16,
    /// 缩进文本（如空格或制表符）
    indent_text: &'static str,
    /// 诊断信息
    diagnostics: Vec<PanduckError>,
}

impl<W> TextWriter<W> {
    /// 创建一个新的文本写入器
    ///
    /// # 参数
    ///
    /// * `generator` - 底层的写入器
    /// * `indent_text` - 缩进文本，如 "    "（4个空格）或 "\t"（制表符）
    ///
    /// # 返回值
    ///
    /// 返回一个新的 `TextWriter` 实例
    ///
    /// # 示例
    ///
    /// ```rust
    /// use gaia_types::generator::TextWriter;
    /// use std::io::Cursor;
    ///
    /// let buffer = Cursor::new(Vec::new());
    /// let generator = TextWriter::new(buffer); // 使用2个空格缩进
    /// ```
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            indent_level: 0,
            indent_text: "    ",
            diagnostics: vec![], // Initialized with success
        }
    }

    pub fn add_error(&mut self, error: PanduckError) {
        self.diagnostics.push(error);
    }
}

impl<W: std::fmt::Write> TextWriter<W> {
    /// 增加缩进级别
    ///
    /// 调用此方法后，后续写入的文本会自动添加额外的缩进
    ///
    /// # 示例
    ///
    /// ```rust
    /// use gaia_types::writer::TextWriter;
    /// use std::io::Cursor;
    ///
    /// let mut buffer = Cursor::new(Vec::new());
    /// let mut writer = TextWriter::new(buffer, "    ");
    ///
    /// writer.write_line("outer {").unwrap();
    /// writer.indent(); // 增加缩进
    /// writer.write_line("inner code").unwrap();
    /// writer.dedent(); // 减少缩进
    /// writer.write_line("}").unwrap();
    /// ```
    pub fn indent(&mut self) {
        self.indent_level = self.indent_level.saturating_add(1);
    }

    /// 减少缩进级别
    ///
    /// 调用此方法后，后续写入的文本会减少缩进级别
    /// 如果当前缩进级别已经是0，则不会产生任何效果
    ///
    /// # 示例
    ///
    /// 参见 [`indent`] 方法的示例
    pub fn dedent(&mut self) {
        self.indent_level = self.indent_level.saturating_sub(1);
    }

    /// 写入一行文本
    ///
    /// 这个方法会自动添加当前缩进级别对应的缩进文本，
    /// 并在文本末尾添加换行符
    ///
    /// # 参数
    ///
    /// * `text` - 要写入的文本内容
    ///
    /// # 返回值
    ///
    /// 返回 `Result<()>`，如果写入成功则返回 `Ok(())`，否则返回错误
    ///
    /// # 示例
    ///
    /// ```rust
    /// use gaia_types::writer::TextWriter;
    /// use std::io::Cursor;
    ///
    /// let mut buffer = Cursor::new(Vec::new());
    /// let mut writer = TextWriter::new(buffer, "    ");
    ///
    /// writer.write_line("这是一行文本").unwrap();
    /// ```
    pub fn write_line(&mut self, text: &str) -> std::fmt::Result {
        self.write_indent()?;
        writeln!(self.writer, "{}", text)
    }

    /// 写入文本（不添加换行）
    ///
    /// 这个方法会写入文本内容，但不会自动添加换行符
    ///
    /// # 参数
    ///
    /// * `text` - 要写入的文本内容
    ///
    /// # 返回值
    ///
    /// 返回 `Result<()>`，如果写入成功则返回 `Ok(())`，否则返回错误
    ///
    /// # 示例
    ///
    /// ```rust
    /// use gaia_types::writer::TextWriter;
    /// use std::io::Cursor;
    ///
    /// let mut buffer = Cursor::new(Vec::new());
    /// let mut writer = TextWriter::new(buffer, "    ");
    ///
    /// writer.write("部分文本").unwrap();
    /// writer.write_line("继续文本").unwrap();
    /// ```
    pub fn write(&mut self, text: &str) -> std::fmt::Result {
        write!(self.writer, "{}", text)
    }

    /// 写入当前缩进级别的缩进文本
    ///
    /// 这个方法会写入与当前缩进级别对应的缩进文本
    ///
    /// # 返回值
    ///
    /// 返回 `Result<()>`，如果写入成功则返回 `Ok(())`，否则返回错误
    ///
    /// # 示例
    ///
    /// ```rust
    /// use gaia_types::writer::TextWriter;
    /// use std::io::Cursor;
    ///
    /// let mut buffer = Cursor::new(Vec::new());
    /// let mut writer = TextWriter::new(buffer, "    ");
    ///
    /// writer.indent();
    /// writer.write_indent().unwrap(); // 写入4个空格
    /// writer.write_line("缩进的文本").unwrap();
    /// ```
    pub fn write_indent(&mut self) -> std::fmt::Result {
        for _ in 0..self.indent_level {
            write!(self.writer, "{}", self.indent_text)?;
        }
        Ok(())
    }

    /// 获取当前的缩进级别
    ///
    /// # 返回值
    ///
    /// 返回当前的缩进级别（u16）
    ///
    /// # 示例
    ///
    /// ```rust
    /// use gaia_types::writer::TextWriter;
    /// use std::io::Cursor;
    ///
    /// let mut buffer = Cursor::new(Vec::new());
    /// let mut writer = TextWriter::new(buffer, "    ");
    ///
    /// assert_eq!(writer.indent_level(), 0);
    /// writer.indent();
    /// assert_eq!(writer.indent_level(), 1);
    /// ```
    pub fn indent_level(&self) -> u16 {
        self.indent_level
    }

    /// 获取内部写入器
    ///
    /// 这个方法会消耗 `TextWriter` 并返回内部的写入器
    ///
    /// # 返回值
    ///
    /// 返回内部的写入器 `W`
    ///
    /// # 示例
    ///
    /// ```rust
    /// use gaia_types::writer::TextWriter;
    /// use std::io::Cursor;
    ///
    /// let buffer = Cursor::new(Vec::new());
    /// let writer = TextWriter::new(buffer, "    ");
    /// let inner_writer = writer.finish();
    /// ```
    pub fn finish(self) -> W {
        self.writer
    }
}
