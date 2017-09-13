use std::fs;
use std::path::Path;

use notedown_ir::DocumentGraph;
use notedown_formats::import::docx::{import_docx, import_docx_bytes};
use panduck_types::{AdapterError, Result};

fn map_format_error(error: notedown_formats::FormatError) -> AdapterError {
    match error {
        notedown_formats::FormatError::NotImplemented { format, direction } => {
            AdapterError::not_implemented(format!("{direction} for {format}"))
        }
        notedown_formats::FormatError::InvalidInput { message } => AdapterError::invalid_input(message),
        notedown_formats::FormatError::Parse { format, message } => AdapterError::adapter(format, message),
        notedown_formats::FormatError::Unsupported { format, operation } => {
            AdapterError::unsupported_format(format, operation)
        }
    }
}

/// Reads a DOCX file from disk into `DocumentGraph`.
pub fn read_docx(path: impl AsRef<Path>) -> Result<DocumentGraph> {
    import_docx(path).map_err(map_format_error)
}

/// Reads DOCX bytes into `DocumentGraph`.
pub fn read_docx_bytes(label: impl Into<String>, bytes: Vec<u8>) -> Result<DocumentGraph> {
    let label = label.into();
    import_docx_bytes(&label, &bytes).map_err(map_format_error)
}

/// Legacy disk read helper retained for callers that manage I/O locally.
pub fn read_docx_file(path: impl AsRef<Path>) -> Result<DocumentGraph> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|source| {
        AdapterError::io(source, Some(path.display().to_string()))
    })?;
    read_docx_bytes(path.display().to_string(), bytes)
}
