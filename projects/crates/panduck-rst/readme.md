# Panduck RST - reStructuredText 解析器

Panduck RST 是一个基于 Rust 的 reStructuredText (RST) 解析工具，旨在将 RST 文本转换为其他格式，例如 HTML。

## 特性

- **RST 词法分析器**: 将 RST 文本分解为 token。
- **RST 抽象语法树 (AST)**: 构建 RST 文档的结构化表示。
- **RST 解析器**: 将 token 流转换为 AST。
- **RST 生成器**: 将 AST 转换为目标格式（例如 HTML）。

## 项目结构

```
panduck-rst/
├── src/
│   ├── ast/          # 抽象语法树定义
│   │   └── mod.rs
│   ├── generator/    # 目标格式生成器
│   │   └── mod.rs
│   ├── lexer/        # 词法分析器
│   │   ├── mod.rs
│   │   └── token_type.rs
│   ├── parser/       # 解析器
│   │   └── mod.rs
│   └── lib.rs        # 库入口点
├── tests/            # 测试文件
└── readme.md         # 本文档
```

## 安装

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
panduck-rst = { path = "../panduck-rst" }
```

## 快速开始

```rust
use panduck_rst::{parse_and_generate_rst, RstReadConfig};

fn main() {
    let rst_input = """
.. sectnum::

.. contents::

.. _my-reference-label:

Section Title
-------------

This is a paragraph.

- List item 1
- List item 2

.. code-block:: rust

    fn main() {
        println!("Hello, RST!");
    }

""";

    let config = RstReadConfig { support_math: false };
    let result = parse_and_generate_rst(rst_input, config);

    match result.into_result() {
        Ok(html_output) => {
            println!("Generated HTML:\n{}", html_output);
        }
        Err(diagnostics) => {
            eprintln!("Error parsing RST:");
            for error in diagnostics.diagnostics {
                eprintln!("  - {:?}", error);
            }
        }
    }
}
```

## 许可证

本项目采用 MPL-2.0 许可证。详见 LICENSE 文件。

## 贡献

欢迎提交 Issue 和 Pull Request 来改进这个项目。