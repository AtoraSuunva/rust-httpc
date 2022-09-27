use clap::{ArgGroup, Parser, Subcommand, ValueHint};

// httpc help [get|post]
// httpc get [-v] (-h "k:v")* URL
// httpc post [-v] (-h "k:v")* [-d inline-data] [-f file] URL

#[derive(Debug, Parser)]
#[clap(version, about)]
pub struct Cli {
    /// Get help for this command.
    #[clap(long)]
    help: bool,

    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
#[clap(group(ArgGroup::new("body")))]
pub enum Commands {
    /// Executes an HTTP GET request and prints the response.
    Get {
        #[clap(flatten)]
        options: CommonOptions,
    },

    /// Executes an HTTP POST request and prints the response.
    Post {
        #[clap(flatten)]
        options: CommonOptions,

        /// Associates an inline data to the body HTTP POST request.
        #[clap(short, group = "body", value_parser)]
        data: Option<String>,

        /// Associates the content of a file to the body HTTP POST request.
        #[clap(short, group = "body", value_parser, value_hint = ValueHint::FilePath)]
        file: Option<String>,
    },
}

#[derive(Debug, Parser)]
pub struct CommonOptions {
    /// Prints the detail of the response such as protocol, status, and headers.
    #[clap(short, value_parser)]
    pub verbose: bool,

    /// Output to a file instead of stdout
    #[clap(short, value_parser, value_name = "FILE", value_hint = ValueHint::FilePath)]
    pub output: Option<String>,

    /// Follow 'Location' header redirects by repeating requests
    #[clap(short, value_parser)]
    pub location: bool,

    /// Associates headers to HTTP Request with the format 'key:value'.
    #[clap(short, value_parser, value_name = "key:value")]
    pub header: Vec<String>,

    /// URL to send the request to.
    #[clap(required = true, value_parser, value_hint = ValueHint::Url)]
    pub url: String,
}
