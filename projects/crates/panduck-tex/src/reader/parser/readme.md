# Gaia Types - 语法分析器模块

这个模块提供了 Gaia 项目的语法分析功能，负责将词法分析器生成的标记序列转换为抽象语法树（AST）。

## 功能概述

### 语法分析
- **递归下降**: 使用递归下降算法进行语法分析
- **错误恢复**: 强大的错误恢复机制
- **语法验证**: 严格的语法规则验证
- **性能优化**: 高效的语法分析算法

### 抽象语法树
- **AST 节点**: 完整的 AST 节点类型定义
- **位置信息**: 每个节点都包含源代码位置信息
- **语义信息**: 节点包含语义信息和类型信息
- **遍历支持**: 支持 AST 的遍历和转换

### 语法规则支持
- **模块定义**: WebAssembly 模块语法
- **函数定义**: 函数声明、参数、返回值、函数体
- **指令序列**: 完整的指令序列语法
- **类型定义**: 类型声明和使用
- **导入导出**: 模块导入和导出语法
- **内存管理**: 内存段和数据段定义
- **全局变量**: 全局变量定义和初始化

## 使用示例

### 基本语法分析

```rust
use gaia_types::parser::{Parser, SyntaxTree};
use gaia_types::lexer::Lexer;

let source = "(module (func (export \"add\") (param i32 i32) (result i32) local.get 0 local.get 1 i32.add))";
let lexer = Lexer::new(source);
let mut parser = Parser::new(lexer);

// 解析为语法树
let ast = parser.parse()?;
println!("Parsed AST: {:?}", ast);
```

### 错误处理

```rust
use gaia_types::parser::ParseError;

match parser.parse() {
    Ok(ast) => println!("Parse successful: {:?}", ast),
    Err(ParseError::UnexpectedToken(expected, found, location)) => {
        eprintln!("Expected {:?}, found {:?} at {:?}", expected, found, location);
    }
    Err(ParseError::MissingToken(expected, location)) => {
        eprintln!("Missing token {:?} at {:?}", expected, location);
    }
    Err(e) => eprintln!("Parse error: {:?}", e),
}
```

### AST 遍历

```rust
use gaia_types::parser::{AstVisitor, ModuleNode};

struct MyVisitor;
impl AstVisitor for MyVisitor {
    fn visit_module(&mut self, module: &ModuleNode) {
        println!("Visiting module with {} functions", module.functions.len());
    }
}

let mut visitor = MyVisitor;
ast.accept(&mut visitor);
```

## 设计特点

### 性能优化
- **预测分析**: 使用预测分析减少回溯
- **缓存机制**: 缓存常用的语法规则
- **内存池**: 使用内存池减少分配开销
- **增量解析**: 支持增量式语法分析

### 错误恢复
- **同步点**: 使用同步点进行错误恢复
- **错误收集**: 收集多个语法错误
- **上下文信息**: 提供详细的错误上下文
- **修复建议**: 提供可能的修复建议

### 扩展性
- **语法扩展**: 支持语法规则的扩展
- **自定义节点**: 支持自定义 AST 节点类型
- **插件机制**: 支持语法分析插件
- **语义动作**: 支持语义动作的自定义

## AST 节点类型

### 顶层节点
- **Module**: WebAssembly 模块
- **Component**: WebAssembly 组件（提案）
- **Script**: 脚本文件

### 定义节点
- **Function**: 函数定义
- **Global**: 全局变量定义
- **Memory**: 内存定义
- **Table**: 表定义
- **Type**: 类型定义
- **Import**: 导入定义
- **Export**: 导出定义

### 指令节点
- **Block**: 块指令
- **Loop**: 循环指令
- **If**: 条件指令
- **Call**: 函数调用
- **Local**: 局部变量操作
- **Global**: 全局变量操作
- **Memory**: 内存操作
- **Numeric**: 数值操作
- **Parametric**: 参数操作

## 语法验证

### 类型检查
- **类型一致性**: 验证类型的一致性
- **类型推导**: 推导表达式的类型
- **类型转换**: 验证类型转换的合法性

### 作用域检查
- **变量作用域**: 验证变量的作用域
- **名称解析**: 解析名称引用
- **重复定义**: 检测重复定义

### 语义验证
- **指令有效性**: 验证指令的语义有效性
- **控制流**: 验证控制流的正确性
- **资源使用**: 验证资源使用的合法性