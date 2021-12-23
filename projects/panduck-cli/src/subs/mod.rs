mod formats;
mod install;

pub use self::{formats::HTMLCommand, install::InstallCommand};

use crate::PanduckConfig;
use clap::Parser;
use notedown_ast::Result;

#[derive(Debug, Parser)]
pub enum SubCommands {
    Install(InstallCommand),
    HTML(HTMLCommand),
}

impl SubCommands {
    pub fn dispatch(&self, cfg: &PanduckConfig) -> Result<()> {
        let mut cfg = cfg.to_owned();

        match self {
            Self::Install(_) => Ok(()),
            Self::HTML(v) => {
                v.apply_args(&mut cfg);
                v.dispatch(&cfg)
            }
            _ => {
                println!("{:#?}", self);
                println!("{:#?}", cfg);
                Ok(())
            }
        }
    }
}
