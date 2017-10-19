#![doc = include_str!("readme.md")]

mod convert;
mod display;

use std::error::Error;
use std::fmt::Debug;

/// Fatal failure in a Panduck reader, writer, or orchestration step.
#[derive(Debug)]
pub enum AdapterError {
    /// File or stream I/O failure.
    Io {
        /// Underlying operating-system I/O error.
        source: std::io::Error,
        /// Optional filesystem path tied to the failure.
        path: Option<String>,
    },
    /// Caller supplied invalid bytes, paths, or parameters.
    InvalidInput {
        /// Human-readable explanation of the invalid input.
        message: String,
    },
    /// Source text slice is out of range.
    InvalidRange {
        /// Byte offset where the invalid slice starts.
        offset: usize,
        /// Requested slice length in bytes.
        len: usize,
    },
    /// Adapter path not implemented yet.
    NotImplemented {
        /// Name of the missing capability.
        feature: String,
    },
    /// Named format adapter failure.
    Adapter {
        /// Adapter identifier that reported the failure.
        adapter: String,
        /// Adapter-specific failure message.
        message: String,
    },
    /// Target format cannot perform the requested operation.
    UnsupportedFormat {
        /// Format name that rejected the operation.
        format: String,
        /// Operation that was requested, such as `read` or `write`.
        operation: String,
    },
    /// Configuration could not be loaded or validated.
    Config {
        /// Optional configuration file path.
        path: Option<String>,
        /// Configuration validation message.
        message: String,
    },
}

impl AdapterError {
    /// I/O failure, optionally tied to a filesystem path.
    pub fn io(source: std::io::Error, path: Option<impl ToString>) -> Self {
        Self::Io {
            source,
            path: path.map(|p| p.to_string()),
        }
    }

    /// Invalid user or adapter input.
    pub fn invalid_input(message: impl ToString) -> Self {
        Self::InvalidInput {
            message: message.to_string(),
        }
    }

    /// Source offset does not fit the backing text.
    pub fn invalid_range(offset: usize, len: usize) -> Self {
        Self::InvalidRange { offset, len }
    }

    /// Missing adapter capability.
    pub fn not_implemented(feature: impl ToString) -> Self {
        Self::NotImplemented {
            feature: feature.to_string(),
        }
    }

    /// Failure inside a named adapter.
    pub fn adapter(adapter: impl ToString, message: impl ToString) -> Self {
        Self::Adapter {
            adapter: adapter.to_string(),
            message: message.to_string(),
        }
    }

    /// Target format does not support the operation.
    pub fn unsupported_format(format: impl ToString, operation: impl ToString) -> Self {
        Self::UnsupportedFormat {
            format: format.to_string(),
            operation: operation.to_string(),
        }
    }

    /// Configuration error.
    pub fn config(path: Option<impl ToString>, message: impl ToString) -> Self {
        Self::Config {
            path: path.map(|p| p.to_string()),
            message: message.to_string(),
        }
    }
}

impl Error for AdapterError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Panduck adapter result type.
pub type Result<T> = std::result::Result<T, AdapterError>;
