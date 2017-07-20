#![doc = include_str!("readme.md")]

mod abi;
mod api;
mod arch;
pub use self::{abi::AbiCompatible, api::ApiCompatible};

pub use self::arch::Architecture;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
/// 编译目标平台结构体
///
/// 三要素组合系统，用于精确描述编译输出的目标平台特性。
/// 格式为 `build-host-target`，类似于LLVM的target triple概念。
///
/// # 字段说明
///
/// - `build`: 底层的运行时架构（Architecture）
/// - `host`: 二进制格式（AbiCompatible）  
/// - `target`: API接口（ApiCompatible）
///
/// # 示例
///
/// ```rust
/// use gaia_types::helpers::compilation_target::{
///     AbiCompatible, ApiCompatible, Architecture, CompilationTarget,
/// };
///
/// // Linux x86_64目标
/// let linux_target = CompilationTarget {
///     build: Architecture::X86_64,
///     host: AbiCompatible::ELF,
///     target: ApiCompatible::Gnu,
/// };
///
/// // Windows MSVC目标
/// let windows_target = CompilationTarget {
///     build: Architecture::X86_64,
///     host: AbiCompatible::PE,
///     target: ApiCompatible::MicrosoftVisualC,
/// };
///
/// // JVM目标
/// let jvm_target = CompilationTarget {
///     build: Architecture::JVM,
///     host: AbiCompatible::Unknown,
///     target: ApiCompatible::JvmRuntime(8),
/// };
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CompilationTarget {
    /// 底层的运行时架构，例如 x86_64, ARM64 等芯片，或是 JVM，CLR 等虚拟机
    pub build: Architecture,
    /// 使用何种二进制格式，这里包括文本格式
    pub host: AbiCompatible,
    /// 提供何种 API 接口
    pub target: ApiCompatible,
}

impl Display for CompilationTarget {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{}-{}", self.build, self.host, self.target)
    }
}
