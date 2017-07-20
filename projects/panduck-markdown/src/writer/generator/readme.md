# Gaia Types - 汇编器模块

这个模块提供了文本和二进制数据的写入功能，是 Gaia 项目的核心写入基础设施。

## 功能概述

### BinaryAssembler
- **二进制写入**: 高效写入二进制数据
- **字节序支持**: 支持大端和小端字节序
- **类型安全**: 类型安全的二进制数据写入
- **性能优化**: 优化的写入性能

### TextWriter
- **文本写入**: 格式化文本输出
- **编码支持**: 支持多种文本编码
- **缓冲机制**: 高效的缓冲写入机制

## 使用示例

### BinaryAssembler 使用

```rust
use panduck_core::BinaryAssembler;
use byteorder::LittleEndian;

let mut assembler = BinaryAssembler::new(Vec::new());

// 写入不同类型的数据
assembler.write_u32::<LittleEndian>(42)?;
assembler.write_f64::<LittleEndian>(3.14)?;
assembler.write_bytes(b"Hello, World!")?;

let result = assembler.finish();
```

### TextWriter 使用

```rust
use panduck_core::TextWriter;

let mut writer = TextWriter::new(Vec::new());

// 写入格式化的文本
writer.write_str("Hello, World!")?;
writer.write_fmt(format_args!("Number: {}", 42))?;

let result = writer.finish();
```

## 设计特点

- **泛型设计**: 支持多种底层写入器类型
- **零拷贝优化**: 最小化内存拷贝操作
- **错误处理**: 完善的错误处理机制
- **扩展性**: 易于扩展新的写入功能

## 性能特性

- **高效缓冲**: 内部缓冲机制减少系统调用
- **内存安全**: 自动内存管理和边界检查
- **类型安全**: 编译时类型检查防止数据损坏
- **跨平台**: 支持所有主流平台