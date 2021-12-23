use clap::Clap;

#[derive(Parser)]
#[clap(version = "1.0", author = "Kevin K. <kbknapp@gmail.com>")]
struct Options {
    /// Sets a custom config file. Could have been an Option<T> with no default too
    #[clap(short, long, default_value = "default.conf")]
    config: String,
    /// Some input. Because this isn't an Option<T> it's required to be used
    input: String,
    /// A level of verbosity, and can be used multiple times
    #[clap(short, long, parse(from_occurrences))]
    verbose: i32,
    #[clap(subcommand)]
    sub_cmd: SubCommand,
}

#[derive(Parser)]
enum SubCommand {
    #[clap(version = "1.0", author = "Someone E. <someone_else@other.com>")]
    Pack(PackCommand),
}

/// A subcommand for controlling testing
#[derive(Parser)]
struct PackCommand {
    /// Print debug info
    #[clap(short)]
    path: bool,
}

fn main() {
    let opts: Options = Options::parse();

    // Gets a value for config if supplied by user, or defaults to "default.conf"
    println!("Value for config: {}", opts.config);
    println!("Using input file: {}", opts.input);

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
    match opts.sub_cmd {
        SubCommand::Pack(t) => {
            if t.path {
                println!("Printing debug info...");
            }
            else {
                println!("Printing normally...");
            }
        }
    }

    // more program logic goes here...
}
