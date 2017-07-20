# Gaia Types - 统一类型系统

Gaia Types 是 Gaia 项目中的核心类型定义库，提供了跨平台汇编器所需的统一类型系统、错误处理和序列化功能。

## 特性

- **统一类型系统**: 定义跨平台的统一指令集、类型系统和程序结构
- **错误处理**: 提供完整的错误处理机制，支持诊断信息收集和错误恢复
- **序列化支持**: 支持 JSON 和二进制序列化，便于数据交换和持久化
- **跨平台兼容**: 支持多种目标架构（x86/x64, ARM, .NET IL, JVM, WASI）

## 项目结构

```
gaia-types/
├── src/
│   ├── errors/          # 错误处理系统
│   │   ├── mod.rs       # 错误类型定义和主要接口
│   │   ├── diagnostics.rs # 诊断信息收集器
│   │   ├── display.rs   # 错误显示实现
│   │   └── convert.rs   # 错误转换实现
│   ├── helpers/         # 辅助类型和工具
│   ├── reader/          # 二进制读取器
│   ├── writer/          # 二进制和文本写入器
│   ├── lexer/           # 词法分析器
│   └── lib.rs           # 库入口点
├── tests/               # 测试文件
└── readme.md           # 本文档
```

## 安装

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
gaia-types = { path = "../gaia-types" }
```

## 快速开始

### 基本类型使用

```rust
use gaia_types::{GaiaProgram, GaiaFunction, GaiaInstruction, GaiaConstant, GaiaType};

// 创建一个简单的程序
let program = GaiaProgram {
name: "hello_world".to_string(),
functions: vec![GaiaFunction {
    name: "main".to_string(),
    parameters: vec![],
    return_type: Some(GaiaType::Int32),
    locals: vec![],
    instructions: vec![
        GaiaInstruction::LoadConstant(GaiaConstant::Int32(42)),
        GaiaInstruction::Return,
    ],
}],
constants: vec![],
};

// 序列化为 JSON
let json = serde_json::to_string( & program).unwrap();
println!("程序 JSON: {}", json);
```

### 错误处理

```rust
use gaia_types::{GaiaError, SourceLocation, Result};

// 创建语法错误
fn parse_source(source: &str) -> Result<()> {
    if source.is_empty() {
        let location = SourceLocation::default();
        return Err(GaiaError::syntax_error("空源代码", location));
    }
    Ok(())
}

// 使用诊断信息收集器
use gaia_types::GaiaDiagnostics;

fn compile_with_diagnostics(program: &GaiaProgram) -> GaiaDiagnostics<Vec<u8>> {
    let mut diagnostics = GaiaDiagnostics::success(vec![]);

    // 添加警告信息
    diagnostics.add_warning(GaiaError::custom_error("优化建议: 可以简化指令序列"));

    diagnostics
}
```

## API 参考

### 主要类型

#### GaiaProgram

表示完整的程序结构，包含函数列表和全局常量。

```rust
pub struct GaiaProgram {
    pub name: String,
    pub functions: Vec<GaiaFunction>,
    pub constants: Vec<(String, GaiaConstant)>,
}
```

#### GaiaFunction

表示函数定义，包含参数、返回类型、局部变量和指令序列。

```rust
pub struct GaiaFunction {
    pub name: String,
    pub parameters: Vec<GaiaType>,
    pub return_type: Option<GaiaType>,
    pub locals: Vec<GaiaType>,
    pub instructions: Vec<GaiaInstruction>,
}
```

#### GaiaInstruction

统一指令集枚举，支持多种操作类型：

- **栈操作**: `LoadConstant`, `LoadLocal`, `StoreLocal`, `Duplicate`, `Pop`
- **算术运算**: `Add`, `Subtract`, `Multiply`, `Divide`, `Remainder`
- **比较指令**: `CompareEqual`, `CompareLessThan`, `CompareGreaterThan`
- **控制流**: `Branch`, `BranchIfTrue`, `BranchIfFalse`, `Call`, `Return`
- **内存操作**: `LoadAddress`, `LoadIndirect`, `StoreIndirect`
- **类型转换**: `Convert`, `Box`, `Unbox`

#### GaiaType

类型系统枚举，支持基本类型和复合类型：

- **基本类型**: `Int32`, `Int64`, `Float32`, `Float64`, `String`, `Boolean`
- **引用类型**: `Object`, `Pointer`
- **复合类型**: `Array(Box<GaiaType>)`, `Custom(String)`

### 错误处理系统

#### GaiaError

主要的错误类型，包装具体的错误种类。

```rust
pub struct GaiaError {
    level: Level,
    kind: Box<GaiaErrorKind>,
}
```

#### GaiaErrorKind

错误种类枚举，定义所有可能的错误类型：

- `InvalidInstruction` - 无效指令错误
- `UnsupportedArchitecture` - 不支持的架构错误
- `InvalidRange` - 无效范围错误
- `IoError` - IO 错误
- `SyntaxError` - 语法错误
- `NotImplemented` - 功能未实现错误
- `CustomError` - 自定义错误

#### GaiaDiagnostics

诊断信息收集器，支持错误恢复和警告收集。

```rust
pub struct GaiaDiagnostics<T> {
    pub result: Result<T, GaiaError>,
    pub diagnostics: Vec<GaiaError>,
}
```

### 辅助功能

#### 序列化

所有主要类型都实现了 `serde` 的 `Serialize` 和 `Deserialize` trait，支持 JSON 和二进制序列化。

#### 二进制读写

提供 `BinaryReader` 和 `BinaryWriter` 用于处理二进制数据。

## 开发指南

### 添加新的指令

1. 在 `GaiaInstruction` 枚举中添加新的指令变体
2. 为指令实现必要的 trait（Debug, Clone, PartialEq, Serialize, Deserialize）
3. 更新相关的后端编译器以支持新指令

### 扩展错误类型

1. 在 `GaiaErrorKind` 枚举中添加新的错误变体
2. 在 `GaiaError` 结构体中添加对应的构造函数
3. 在 `display.rs` 中实现错误显示逻辑

### 运行测试

```bash
cd gaia-types
cargo test
```

## 许可证

本项目采用 MIT 许可证。详见 LICENSE 文件。

## 贡献

欢迎提交 Issue 和 Pull Request 来改进这个项目。