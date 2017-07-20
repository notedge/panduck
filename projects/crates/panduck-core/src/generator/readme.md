# Writer 模块

Writer 模块提供了文本和二进制数据的写入功能，支持格式化的文本输出和结构化的二进制数据写入。

## 主要组件

### TextWriter

文本写入器，用于生成格式化的文本输出，支持缩进管理和自定义缩进文本。

```rust
pub struct TextWriter<W: Write> {
    writer: W,
    indent_level: u16,
    indent_text: &'static str,
}
```

#### 特性

- **缩进管理**: 支持嵌套缩进，自动处理缩进级别
- **自定义缩进**: 可配置缩进文本（默认为空格）
- **格式化输出**: 提供便捷的文本格式化方法

#### 使用示例

```rust
use gaia_types::writer::TextWriter;
use std::io::Cursor;

let mut buffer = Cursor::new(Vec::new());
let mut writer = TextWriter::new(buffer, "    ");

writer.write_line("function main() {").unwrap();
writer.indent();
writer.write_line("return 42;").unwrap();
writer.dedent();
writer.write_line("}").unwrap();

let result = String::from_utf8(writer.into_inner().into_inner()).unwrap();
assert_eq!(result, "function main() {\n    return 42;\n}\n");
```

### BinaryWriter

二进制写入器，用于生成结构化的二进制数据，支持字节序控制和类型安全的写入操作。

```rust
pub struct BinaryWriter<W: Write> {
    writer: W,
    byte_order: ByteOrder,
}
```

#### 特性

- **字节序控制**: 支持大端序和小端序写入
- **类型安全**: 提供类型安全的写入方法
- **错误处理**: 完善的错误处理机制

#### 使用示例

```rust
use gaia_types::writer::BinaryWriter;
use std::io::Cursor;

let mut buffer = Cursor::new(Vec::new());
let mut writer = BinaryWriter::new(buffer);

writer.write_u32(0x12345678).unwrap();
writer.write_string("hello").unwrap();

let data = writer.into_inner().into_inner();
// 处理生成的二进制数据
```

## API 参考

### TextWriter 方法

- `new(writer: W, indent_text: &'static str) -> Self` - 创建新的文本写入器
- `indent(&mut self)` - 增加缩进级别
- `dedent(&mut self)` - 减少缩进级别
- `write_line(&mut self, text: &str) -> Result<()>` - 写入一行文本（自动添加换行）
- `write(&mut self, text: &str) -> Result<()>` - 写入文本（不添加换行）
- `into_inner(self) -> W` - 获取内部写入器

### BinaryWriter 方法

- `new(writer: W) -> Self` - 创建新的二进制写入器
- `set_byte_order(&mut self, byte_order: ByteOrder)` - 设置字节序
- `write_u8(&mut self, value: u8) -> Result<()>` - 写入无符号8位整数
- `write_u16(&mut self, value: u16) -> Result<()>` - 写入无符号16位整数
- `write_u32(&mut self, value: u32) -> Result<()>` - 写入无符号32位整数
- `write_u64(&mut self, value: u64) -> Result<()>` - 写入无符号64位整数
- `write_string(&mut self, value: &str) -> Result<()>` - 写入字符串（带长度前缀）
- `into_inner(self) -> W` - 获取内部写入器

## 开发指南

### 扩展 TextWriter

要添加新的格式化方法，可以在 `TextWriter` 的实现中添加：

```rust
impl<W: Write> TextWriter<W> {
    pub fn write_indented_line(&mut self, text: &str) -> Result<()> {
        self.write_indent()?;
        self.write_line(text)
    }
    
    pub fn write_indent(&mut self) -> Result<()> {
        for _ in 0..self.indent_level {
            write!(self.writer, "{}", self.indent_text)?;
        }
        Ok(())
    }
}
```

### 扩展 BinaryWriter

要添加新的数据类型支持，可以在 `BinaryWriter` 的实现中添加：

```rust
impl<W: Write> BinaryWriter<W> {
    pub fn write_bool(&mut self, value: bool) -> Result<()> {
        self.write_u8(if value { 1 } else { 0 })
    }
    
    pub fn write_f32(&mut self, value: f32) -> Result<()> {
        // 实现浮点数写入逻辑
        Ok(())
    }
}
```