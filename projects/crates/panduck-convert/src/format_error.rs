use notedown_formats::{from_format_error, FormatError};
use panduck_types::{AdapterError, Result};

pub(crate) fn fail<T>(error: AdapterError) -> Result<T> {
    panduck_diagnostic::log_adapter_error(&error);
    Err(error)
}

pub(crate) fn propagate<T>(result: Result<T>) -> Result<T> {
    if let Err(error) = &result {
        panduck_diagnostic::log_adapter_error(error);
    }
    result
}

/// Convert a Notedown [`FormatError`] into an [`AdapterError`] while preserving
/// the structured diagnostic code, typed args, and origin from the formats
/// layer. This avoids the previous pattern of stringifying the error into a
/// free-form message and losing the stable wire identifiers.
pub(crate) fn map_format_error(error: FormatError) -> AdapterError {
    let diagnostic = from_format_error(&error);
    AdapterError::diagnostic(diagnostic)
}
