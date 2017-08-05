#![doc = include_str!("readme.md")]

use crate::AdapterError;
use std::fs::File;
use std::path::Path;
pub use url::Url;

mod source_text;

pub use self::source_text::{SourceLocation, SourcePosition, SourceText};

/// Open a filesystem path and return its file handle plus `file://` URL.
pub fn check_path(path: impl AsRef<Path>) -> crate::Result<(File, Url)> {
    let path = path.as_ref();
    let url = match Url::from_file_path(path) {
        Ok(url) => url,
        Err(_) => return Err(AdapterError::invalid_input("invalid file path")),
    };
    let file = File::open(path).map_err(|e| AdapterError::io(e, Some(path.display().to_string())))?;
    Ok((file, url))
}
