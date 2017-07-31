//! This module defines the error types and diagnostic mechanisms for the Panduck project.
//! It provides a structured way to handle various errors that can occur during compilation,
//! parsing, and other operations, including syntax errors, I/O errors, and unsupported features.
//! 
//! The main error type is [PanduckError], which wraps specific error kinds defined in [PanduckErrorKind].
//! Diagnostic information is collected using [PanduckDiagnostics], allowing for comprehensive
//! error reporting and recovery.

pub use self::diagnostics::PanduckDiagnostics;
use crate::helpers::{Architecture, CompilationTarget, SourceLocation};
use std::{
    error::Error,
    fmt::{Debug, Display, Formatter},
    panic::Location,
};
use tracing::Level;
use url::Url;

mod convert;
mod diagnostics;
mod display;

/// Result type for this crate, using PanduckError as the error type.
///
/// This type alias simplifies error handling; all functions that might return errors should use this type.
pub type Result<T> = std::result::Result<T, PanduckError>;

/// Panduck error type, wrapping specific error kinds [PanduckErrorKind].
///
/// Uses Box to reduce the enum size and improve performance.
pub struct PanduckError {
    level: Level,
    /// The specific error kind, wrapped in a Box to reduce memory footprint.
    ///
    /// This field contains the actual error information, stored indirectly via a Box pointer,
    /// which avoids allocating larger enum values on the stack and improves performance.
    kind: Box<PanduckErrorKind>,
}

/// Panduck error kind enumeration, defining all possible error types.
#[derive(Debug)]
pub enum PanduckErrorKind {
    InvalidInstruction {
        instruction: String,
        architecture: Architecture,
    },
    UnsupportedArchitecture {
        architecture: Architecture,
    },
    /// Invalid range error, used when the actual length does not match the expected length.
    ///
    /// This error typically occurs when parsing binary data or validating data structures,
    /// when the actual data length does not match the expected length.
    InvalidRange {
        /// Actual length.
        ///
        /// Represents the actual measured or parsed data length.
        length: usize,
        /// Expected length.
        ///
        /// Represents the length expected according to specifications or expectations.
        expect: usize,
    },
    /// I/O error, containing the underlying I/O error and optional URL information.
    ///
    /// Used when file read/write, network requests, or other I/O operations fail.
    IoError {
        /// The underlying I/O error.
        ///
        /// Contains specific I/O error information, such as file not found, insufficient permissions, etc.
        io_error: std::io::Error,
        /// Optional URL related to the I/O operation.
        ///
        /// If the I/O operation is related to a specific file or network resource, its URL is stored here.
        /// Can be a file system path or a network address.
        url: Option<Url>,
    },
    /// Syntax error, containing the error message and source code location information.
    ///
    /// Used when syntax issues are found during source code parsing, providing detailed error location information.
    SyntaxError {
        /// Error message, describing the specific syntax issue.
        ///
        /// Contains a human-readable description of the syntax error, such as "missing semicolon", "unclosed parenthesis", etc.
        message: String,
        /// Source code location information where the error occurred.
        ///
        /// Contains location information such as the file, line number, column number, etc.,
        /// helping developers quickly locate the problem.
        location: SourceLocation,
    },
    /// Stage error, indicating a stop in execution at a specific location.
    StageError {
        /// The location where the execution stopped.
        location: Location<'static>,
    },
    /// Feature not implemented error.
    ///
    /// Used when calling a feature that has not yet been implemented.
    NotImplemented {
        /// Description of the unimplemented feature.
        feature: String,
    },
    /// Custom error, containing a custom error message.
    ///
    /// Used when needing to represent specific business logic errors or other non-standard errors.
    CustomError {
        /// Custom error message.
        message: String,
    },
    /// Adapter error, used when an adapter operation fails.
    ///
    /// Contains the adapter name and specific error information.
    AdapterError {
        /// Adapter name.
        adapter_name: String,
        /// Error message.
        message: String,
        /// Optional source error.
        source: Option<Box<PanduckError>>,
    },
    /// Platform unsupported error, used when a target platform does not support an operation.
    ///
    /// Contains the platform name and a description of the unsupported operation.
    PlatformUnsupported {
        /// Platform name.
        platform: String,
        /// Description of the unsupported operation.
        operation: String,
    },
    /// Configuration error, used when configuration file parsing or validation fails.
    ///
    /// Contains the configuration file path and error message.
    ConfigError {
        /// Optional configuration file path.
        config_path: Option<String>,
        /// Error message.
        message: String,
    },
    /// Unsupported compilation target error.
    ///
    /// Used when attempting to compile to an unsupported target platform.
    UnsupportedTarget {
        /// The unsupported compilation target.
        target: CompilationTarget,
    },
    /// Compilation failed error.
    ///
    /// Used when an error occurs during the compilation process.
    CompilationFailed {
        /// The compilation target.
        target: CompilationTarget,
        /// Error message.
        message: String,
    },
}

impl PanduckError {
    /// Creates a syntax error.
    ///
    /// Use this function to create an error when a syntax issue is found during source code parsing.
    ///
    /// # Arguments
    ///
    /// * `message` - The error message, describing the specific syntax issue.
    /// * `location` - The source code location information where the error occurred.
    ///
    /// # Returns
    ///
    /// Returns a [PanduckError] instance containing syntax error information.
    ///
    /// # Examples
    ///
    /// ```
    /// use panduck_types::errors::{PanduckError, SourceLocation};
    /// let location = SourceLocation::default();
    /// let error = PanduckError::syntax_error("Missing semicolon", location);
    /// ```
    pub fn syntax_error(message: impl ToString, location: SourceLocation) -> Self {
        PanduckErrorKind::SyntaxError {
            message: message.to_string(),
            location,
        }
        .into()
    }

    /// Creates an I/O error.
    ///
    /// Use this function to create an error when file read/write, network requests, or other I/O operations fail.
    ///
    /// # Arguments
    ///
    /// * `io_error` - The underlying I/O error.
    /// * `url` - The URL related to the I/O operation (e.g., file path or network address).
    ///
    /// # Returns
    ///
    /// Returns a [PanduckError] instance containing I/O error information.
    ///
    /// # Examples
    ///
    /// ```
    /// use panduck_types::errors::PanduckError;
    /// use url::Url;
    /// let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
    /// let url = Url::from_file_path("/path/to/file")
    ///     .ok()
    ///     .and_then(|x| Some(x))
    ///     .unwrap_or_else(|| Url::parse("file:///path/to/file").unwrap());
    /// let error = PanduckError::io_error(io_err, url);
    /// ```
    pub fn io_error(io_error: std::io::Error, url: Url) -> Self {
        PanduckErrorKind::IoError {
            io_error,
            url: Some(url),
        }
        .into()
    }

    /// Creates an invalid instruction error.
    ///
    /// Use this function to create an error when an unknown or unsupported instruction is parsed.
    ///
    /// # Arguments
    ///
    /// * `instruction` - The invalid instruction string.
    /// * `architecture` - The architecture to which the instruction belongs.
    ///
    /// # Returns
    ///
    /// Returns a [PanduckError] instance containing invalid instruction error information.
    ///
    /// # Examples
    ///
    /// ```
    /// use panduck_types::errors::{helpers::Architecture, PanduckError};
    /// let error = PanduckError::invalid_instruction("Unknown instruction", Architecture::X86);
    /// ```
    pub fn invalid_instruction(instruction: impl ToString, architecture: Architecture) -> Self {
        PanduckErrorKind::InvalidInstruction {
            instruction: instruction.to_string(),
            architecture,
        }
        .into()
    }

    /// Creates an unsupported architecture error.
    ///
    /// Use this function to create an error when attempting to perform an operation on an unsupported architecture.
    ///
    /// # Arguments
    ///
    /// * `architecture` - The unsupported architecture.
    ///
    /// # Returns
    ///
    /// Returns a [PanduckError] instance containing unsupported architecture error information.
    ///
    /// # Examples
    ///
    /// ```
    /// use panduck_types::errors::{helpers::Architecture, PanduckError};
    /// let error = PanduckError::unsupported_architecture(Architecture::ARM32);
    /// ```
    pub fn unsupported_architecture(architecture: Architecture) -> Self {
        PanduckErrorKind::UnsupportedArchitecture { architecture }.into()
    }

    /// Creates an invalid range error.
    ///
    /// Use this function to create an error when the actual data length does not match the expected length.
    ///
    /// # Arguments
    ///
    /// * `length` - The actual length.
    /// * `expect` - The expected length.
    ///
    /// # Returns
    ///
    /// Returns a [PanduckError] instance containing invalid range error information.
    ///
    /// # Examples
    ///
    /// ```
    /// use panduck_types::errors::PanduckError;
    /// let error = PanduckError::invalid_range(1024, 2048);
    /// ```
    pub fn invalid_range(length: usize, expect: usize) -> Self {
        PanduckErrorKind::InvalidRange { length, expect }.into()
    }

    pub fn invalid_data(data: &str) -> Self {
        Self {
            level: Level::ERROR,
            kind: Box::new(PanduckErrorKind::CustomError {
                message: format!("Invalid data: {}", data),
            }),
        }
    }

    pub fn kind(&self) -> &PanduckErrorKind {
        &self.kind
    }

    pub fn level(&self) -> &Level {
        &self.level
    }

    /// Creates a feature not implemented error.
    ///
    /// Use this function to create an error when calling a feature that has not yet been implemented.
    ///
    /// # Arguments
    ///
    /// * `feature` - Description of the unimplemented feature.
    ///
    /// # Returns
    ///
    /// Returns a [PanduckError] instance containing feature not implemented error information.
    ///
    /// # Examples
    ///
    /// ```
    /// use panduck_types::errors::PanduckError;
    /// let error = PanduckError::not_implemented("PE context creation");
    /// ```
    pub fn not_implemented(feature: impl ToString) -> Self {
        PanduckErrorKind::NotImplemented {
            feature: feature.to_string(),
        }
        .into()
    }

    /// Creates an adapter error.
    ///
    /// # Arguments
    /// * `adapter_name` - The name of the adapter.
    /// * `message` - The error message.
    /// * `source` - Optional source error.
    ///
    /// # Examples
    /// ```
    /// use panduck_types::errors::PanduckError;
    /// let error = PanduckError::adapter_error("PeExportAdapter", "Export failed", None);
    /// ```
    pub fn adapter_error(
        adapter_name: impl ToString,
        message: impl ToString,
        source: Option<Box<PanduckError>>,
    ) -> Self {
        PanduckErrorKind::AdapterError {
            adapter_name: adapter_name.to_string(),
            message: message.to_string(),
            source,
        }
        .into()
    }

    /// Creates a platform unsupported error.
    ///
    /// # Arguments
    /// * `platform` - The name of the platform.
    /// * `operation` - Description of the unsupported operation.
    ///
    /// # Examples
    /// ```
    /// use panduck_types::errors::PanduckError;
    /// let error = PanduckError::platform_unsupported("WASI", "Inline assembly");
    /// ```
    pub fn platform_unsupported(platform: impl ToString, operation: impl ToString) -> Self {
        PanduckErrorKind::PlatformUnsupported {
            platform: platform.to_string(),
            operation: operation.to_string(),
        }
        .into()
    }

    /// Creates a configuration error.
    ///
    /// # Arguments
    /// * `config_path` - Optional path to the configuration file.
    /// * `message` - The error message.
    ///
    /// # Examples
    /// ```
    /// use panduck_types::errors::PanduckError;
    /// let error = PanduckError::config_error(Some("config.toml"), "Configuration file format error");
    /// ```
    pub fn config_error(config_path: Option<impl ToString>, message: impl ToString) -> Self {
        PanduckErrorKind::ConfigError {
            config_path: config_path.map(|p| p.to_string()),
            message: message.to_string(),
        }
        .into()
    }

    /// Creates an unsupported compilation target error.
    ///
    /// # Arguments
    /// * `target` - The unsupported compilation target.
    ///
    /// # Examples
    /// ```
    /// use panduck_types::errors::{CompilationTarget, PanduckError};
    /// let target = CompilationTarget::default();
    /// let error = PanduckError::unsupported_target(target);
    /// ```
    pub fn unsupported_target(target: CompilationTarget) -> Self {
        PanduckErrorKind::UnsupportedTarget { target }.into()
    }

    /// Creates a compilation failed error.
    ///
    /// # Arguments
    /// * `target` - The compilation target.
    /// * `message` - The error message.
    ///
    /// # Examples
    /// ```
    /// use panduck_types::errors::{CompilationTarget, PanduckError};
    /// let target = CompilationTarget::default();
    /// let error = PanduckError::compilation_failed(target, "Failed to generate bytecode");
    /// ```
    pub fn compilation_failed(target: CompilationTarget, message: impl ToString) -> Self {
        PanduckErrorKind::CompilationFailed {
            target,
            message: message.to_string(),
        }
        .into()
    }

    pub fn unsupported_feature(feature_name: String, location: SourceLocation) -> PanduckError {
        PanduckErrorKind::NotImplemented {
            feature: format!("Unsupported feature: {} at {:?}", feature_name, location),
        }
        .into()
    }
}
