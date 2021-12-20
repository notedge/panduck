use super::*;

use serde_json::Error;

impl From<Error> for PanduckError {
    fn from(e: Error) -> Self {
        Self::parse_error(e.to_string())
    }
}
