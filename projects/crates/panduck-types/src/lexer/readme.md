# Gaia Types - 词法分析器模块

这个模块提供了 Gaia 项目的词法分析功能，负责将源代码文本转换为标记（tokens）序列。

## 功能概述

### 词法分析
- **标记识别**: 识别关键字、标识符、字面量、运算符等
- **源代码定位**: 精确的源代码位置信息
- **错误处理**: 友好的词法错误报告
- **性能优化**: 高效的标记化算法

### 支持的标记类型
- **关键字**: WebAssembly 和 Gaia 扩展的关键字
- **标识符**: 变量名、函数名、类型名等
- **字面量**: 整数、浮点数、字符串字面量
- **运算符**: 算术、逻辑、比较运算符
- **分隔符**: 括号、逗号、分号等
- **注释**: 单行和多行注释支持

### 源代码位置
- **行列信息**: 精确的行号和列号
- **跨度信息**: 标记的开始和结束位置
- **文件名**: 源文件名信息
- **上下文**: 错误发生时的上下文信息

## 使用示例

### 基本词法分析

```rust
use gaia_types::lexer::{Lexer, Token};

let source = "(module (func (export \"add\") (param i32 i32) (result i32)))";
let mut lexer = Lexer::new(source);

// 获取下一个标记
while let Some(token) = lexer.next_token()? {
    println!("Token: {:?} at {:?}", token.kind(), token.location());
}
```

### 自定义标记类型

```rust
use gaia_types::lexer::{TokenKind, Token};

// 创建自定义标记
let token = Token::new(TokenKind::Keyword("module"), location);
let identifier = Token::new(TokenKind::Identifier("add"), location);
let number = Token::new(TokenKind::Integer(42), location);
```

### 错误处理

```rust
use gaia_types::lexer::LexerError;

match lexer.next_token() {
    Ok(Some(token)) => println!("Got token: {:?}", token),
    Ok(None) => println!("End of input"),
    Err(LexerError::InvalidCharacter(ch, location)) => {
        eprintln!("Invalid character '{}' at {:?}", ch, location);
    }
    Err(e) => eprintln!("Lexer error: {:?}", e),
}
```

## 设计特点

### 性能优化
- **流式处理**: 支持流式输入，无需完整加载源代码
- **零拷贝**: 最小化字符串拷贝操作
- **缓存友好**: 优化的数据结构，提高缓存命中率
- **增量分析**: 支持增量式词法分析

### 错误恢复
- **鲁棒性**: 遇到错误时继续分析，收集多个错误
- **上下文信息**: 提供详细的错误上下文
- **建议修复**: 提供可能的修复建议

### 扩展性
- **自定义关键字**: 支持添加新的关键字
- **扩展标记**: 支持自定义标记类型
- **插件机制**: 支持词法分析插件

## 标记类型详解

### 字面量标记
- **整数**: 支持十进制、十六进制、八进制、二进制
- **浮点数**: 支持标准和小数形式
- **字符串**: 支持转义序列和 Unicode
- **标识符**: Unicode 标识符支持

### 运算符标记
- **算术**: +, -, *, /, % 等
- **逻辑**: &&, ||, ! 等
- **比较**: ==, !=, <, >, <=, >= 等
- **位运算**: &, |, ^, ~, <<, >> 等

## 内存效率

词法分析器设计为内存高效，使用引用和切片来避免不必要的内存分配，同时保持高性能的标记识别速度。