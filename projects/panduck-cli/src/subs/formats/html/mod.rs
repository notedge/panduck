use clap::Parser;

/// A subcommand for controlling testing
#[derive(Debug, Parser)]
pub struct HTMLCommand {
    /// Print debug info
    #[clap(short, long)]
    /// Some input. Because this isn't an Option<T> it's required to be used
    input: String,
    #[clap(short, long)]
    output: Option<String>,
    #[clap(short, long)]
    pretty_print: bool,
}
