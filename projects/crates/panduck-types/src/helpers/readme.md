# Gaia Types - 辅助工具模块

这个模块提供了 Gaia 项目中使用的各种辅助工具和数据结构。

## 模块结构

```
helpers/
├── compilation_target/    # 编译目标相关类型
│   ├── abi.rs            # 应用二进制接口
│   ├── api.rs            # 应用程序接口
│   ├── arch.rs           # 架构定义
│   └── mod.rs            # 模块主文件
└── dwarf/                # DWARF 调试信息
    ├── custom_sections.rs # 自定义段处理
    └── mod.rs            # DWARF 主模块
```

## 主要功能

### 编译目标 (compilation_target)

- **架构支持**: 定义支持的处理器架构
- **ABI 接口**: 应用二进制接口规范
- **API 接口**: 应用程序接口定义
- **目标平台**: 编译目标平台信息

### DWARF 调试信息 (dwarf)

- **标准兼容**: 遵循 WebAssembly DWARF 规范
- **自定义段**: 处理调试信息自定义段
- **调试支持**: 提供完整的调试信息支持

## 使用示例

### 编译目标使用

```rust
use gaia_types::helpers::CompilationTarget;

let target = CompilationTarget::new()
    .architecture("wasm32")
    .abi("wasm")
    .build()?;
```

### DWARF 调试信息

```rust
use gaia_types::helpers::dwarf::DwarfInfo;

let dwarf_info = DwarfInfo::from_custom_section(section_data)?;
```

## 设计原则

- **类型安全**: 所有辅助类型都经过严格验证
- **性能优化**: 高效的数据结构和算法
- **标准兼容**: 遵循相关标准和规范
- **扩展性**: 支持未来的扩展需求