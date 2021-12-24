use super::*;

/// A subcommand for controlling testing
#[derive(Debug, Parser)]
pub struct BatchCommand {
    /// Format conversion in batch
    #[clap(short)]
    path: String,
}
