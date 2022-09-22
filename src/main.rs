use clap::Parser;

use cli::{Cli, Commands};

mod cli;
mod http_get;
mod request;

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Get { options } => {
            http_get::http_get(options);
        }
        Commands::Post { options, data, file } => {
            println!("post: verbose: {}, header: {:?}, data: {:?}, file: {:?}, url: {}", options.verbose, options.header, data, file, options.url);
        }
    }
}
