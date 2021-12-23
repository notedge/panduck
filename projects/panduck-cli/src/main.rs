mod config;
mod subs;

use clap::Parser;
use notedown_ast::Result;
use subs::SubCommands;

#[derive(Parser)]
#[clap(version = env!("CARGO_PKG_VERSION"), author = env!("CARGO_PKG_AUTHORS"))]
struct Arguments {
    /// Sets a custom config file. Could have been an Option<T> with no default too
    #[clap(short, long, default_value = "panduck.toml")]
    config: String,
    /// A level of verbosity, and can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,
    #[clap(subcommand)]
    sub_cmd: SubCommands,
}

fn main() -> Result<()> {
    let opts: Arguments = Arguments::parse();

    // Gets a value for config if supplied by user, or defaults to "default.conf"
    println!("Value for config: {}", opts.config);

    // Vary the output based on how many times the user used the "verbose" flag
    // (i.e. 'myprog -v -v -v' or 'myprog -vvv' vs 'myprog -v'
    match opts.verbose {
        0 => println!("No verbose info"),
        1 => println!("Some verbose info"),
        2 => println!("Tons of verbose info"),
        3 | _ => println!("Don't be crazy"),
    }

    // You can handle information about subcommands by requesting their matches by name
    // (as below), requesting just the name used, or both at the same time

    match &opts.sub_cmd {
        _ => {
            println!("{:#?}", opts.sub_cmd)
        }
    }

    Ok(())
}
