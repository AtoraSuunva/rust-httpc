use clap::{ArgGroup, Parser, Subcommand, ValueHint};

// httpc help [get|post]
// httpc get [-v] (-h "k:v")* URL
// httpc post [-v] (-h "k:v")* [-d inline-data] [-f file] URL

#[derive(Debug, Parser)]
#[clap(name = "httpc")]
#[clap(version = "1.0")]
#[clap(about = "HTTP client", long_about = "long http client")]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
#[clap(group(ArgGroup::new("body")))]
pub enum Commands {
    /// Executes a HTTP GET request and prints the response.
    Get {
        #[clap(flatten)]
        options: CommonOptions,
    },
    /// Executes a HTTP POST request and prints the response.
    Post {
        #[clap(flatten)]
        options: CommonOptions,
        /// string Associates an inline data to the body HTTP POST request.
        #[clap(short, group = "body", value_parser)]
        data: Option<String>,
        /// file Associates the content of a file to the body HTTP POST request.
        #[clap(short, group = "body", value_parser, value_hint = ValueHint::FilePath)]
        file: Option<String>,
    },
}

#[derive(Debug, Parser)]
pub struct CommonOptions {
    /// Get help for this command.
    #[clap(long)]
    help: bool,
    /// Prints the detail of the response such as protocol, status, and headers.
    #[clap(short, value_parser)]
    pub verbose: bool,
    /// key:value Associates headers to HTTP Request with the format 'key:value'.
    #[clap(short, value_parser)]
    pub header: Vec<String>,
    /// URL to send the request to.
    #[clap(required = true, value_parser, value_hint = ValueHint::Url)]
    pub url: String,
}
