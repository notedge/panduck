# Gaia Types - 错误处理系统

这个模块提供了 Gaia 项目中完整的错误处理基础设施，包括错误类型定义、诊断信息收集和错误转换等功能。

## 功能概述

### GaiaError
主要的错误类型，包装了具体的错误种类，提供了统一的错误处理接口。

### GaiaErrorKind
错误种类枚举，定义了所有可能的错误类型：
- **InvalidInstruction**: 无效指令错误
- **IoError**: I/O 操作错误
- **SyntaxError**: 语法错误
- **TypeError**: 类型错误
- **ValidationError**: 验证错误
- **LinkError**: 链接错误
- **RuntimeError**: 运行时错误

### GaiaDiagnostics
诊断信息收集器，支持错误恢复和警告收集，提供了与 Rust 的 `?` 操作符的集成。

## 使用示例

### 基本错误处理

```rust
use gaia_types::{GaiaError, Result, SourceLocation};

// 创建语法错误
let location = SourceLocation::default();
let error = GaiaError::syntax_error("缺少分号", location);

// 使用 Result 类型别名
fn parse_source(source: &str) -> Result<()> {
    if source.is_empty() {
        return Err(GaiaError::syntax_error("空源代码", SourceLocation::default()));
    }
    Ok(())
}
```

### 诊断系统使用

```rust
use gaia_types::GaiaDiagnostics;

let mut diagnostics = GaiaDiagnostics::new();

// 收集错误信息
if let Err(error) = parse_source(source) {
    diagnostics.push_error(error);
}

// 获取诊断结果
let result = diagnostics.finish();
```

## 设计特点

- **类型安全**: 强类型的错误处理，避免错误类型的混淆
- **诊断友好**: 详细的错误信息和位置信息
- **可恢复**: 支持错误恢复和继续处理
- **性能优化**: 高效的错误收集和处理机制

## 错误类型详解

### 语法错误 (SyntaxError)
源代码语法不符合规范时触发，包含详细的错误位置和期望信息。

### 类型错误 (TypeError)
类型检查失败时触发，提供类型不匹配的具体信息。

### 验证错误 (ValidationError)
语义验证失败时触发，确保程序的正确性和安全性。

### 链接错误 (LinkError)
模块链接过程中出现的错误，如符号未定义、重复定义等。

### 运行时错误 (RuntimeError)
程序执行过程中出现的错误，如除零、数组越界等。