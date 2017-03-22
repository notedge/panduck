use super::*;
use html_parser::Error;

impl From<Error> for PanduckError {
    fn from(e: Error) -> Self {
        let kind = match e {
            Error::Parsing(e) => PanduckErrorKind::ParseError(e),
            Error::IO(e) => PanduckErrorKind::IOError(e),
            Error::Cli(_) => {unimplemented!()}
            Error::Serde(_) => {unimplemented!()}
        };
        Self {
            kind
        }
    }
}