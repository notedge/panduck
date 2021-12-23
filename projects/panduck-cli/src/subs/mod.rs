mod formats;
mod install;

pub use self::{formats::HTMLCommand, install::InstallCommand};

use clap::Parser;

#[derive(Debug, Parser)]
pub enum SubCommands {
    Install(InstallCommand),
    HTML(HTMLCommand),
}
