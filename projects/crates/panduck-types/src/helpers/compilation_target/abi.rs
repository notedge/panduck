use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// ABI（应用程序二进制接口）兼容枚举
///
/// 表示二进制格式和调用约定，决定了代码如何与操作系统交互。
/// 包括实际的二进制格式和文本格式的中间表示。
/// ABI（应用程序二进制接口）兼容枚举
///
/// 表示二进制格式和调用约定，决定了代码如何与操作系统交互。
/// 包括实际的二进制格式和文本格式的中间表示。
///
/// # 变体说明
///
/// ## 二进制格式
/// - `Unknown`: 最大兼容，虚拟机字节码或者裸机机器码
/// - `ELF`: ELF格式，用于Linux、macOS等类Unix系统
/// - `PE`: PE格式，用于Windows系统
///
/// ## 文本格式
/// - `Jasm`: JVM字节码文本格式（Java Assembly）
/// - `Msil`: CLR字节码文本格式（Microsoft Intermediate Language）
/// - `WAT`: WebAssembly文本格式（WebAssembly Text Format）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AbiCompatible {
    /// 最大兼容性模式，适用于虚拟机字节码或裸机机器码
    ///
    /// 当无法确定具体的二进制格式时使用，提供最大的兼容性。
    /// 常用于通用编译器后端或跨平台工具链。
    Unknown,

    /// ELF (Executable and Linkable Format) 格式
    ///
    /// 主要用于类 Unix 系统，包括：
    /// - Linux 各种发行版
    /// - macOS (Mach-O 格式，但工具链可能使用 ELF 作为中间格式)
    /// - 各种嵌入式 Linux 系统
    /// - BSD 系列系统
    ELF,

    /// PE (Portable Executable) 格式
    ///
    /// 主要用于 Windows 系统，包括：
    /// - Windows PC 桌面系统
    /// - Windows Server 服务器系统
    /// - Windows 10/11 等现代版本
    /// - Windows CE 等嵌入式版本
    PE,

    /// JVM 字节码的文本格式 (Java Assembly)
    ///
    /// 用于 JVM 字节码的人类可读文本表示，常用于：
    /// - 教学和学习 JVM 指令集
    /// - 调试和分析 Java 字节码
    /// - 编译器开发中的中间表示
    JavaAssembly,

    /// CLR 字节码的文本格式 (Microsoft Intermediate Language)
    ///
    /// 用于 .NET 平台字节码的人类可读文本表示，常用于：
    /// - 教学和学习 .NET IL 指令集
    /// - 调试和分析 .NET 程序集
    /// - 编译器开发中的中间表示
    MicrosoftIntermediateLanguage,

    /// WebAssembly 文本格式 (WebAssembly Text Format)
    ///
    /// 用于 WebAssembly 的人类可读文本表示，常用于：
    /// - 教学和学习 WebAssembly 指令集
    /// - 调试和分析 WebAssembly 模块
    /// - 编译器开发中的中间表示
    /// - Web 开发中的手动优化
    WebAssemblyTextFormat,
}

impl Display for AbiCompatible {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AbiCompatible::Unknown => write!(f, "unknown"),
            AbiCompatible::ELF => write!(f, "elf"),
            AbiCompatible::PE => write!(f, "pe"),
            AbiCompatible::JavaAssembly => write!(f, "jasm"),
            AbiCompatible::MicrosoftIntermediateLanguage => write!(f, "msil"),
            AbiCompatible::WebAssemblyTextFormat => write!(f, "wat"),
        }
    }
}
