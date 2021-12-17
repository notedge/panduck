mod basic;
#[cfg(feature = "docx-rs")]
mod docx;
#[cfg(feature = "html_parser")]
mod html;
#[cfg(feature = "serde_json")]
mod json;

use self::PanduckErrorKind::*;
use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    ops::Range,
    path::Path,
};
use yggdrasil_shared::records::Url;

pub type Result<T> = std::result::Result<T, PanduckError>;

#[derive(Debug)]
pub struct PanduckError {
    kind: PanduckErrorKind,
    file: Option<Url>,
    range: Option<Range<usize>>,
}

#[derive(Debug)]
pub enum PanduckErrorKind {
    IOError(std::io::Error),
    ParseError(String),
    UnsupportedFormat(String),
    Unknown,
}

// noinspection ALL
impl PanduckError {
    #[inline]
    pub fn set_url(mut self, url: Url) -> Self {
        self.file = Some(url);
        return self;
    }
    #[inline]
    pub fn set_path(self, path: impl AsRef<Path>) -> Self {
        match Url::from_directory_path(path) {
            Ok(url) => self.set_url(url),
            Err(_) => self,
        }
    }
    #[inline]
    pub fn set_range(mut self, range: Range<usize>) -> Self {
        self.range = Some(range);
        return self;
    }
    #[inline]
    pub fn set_offset(mut self, start: usize, end: usize) -> Self {
        self.range = Some(Range { start, end });
        return self;
    }
}

impl PanduckError {
    pub fn unsupported_file(msg: impl Into<String>) -> Self {
        Self { kind: UnsupportedFormat(msg.into()), file: None, range: None }
    }
}

impl Default for PanduckError {
    fn default() -> Self {
        Self { kind: Unknown, file: None, range: None }
    }
}

impl Error for PanduckError {}

impl Display for PanduckError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let path = match &self.file {
            Some(s) => s.path(),
            None => "<Anonymous>",
        };
        match &self.range {
            Some(s) => writeln!(f, "at ({}, {}) of {}", s.start, s.end, path)?,
            None => writeln!(f, "at {}", path)?,
        }
        write!(f, "{:indent$}{}", self.kind, indent = 4)
    }
}

impl Display for PanduckErrorKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            IOError(e) => Display::fmt(e, f),
            ParseError(_) => {
                unimplemented!()
            }
            UnsupportedFormat(_) => {
                unimplemented!()
            }
            Unknown => {
                unimplemented!()
            }
        }
    }
}
