# Panduck Core - Unified Type System

Panduck Core is the core type definition library within the Panduck project, providing a unified type system, error handling, and serialization capabilities required for cross-platform assemblers.

## Features

- **Unified Type System**: Defines a unified instruction set, type system, and program structure across platforms.
- **Error Handling**: Provides a comprehensive error handling mechanism, supporting diagnostic information collection and error recovery.
- **Serialization Support**: Supports JSON and binary serialization for easy data exchange and persistence.
- **Cross-Platform Compatibility**: Supports multiple target architectures (x86/x64, ARM, .NET IL, JVM, WASI).

## Project Structure

```
panduck-core/
├── src/
│   ├── errors/          # Error handling system
│   │   ├── mod.rs       # Error type definitions and main interfaces
│   │   ├── diagnostics.rs # Diagnostic information collector
│   │   ├── display.rs   # Error display implementation
│   │   └── convert.rs   # Error conversion implementation
│   ├── helpers/         # Helper types and utilities
│   ├── reader/          # Binary reader
│   ├── writer/          # Binary and text writer
│   ├── lexer/           # Lexical analyzer
│   └── lib.rs           # Library entry point
├── tests/               # Test files
└── readme.md           # This document
```

## Installation

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
panduck-core = { path = "../panduck-core" }
```

## Quick Start

### Basic Type Usage

```rust
use panduck_core::{PanduckProgram, PanduckFunction, PanduckInstruction, PanduckConstant, PanduckType};

// Create a simple program
let program = PanduckProgram {
    name: "hello_world".to_string(),
    functions: vec![PanduckFunction {
        name: "main".to_string(),
        parameters: vec![],
        return_type: Some(PanduckType::Int32),
        locals: vec![],
        instructions: vec![
            PanduckInstruction::LoadConstant(PanduckConstant::Int32(42)),
            PanduckInstruction::Return,
        ],
    }],
    constants: vec![],
};

// Serialize to JSON
let json = serde_json::to_string(&program).unwrap();
println!("Program JSON: {}", json);
```

### Error Handling

```rust
use panduck_core::{PanduckError, SourceLocation, Result};

// Create a syntax error
fn parse_source(source: &str) -> Result<()> {
    if source.is_empty() {
        let location = SourceLocation::default();
        return Err(PanduckError::syntax_error("Empty source code", location));
    }
    Ok(())
}

// Use the diagnostic information collector
use panduck_core::PanduckDiagnostics;

fn compile_with_diagnostics(program: &PanduckProgram) -> PanduckDiagnostics<Vec<u8>> {
    let mut diagnostics = PanduckDiagnostics::success(vec![]);

    // Add a warning message
    diagnostics.add_warning(PanduckError::custom_error("Optimization suggestion: instruction sequence can be simplified"));

    diagnostics
}
```

## API Reference

### Main Types

#### PanduckProgram

Represents the complete program structure, including a list of functions and global constants.

```rust
pub struct PanduckProgram {
    pub name: String,
    pub functions: Vec<PanduckFunction>,
    pub constants: Vec<(String, PanduckConstant)>,
}
```

#### PanduckFunction

Represents a function definition, including parameters, return type, local variables, and instruction sequence.

```rust
pub struct PanduckFunction {
    pub name: String,
    pub parameters: Vec<PanduckType>,
    pub return_type: Option<PanduckType>,
    pub locals: Vec<PanduckType>,
    pub instructions: Vec<PanduckInstruction>,
}
```

#### PanduckInstruction

Unified instruction set enumeration, supporting various operation types:

- **Stack Operations**: `LoadConstant`, `LoadLocal`, `StoreLocal`, `Duplicate`, `Pop`
- **Arithmetic Operations**: `Add`, `Subtract`, `Multiply`, `Divide`, `Remainder`
- **Comparison Instructions**: `CompareEqual`, `CompareLessThan`, `CompareGreaterThan`
- **Control Flow**: `Branch`, `BranchIfTrue`, `BranchIfFalse`, `Call`, `Return`
- **Memory Operations**: `LoadAddress`, `LoadIndirect`, `StoreIndirect`
- **Type Conversion**: `Convert`, `Box`, `Unbox`

#### PanduckType

Type system enumeration, supporting basic and composite types:

- **Basic Types**: `Int32`, `Int64`, `Float32`, `Float64`, `String`, `Boolean`
- **Reference Types**: `Object`, `Pointer`
- **Composite Types**: `Array(Box<PanduckType>)`, `Custom(String)`

### Error Handling System

#### PanduckError

The main error type, wrapping specific error kinds.

```rust
pub struct PanduckError {
    level: Level,
    kind: Box<PanduckErrorKind>,
}
```

#### PanduckErrorKind

Error kind enumeration, defining all possible error types:

- `InvalidInstruction` - Invalid instruction error
- `UnsupportedArchitecture` - Unsupported architecture error
- `InvalidRange` - Invalid range error
- `IoError` - IO error
- `SyntaxError` - Syntax error
- `NotImplemented` - Feature not implemented error
- `CustomError` - Custom error

#### PanduckDiagnostics

Diagnostic information collector, supporting error recovery and warning collection.

```rust
pub struct PanduckDiagnostics<T> {
    pub result: Result<T, PanduckError>,
    pub diagnostics: Vec<PanduckError>,
}
```

### Helper Functions

#### Serialization

All main types implement `serde`'s `Serialize` and `Deserialize` traits, supporting JSON and binary serialization.

#### Binary Read/Write

Provides `BinaryReader` and `BinaryWriter` for handling binary data.

## Development Guide

### Adding New Instructions

1. Add a new instruction variant to the `PanduckInstruction` enum.
2. Implement necessary traits (Debug, Clone, PartialEq, Serialize, Deserialize) for the instruction.
3. Update relevant backend compilers to support the new instruction.

### Extending Error Types

1. Add a new error variant to the `PanduckErrorKind` enum.
2. Add a corresponding constructor to the `PanduckError` struct.
3. Implement error display logic in `display.rs`.

### Running Tests

```bash
cd panduck-core
cargo test
```

## License

This project is licensed under the MIT License. See the LICENSE file for details.

## Contributions

Contributions via Issues and Pull Requests are welcome to improve this project.