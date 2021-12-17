use super::*;

use serde_json::Error;

impl From<Error> for PanduckError {
    fn from(e: Error) -> Self {
        let kind = match e {
            Error { .. } => {}
        };

        Self::ParseError(format!("{}", e))
    }
}
