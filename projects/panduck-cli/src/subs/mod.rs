mod batch;
mod formats;
mod install;
mod update;

pub use self::{formats::*, install::InstallCommand};

use crate::PanduckConfig;
use clap::Parser;
use notedown_ast::Result;

// todo: sort commands
#[derive(Debug, Parser)]
pub enum SubCommands {
    Install(InstallCommand),
    Update(InstallCommand),
    Batch(InstallCommand),
    HTML(HTML),
    PDF(HTML),
    #[clap(name = "latex")]
    LaTeX(HTML),
    CommonMD(CommonMD),
    GithubMD(GithubFavoredMD),
    #[clap(name = "pandoc")]
    PandocJSON(HTML),
    Notedown(HTML),
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
