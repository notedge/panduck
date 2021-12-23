mod config;
mod subs;

use self::{config::PanduckConfig, subs::SubCommands};
use clap::Parser;
pub use notedown_ast::Result;

#[derive(Parser)]
#[clap(version = env!("CARGO_PKG_VERSION"), author = env!("CARGO_PKG_AUTHORS"))]
struct Arguments {
    /// Sets a custom config file. Could have been an Option<T> with no default too
    #[clap(short, long, default_value = "panduck.toml")]
    config: String,
    /// A level of verbosity, and can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    verbose: u32,
    #[clap(subcommand)]
    sub_cmd: SubCommands,
}

fn main() -> Result<()> {
    let args: Arguments = Arguments::parse();
    // Gets a value for config if supplied by user, or defaults to "default.conf"
    println!("Value for config: {}", args.config);
    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
    match args.verbose {
        0 => println!("No verbose info"),
        1 => println!("Some verbose info"),
        2 => println!("Tons of verbose info"),
        3 | _ => println!("Don't be crazy"),
    }
    let mut config = PanduckConfig::default();
    config.verbose = args.verbose;
    args.sub_cmd.dispatch(&config)
}
