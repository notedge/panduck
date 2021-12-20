use super::*;
use notedown_ast::{NoteError, NoteErrorKind};
use std::string::FromUtf8Error;

impl From<std::io::Error> for PanduckError {
    fn from(e: std::io::Error) -> Self {
        Self { kind: box PanduckErrorKind::IOError(e), file: None, range: None }
    }
}

impl From<()> for PanduckError {
    fn from(_: ()) -> Self {
        Self { kind: box PanduckErrorKind::Unknown, file: None, range: None }
    }
}

impl From<FromUtf8Error> for PanduckError {
    fn from(e: FromUtf8Error) -> Self {
        Self { kind: box PanduckErrorKind::ParseError(e.to_string()), file: None, range: None }
    }
}

impl From<NoteError> for PanduckError {
    fn from(e: NoteError) -> Self {
        let kind = box match *e.kind {
            NoteErrorKind::IOError(e) => PanduckErrorKind::IOError(e),
            NoteErrorKind::FormatError(e) => PanduckErrorKind::ParseError(e.to_string()),
            NoteErrorKind::TypeMismatch(e) => PanduckErrorKind::ParseError(e.to_string()),
            NoteErrorKind::RuntimeError(e) => PanduckErrorKind::ParseError(e.to_string()),
            NoteErrorKind::Unreachable => PanduckErrorKind::Unknown,
        };
        Self { kind, file: e.file, range: e.range }
    }
}
