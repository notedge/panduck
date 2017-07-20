use crate::{PanduckError, PanduckErrorKind};
use tracing::Level;

/// 将GaiaErrorKind转换为GaiaError的实现
///
/// 这个转换会自动将错误种类包装到Box中，
/// 这是GaiaError结构体所要求的，可以减少内存占用
impl From<PanduckErrorKind> for PanduckError {
    fn from(error: PanduckErrorKind) -> Self {
        Self { level: Level::ERROR, kind: Box::new(error) }
    }
}

impl From<std::io::Error> for PanduckError {
    #[track_caller]
    fn from(error: std::io::Error) -> Self {
        PanduckErrorKind::IoError { io_error: error, url: None }.into()
    }
}

impl From<std::fmt::Error> for PanduckError {
    fn from(value: std::fmt::Error) -> Self {
        PanduckError::not_implemented(format!("MSIL 写入失败: {}", value))
    }
}
