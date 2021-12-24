use super::*;

/// A subcommand for controlling testing
#[derive(Debug, Parser)]
pub struct UpdateCommand {
    /// Print debug info
    #[clap(short)]
    path: bool,
}
