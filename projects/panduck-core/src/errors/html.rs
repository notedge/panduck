use html_parser::Error;

use super::*;

impl From<Error> for PanduckError {
    fn from(e: Error) -> Self {
        let kind = box match e {
            Error::Parsing(e) => PanduckErrorKind::SyntaxError(e),
            Error::IO(e) => PanduckErrorKind::IOError(e),
            Error::Cli(_) => {
                unimplemented!()
            }
            Error::Serde(_) => {
                unimplemented!()
            }
        };
        Self { kind, file: None, range: None }
    }
}
