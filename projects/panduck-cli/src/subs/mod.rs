mod formats;
mod install;

pub use self::{formats::HTMLCommand, install::InstallCommand};

use crate::PanduckConfig;
use clap::Parser;
use notedown_ast::Result;

// todo: sort commands
#[derive(Debug, Parser)]
pub enum SubCommands {
    Install(InstallCommand),
    Update(InstallCommand),
    HTML(HTMLCommand),
    PDF(HTMLCommand),
    LATEX(HTMLCommand),
    CommonMD(HTMLCommand),
    GithubMD(HTMLCommand),
    PandocJSON(HTMLCommand),
    Notedown(HTMLCommand),
}

impl SubCommands {
    pub fn dispatch(&self, cfg: &mut PanduckConfig) -> Result<()> {
        match self {
            Self::Install(_) => Ok(()),
            Self::HTML(v) => {
                v.apply_args(cfg);
                v.dispatch(cfg)
            }
            _ => {
                println!("{:#?}", self);
                println!("{:#?}", cfg);
                Ok(())
            }
        }
    }
}
