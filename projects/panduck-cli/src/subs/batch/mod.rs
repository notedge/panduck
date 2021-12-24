use super::*;

/// Format conversion in batch progressing
#[derive(Debug, Parser)]
pub struct BatchCommand {
    /// path of the batch file
    #[clap(short, long)]
    path: String,
    /// format of the batch file
    #[clap(short, long)]
    format: Option<String>,
}
