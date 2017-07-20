# Gaia Types - 读取器模块

这个模块提供了 Gaia 项目的读取功能，包括二进制数据读取、源代码位置跟踪和错误定位等功能。

## 功能概述

### BinaryReader
- **二进制读取**: 高效读取二进制数据
- **字节序支持**: 支持大端和小端字节序
- **边界检查**: 安全的边界检查机制
- **性能优化**: 优化的读取性能

### 源代码位置跟踪
- **SourceLocation**: 源代码位置信息
- **SourcePosition**: 精确的行列位置
- **SourceSpan**: 源代码范围信息
- **错误定位**: 精确的错误位置报告

### TokenStream
- **标记流**: 标记序列的抽象表示
- **位置跟踪**: 标记的精确位置信息
- **错误报告**: 基于位置的错误报告
- **性能优化**: 高效的标记流处理

## 使用示例

### 二进制读取

```rust
use gaia_types::reader::{BinaryReader, Endianness};

let data = vec![0x01, 0x02, 0x03, 0x04];
let mut reader = BinaryReader::new(&data);

// 读取不同类型的数据
let byte = reader.read_u8()?;
let word = reader.read_u16::<Endianness::LittleEndian>()?;
let dword = reader.read_u32::<Endianness::BigEndian>()?;
```

### 源代码位置跟踪

```rust
use gaia_types::reader::{SourceLocation, SourcePosition};

// 创建位置信息
let position = SourcePosition::new(10, 5); // 第10行，第5列
let location = SourceLocation::new("example.wat", position);

// 在错误中使用
let error = GaiaError::syntax_error("语法错误", location);
```

### TokenStream 使用

```rust
use gaia_types::reader::TokenStream;
use gaia_types::lexer::{Lexer, Token};

let source = "(module (func))";
let lexer = Lexer::new(source);
let tokens: Vec<Token> = lexer.collect()?;
let token_stream = TokenStream::new(tokens);

// 遍历标记流
for token in token_stream {
    println!("Token: {:?} at {:?}", token.kind(), token.location());
}
```

## 设计特点

### 性能优化
- **零拷贝**: 最小化数据拷贝操作
- **缓存友好**: 优化的内存访问模式
- **流式处理**: 支持流式数据处理
- **内存池**: 使用内存池减少分配开销

### 安全性
- **边界检查**: 严格的边界检查防止越界访问
- **类型安全**: 类型安全的读取操作
- **错误处理**: 完善的错误处理机制
- **内存安全**: 自动内存管理和生命周期管理

### 精确位置跟踪
- **字符级别**: 精确的字符级别位置跟踪
- **行列信息**: 准确的行列位置信息
- **范围信息**: 标记和错误的范围信息
- **上下文**: 错误发生时的上下文信息

## 二进制格式支持

### WebAssembly 格式
- **魔数验证**: WebAssembly 魔数验证
- **版本检查**: 版本号检查和验证
- **段解析**: 各个段的解析和处理
- **自定义段**: 自定义段的读取和处理

### 自定义格式
- **扩展性**: 支持自定义二进制格式
- **验证**: 格式验证和完整性检查
- **性能**: 高效的格式解析
- **兼容性**: 向前和向后兼容性

## 错误处理

### 读取错误
- **UnexpectedEof**: 意外的文件结束
- **InvalidData**: 无效的数据格式
- **OutOfBounds**: 越界访问
- **InvalidUtf8**: 无效的 UTF-8 编码

### 位置错误
- **InvalidLocation**: 无效的位置信息
- **MissingLocation**: 缺失位置信息
- **LocationMismatch**: 位置信息不匹配

## 内存管理

### 生命周期管理
- **引用安全**: 安全的引用管理
- **所有权**: 清晰的所有权模型
- **借用检查**: 编译时借用检查
- **内存回收**: 自动内存回收

### 性能特性
- **分配优化**: 最小化内存分配
- **缓存优化**: 优化的缓存使用
- **并发安全**: 线程安全的操作
- **资源管理**: 高效的资源管理