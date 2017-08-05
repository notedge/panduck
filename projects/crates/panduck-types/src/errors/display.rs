use super::AdapterError;
use std::fmt::{Display, Formatter};

impl Display for AdapterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AdapterError::Io { source, path } => match path {
                Some(path) => write!(f, "I/O error at {}: {}", path, source),
                None => write!(f, "I/O error: {}", source),
            },
            AdapterError::InvalidInput { message } => write!(f, "invalid input: {}", message),
            AdapterError::InvalidRange { offset, len } => {
                write!(f, "invalid source range: offset {} length {}", offset, len)
            }
            AdapterError::NotImplemented { feature } => write!(f, "not implemented: {}", feature),
            AdapterError::Adapter { adapter, message } => {
                write!(f, "adapter {}: {}", adapter, message)
            }
            AdapterError::UnsupportedFormat { format, operation } => {
                write!(f, "format {} does not support {}", format, operation)
            }
            AdapterError::Config { path, message } => match path {
                Some(path) => write!(f, "config error at {}: {}", path, message),
                None => write!(f, "config error: {}", message),
            },
        }
    }
}
