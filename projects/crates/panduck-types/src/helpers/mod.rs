#![doc = include_str!("readme.md")]
pub use self::compilation_target::{AbiCompatible, ApiCompatible, Architecture, CompilationTarget};
use crate::PanduckError;
use std::fs::File;
use std::path::Path;
pub use url::Url;

mod compilation_target;
mod source_text;

pub use self::source_text::{SourceLocation, SourcePosition, SourceText};

pub fn check_path(path: impl AsRef<Path>) -> Result<(File, Url), PanduckError> {
    let path = path.as_ref();
    let url = match Url::from_file_path(path) {
        Ok(o) => o,
        Err(_) => Err(PanduckError::invalid_data("invalid file path"))?,
    };
    match File::open(path) {
        Ok(o) => Ok((o, url)),
        Err(e) => Err(PanduckError::io_error(e, url))?,
    }
}
