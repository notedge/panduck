# Gaia Types - DWARF 调试信息模块

这个模块提供了 WebAssembly DWARF 调试信息的完整支持。

## 功能概述

### DWARF 结构体支持
- **标准兼容**: 遵循 WebAssembly DWARF 规范
- **完整实现**: 支持所有主要的 DWARF 调试信息结构体
- **内存高效**: 优化的数据结构，减少内存占用

### 自定义段处理
- **段解析**: 解析 WebAssembly 文件中的 DWARF 自定义段
- **数据提取**: 从自定义段中提取调试信息
- **格式验证**: 验证 DWARF 数据的完整性和正确性

## 主要组件

### DWARF 结构体
- **调试信息**: 编译单元、类型信息、变量信息
- **行号信息**: 源代码行号映射
- **调用栈**: 调用栈展开信息
- **宏信息**: 宏定义和展开信息

### 自定义段类型
- `.debug_info`: 主要调试信息段
- `.debug_line`: 行号信息段
- `.debug_abbrev`: 缩写信息段
- `.debug_str`: 字符串表段
- `.debug_ranges`: 地址范围段

## 使用示例

```rust
use gaia_types::helpers::dwarf::{DwarfInfo, CustomSection};

// 解析 DWARF 信息
let dwarf_info = DwarfInfo::parse(dwarf_data)?;

// 处理自定义段
let custom_section = CustomSection::from_bytes(section_data)?;
let dwarf_info = custom_section.parse_dwarf()?;

// 获取调试信息
let compile_units = dwarf_info.compile_units();
for unit in compile_units {
    println!("编译单元: {}", unit.name());
}
```

## 参考规范

- [WebAssembly DWARF 规范](https://yurydelendik.github.io/webassembly-dwarf/)
- [DWARF 调试格式标准](https://dwarfstd.org/)
- [WebAssembly 自定义段规范](https://webassembly.github.io/spec/core/appendix/custom.html)

## 设计原则

- **标准遵循**: 严格遵循 DWARF 和 WebAssembly 标准
- **性能优化**: 高效的解析和处理算法
- **内存安全**: 安全的内存管理和错误处理
- **扩展性**: 支持未来的 DWARF 扩展