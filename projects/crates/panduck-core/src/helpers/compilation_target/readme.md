# 编译目标平台定义模块

本模块定义了编译器目标平台的三要素组合系统，类似于LLVM的target triple概念。用于精确描述编译输出的目标平台特性。

## 三要素架构

格式为：`build-host-target`

- **build**: 底层的运行时架构（Architecture）
- **host**: 二进制格式（AbiCompatible）  
- **target**: API接口（ApiCompatible）

## 示例组合

### 传统平台
```
x86-linux-gnu           // 32位x86架构，Linux系统，GNU工具链
x86_64-windows_pc-msvc  // 64位x86架构，Windows系统，MSVC工具链
x86_64-windows_pc-gnu   // 64位x86架构，Windows系统，GNU工具链
arm64-linux-gnu         // 64位ARM架构，Linux系统，GNU工具链
```

### 虚拟机平台
```
jvm-unknown-unknown     // JVM架构，未知ABI，通用API
jvm-unknown-jdk8        // JVM架构，未知ABI，JDK8 API
jvm-jasm-jdk8           // JVM架构，JASM文本格式，JDK8 API
clr-unknown-net2_0      // CLR架构，未知ABI，.NET 2.0 API
clr-msil-net4_0         // CLR架构，MSIL文本格式，.NET 4.0 API
```

### WebAssembly平台
```
wasm32-unknown-unknown  // 32位WebAssembly，未知ABI，通用API
wasm32-wat-unknown      // 32位WebAssembly，WAT文本格式，通用API
wasm32-wat-wasi_p2gnu   // 32位WebAssembly，WAT文本格式，WASI P2 GNU接口
```

## 架构类型 (Architecture)

### 物理架构
- **X86**: 32位x86架构
- **X86_64**: 64位x86架构  
- **ARM32**: 32位ARM架构
- **ARM64**: 64位ARM/AArch64架构
- **RISCV32**: 32位RISC-V架构
- **RISCV64**: 64位RISC-V架构
- **MIPS32**: 32位MIPS架构
- **MIPS64**: 64位MIPS架构
- **WASM32**: 32位WebAssembly
- **WASM64**: 64位WebAssembly

### 虚拟机架构
- **JVM**: Java虚拟机
- **CLR**: .NET公共语言运行时
- **Other(String)**: 自定义架构名称

## ABI兼容 (AbiCompatible)

表示二进制接口格式：

- **Unknown**: 最大兼容，虚拟机字节码或裸机机器码
- **ELF**: ELF格式（Linux、macOS等）
- **PE**: PE格式（Windows）
- **Jasm**: JVM字节码文本格式
- **Msil**: CLR字节码文本格式
- **WAT**: WebAssembly文本格式

## API兼容 (ApiCompatible)

表示目标平台的API接口：

- **Unknown**: 未知API，最大兼容性
- **Msvc**: Microsoft Visual C++运行时
- **Gnu**: GNU工具链和glibc
- **JDK(u16)**: Java开发工具包版本（如JDK8、JDK11）
- **CLR(u16)**: .NET公共语言运行时版本
- **Unity**: Unity引擎API
- **WASI**: WebAssembly系统接口

## 实现功能

### Architecture 方法

```rust
/// 从 COFF 机器类型创建架构
pub fn from_machine_type(machine: u16) -> Self
```

支持的COFF机器类型：
- `0x014c`: IMAGE_FILE_MACHINE_I386 → X86
- `0x8664`: IMAGE_FILE_MACHINE_AMD64 → X86_64  
- `0x01c0`: IMAGE_FILE_MACHINE_ARM → ARM32
- `0xaa64`: IMAGE_FILE_MACHINE_ARM64 → ARM64
- `0x0166`: IMAGE_FILE_MACHINE_R4000 → MIPS32
- 其他：转换为 Other("machine_XXXX") 格式

### Display 实现

Architecture 实现了 Display trait，输出格式：
- X86 → "x86"
- X86_64 → "x64"
- ARM32 → "arm"
- ARM64 → "arm64"
- RISCV32 → "riscv32"
- RISCV64 → "riscv64"
- MIPS32 → "mips"
- MIPS64 → "mips64"
- WASM32 → "wasm32"
- WASM64 → "wasm64"
- JVM → "jvm"
- CLR → "clr"
- Other(name) → name

## 使用示例

```rust
use gaia_types::helpers::compilation_target::{CompilationTarget, Architecture, AbiCompatible, ApiCompatible};

// 创建Linux x86_64目标
let linux_target = CompilationTarget {
    build: Architecture::X86_64,
    host: AbiCompatible::ELF,
    target: ApiCompatible::Gnu,
};

// 创建Windows MSVC目标
let windows_target = CompilationTarget {
    build: Architecture::X86_64,
    host: AbiCompatible::PE,
    target: ApiCompatible::Msvc,
};

// 创建JVM目标
let jvm_target = CompilationTarget {
    build: Architecture::JVM,
    host: AbiCompatible::Unknown,
    target: ApiCompatible::JDK(8),
};

// 从COFF机器类型创建架构
let arch = Architecture::from_machine_type(0x8664); // X86_64
println!("Architecture: {}", arch); // 输出: Architecture: x64
```

## 设计特点

1. **三要素分离**: 清晰地区分了架构、ABI和API三个层面
2. **序列化支持**: 使用`serde`支持JSON等格式的序列化/反序列化
3. **扩展性**: 通过`Other`变体支持自定义架构
4. **版本支持**: JDK和CLR支持版本号参数
5. **多平台覆盖**: 支持物理架构、虚拟机和WebAssembly
6. **实用性**: 提供从COFF机器类型转换的实用函数

这个设计为编译器提供了灵活的target描述机制，可以精确指定编译输出的目标平台特性。