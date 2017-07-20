use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// API兼容枚举
///
/// 表示目标平台提供的API接口和运行时环境。
/// 决定了代码可以调用哪些系统API和库函数。
///
/// # 变体说明
///
/// ## 系统API
/// - `Unknown`：未知API，最大兼容性，适用于裸机或通用环境
/// - `Msvc`：Microsoft Visual C++运行时，用于Windows平台
/// - `Gnu`：GNU工具链和glibc，用于Linux等类Unix系统
///
/// ## 运行时版本
/// - `JvmRuntime(u32)`：Java开发工具包版本（如JDK8、JDK11、JDK16）
/// - `ClrRuntime(u16)`：.NET公共语言运行时版本（如.NET 2.0、4.0、5.0）
///
/// ## 专用API
/// - `Unity`：Unity引擎API，用于Unity游戏开发
/// - `WASI`：WebAssembly系统接口，用于WebAssembly运行时
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ApiCompatible {
    /// 未知或通用 API 兼容模式
    ///
    /// 当无法确定具体的 API 兼容性要求时使用，提供最大的灵活性。
    /// 常用于通用编译器后端或跨平台工具链的默认选项。
    Unknown,

    /// GNU C/C++ 运行时库
    ///
    /// 适用于类 Unix 平台的 GCC 编译器工具链，包括：
    /// - glibc (GNU C Library) 提供的标准库
    /// - GCC 特定的编译器扩展
    /// - POSIX 标准 API 接口
    /// - 各种 Linux 发行版的系统调用接口
    Gnu,

    /// Microsoft Visual C++ 运行时库
    ///
    /// 适用于 Windows 平台的 MSVC 编译器工具链，包括：
    /// - Visual Studio 各版本的运行时库
    /// - Windows SDK 提供的 API 接口
    /// - Microsoft 特定的编译器扩展和优化
    MicrosoftVisualC,

    /// Java 虚拟机运行时版本
    ///
    /// 参数 `u32` 表示 JDK 对应的 JVM 字节码版本号：
    ///
    /// - JDK 5 对应字节码版本 49
    /// - JDK 6 对应字节码版本 50
    /// - JDK 7 对应字节码版本 51
    /// - JDK 8 对应字节码版本 52
    /// - JDK 9 对应字节码版本 53
    /// - JDK 10 对应字节码版本 54
    /// - JDK 11 对应字节码版本 55
    /// - JDK 12 对应字节码版本 56
    /// - JDK 13 对应字节码版本 57
    /// - JDK 14 对应字节码版本 58
    /// - JDK 15 对应字节码版本 59
    /// - JDK 16 对应字节码版本 60
    /// - JDK 17 对应字节码版本 61
    /// - JDK 18 对应字节码版本 62
    /// - JDK 19 对应字节码版本 63
    /// - JDK 20 对应字节码版本 64
    /// - JDK 21 对应字节码版本 65
    ///
    /// 用于指定目标 JVM 版本的字节码兼容性和 API 可用性
    JvmRuntime(u32),

    /// .NET Common Language Runtime 版本
    ///
    /// 参数 `u16` 表示 CLR 版本号，例如：
    /// - 2 表示 .NET Framework 2.0/3.0/3.5 的 CLR 版本
    /// - 4 表示 .NET Framework 4.x 的 CLR 版本
    /// - 5 表示 .NET 5.0 的 CLR 版本
    /// - 6 表示 .NET 6.0 的 CLR 版本
    ///
    /// 用于指定目标 .NET 运行时的字节码兼容性和 API 可用性
    ClrRuntime(u16),

    /// Unity 游戏引擎运行时
    ///
    /// 适用于 Unity 游戏引擎的特定 API 兼容性，包括：
    /// - Unity 提供的 Mono/.NET 运行时版本
    /// - Unity 特定的脚本 API 接口
    /// - 跨平台游戏开发的相关库支持
    Unity,

    /// WebAssembly System Interface
    ///
    /// 可能是 wasi_p1 (WASI Preview 1) 或 wasi_p2 (WASI Preview 2)。
    ///
    /// 用于 WebAssembly 模块访问系统资源的标准化接口，包括：
    /// - 文件系统访问
    /// - 网络功能
    /// - 时间功能
    /// - 随机数生成
    /// - 环境变量访问
    ///
    /// 允许 WebAssembly 代码在不同环境中安全地运行
    WASI,
}

impl Display for ApiCompatible {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiCompatible::Unknown => f.write_str("unknown"),
            ApiCompatible::Gnu => f.write_str("gnu"),
            ApiCompatible::MicrosoftVisualC => f.write_str("msvc"),
            ApiCompatible::JvmRuntime(version) => write!(f, "jvm{}", version),
            ApiCompatible::ClrRuntime(version) => write!(f, "clr{}", version),
            ApiCompatible::Unity => f.write_str("unity"),
            ApiCompatible::WASI => f.write_str("wasi"),
        }
    }
}
