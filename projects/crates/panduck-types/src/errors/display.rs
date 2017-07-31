use super::*;

/// 为GaiaError实现标准库Error trait
///
/// 这使得GaiaError可以作为标准错误类型使用
impl Error for PanduckError {}

/// 为GaiaError实现Debug trait
///
/// 使用Debug格式输出时，会委托给内部的GaiaErrorKind
impl Debug for PanduckError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.kind, f)
    }
}

/// 为GaiaError实现Display trait
///
/// 使用Display格式输出时，会委托给内部的GaiaErrorKind
impl Display for PanduckError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.kind, f)
    }
}

/// 为GaiaErrorKind实现Display trait
///
/// 提供用户友好的错误信息输出格式
impl Display for PanduckErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PanduckErrorKind::InvalidInstruction { instruction, architecture } => {
                write!(f, "无效指令 '{}' 在架构 '{}'", instruction, architecture)?;
            }
            PanduckErrorKind::UnsupportedArchitecture { architecture } => {
                write!(f, "不支持的架构: {}", architecture)?;
            }
            PanduckErrorKind::InvalidRange { length, expect } => {
                write!(f, "无效范围: 实际长度 {}，期望长度 {}", length, expect)?;
            }
            PanduckErrorKind::SyntaxError { message, location } => {
                write!(f, "语法错误在 {:?}: {}", location.url, message)?;
            }
            PanduckErrorKind::IoError { io_error, url } => {
                if let Some(url) = url {
                    write!(f, "IO错误在 {}: {}", url, io_error)?;
                }
                else {
                    write!(f, "IO错误: {}", io_error)?;
                }
            }
            PanduckErrorKind::StageError { location } => {
                write!(f, "阶段错误在 {:?}", location)?;
            }
            PanduckErrorKind::NotImplemented { feature } => {
                write!(f, "功能未实现: {}", feature)?;
            }
            PanduckErrorKind::CustomError { message } => {
                write!(f, "自定义错误: {}", message)?;
            }
            PanduckErrorKind::AdapterError { adapter_name, message, source } => {
                write!(f, "适配器错误 [{}]: {}", adapter_name, message)?;
                if let Some(source) = source {
                    write!(f, " (源错误: {})", source)?;
                }
            }
            PanduckErrorKind::PlatformUnsupported { platform, operation } => {
                write!(f, "平台 '{}' 不支持操作: {}", platform, operation)?;
            }
            PanduckErrorKind::ConfigError { config_path, message } => {
                if let Some(path) = config_path {
                    write!(f, "配置错误在 '{}': {}", path, message)?;
                }
                else {
                    write!(f, "配置错误: {}", message)?;
                }
            }
            PanduckErrorKind::UnsupportedTarget { target } => {
                write!(f, "不支持的编译目标: {:?}", target)?;
            }
            PanduckErrorKind::CompilationFailed { target, message } => {
                write!(f, "编译失败 (目标: {:?}): {}", target, message)?;
            }
        }
        Ok(())
    }
}
