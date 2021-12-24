mod parse;

use panduck_html::HTMLConfig;
use serde::{Deserialize, Serialize};
use std::{env::VarError, io::Error, path::PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PanduckConfig {
    pub verbose: u32,
    pub dry_run: bool,
    pub package_root: PathBuf,
    pub html: HTMLConfig,
}

impl Default for PanduckConfig {
    fn default() -> Self {
        Self {
            verbose: 0, //
            html: Default::default(),
            package_root: Self::get_pkg_path().unwrap_or_default(),
            dry_run: false,
        }
    }
}

impl PanduckConfig {
    #[inline]
    fn get_pkg_path() -> Result<PathBuf, Error> {
        std::env::var("NOTEDOWN_PATH") //
            .map(|f| PathBuf::from(f))
            .or(Self::get_cur_path())
    }

    fn get_cur_path() -> Result<PathBuf, Error> {
        std::env::current_exe().map(|f| f.parent().map(|f| f.to_path_buf()).unwrap_or(PathBuf::from("/")))
    }
}
