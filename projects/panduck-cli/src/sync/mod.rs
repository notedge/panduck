use std::{env::VarError, path::PathBuf};

use notedown_ast::{NoteError, Result};

pub static NOTEDOWN_PATH: &str = "NOTEDOWN_PATH";

pub fn get_root_path() -> Result<PathBuf> {
    match std::env::var(NOTEDOWN_PATH) {
        Ok(o) => {
            let maybe_dir = PathBuf::from(o);
            match maybe_dir.is_dir() {
                true => Ok(maybe_dir),
                false => Err(NoteError::runtime_error(&format!(
                    "The environment variable {} does not point to a directory!",
                    NOTEDOWN_PATH
                ))),
            }
        }
        Err(VarError::NotPresent) => {
            Err(NoteError::runtime_error(&format!("The environment variable {} does not found!", NOTEDOWN_PATH)))
        }
        Err(VarError::NotUnicode(s)) => Err(NoteError::runtime_error(&format!(
            "The environment variable {} seems not valid unicode: {:?}",
            NOTEDOWN_PATH, s
        ))),
    }
}
