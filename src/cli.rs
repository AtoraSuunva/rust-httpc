use clap::{ArgEnum, ArgGroup, Parser, Subcommand, ValueHint};

#[derive(ArgEnum, Clone, Copy, Debug)]
pub enum Color {
    Always,
    Auto,
    Never,
}

impl Color {
    pub fn init(self) {
        // Set a supports-color override based on the variable passed in.
        match self {
            Color::Always => owo_colors::set_override(true),
            Color::Auto => {}
            Color::Never => owo_colors::set_override(false),
        }
    }
}

// httpc help [get|post]
// httpc get [-v] (-h "k:v")* URL
// httpc post [-v] (-h "k:v")* [-d inline-data] [-f file] URL

#[derive(Debug, Parser)]
#[clap(version, about)]
pub struct Cli {
    /// Get help for this command.
    #[clap(long)]
    help: bool,

    /// Should the output be in color?
    #[clap(long, arg_enum, global = true, default_value = "auto")]
    pub color: Color,

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
    /// Verbosity of the output, -v = Prints the detail of the response such as protocol, status, and headers., -vv = and print request message
    #[clap(short, action = clap::ArgAction::Count)]
    pub verbosity: u8,

    /// Output to a file instead of stdout
    #[clap(short, value_name = "FILE", value_hint = ValueHint::FilePath)]
    pub output: Option<String>,

    /// Follow 'Location' header redirects by repeating requests
    #[clap(short)]
    pub location: bool,

    /// Associates headers to HTTP Request with the format 'key:value'.
    #[clap(short, value_name = "key:value")]
    pub header: Vec<String>,

    /// URL to send the request to.
    #[clap(required = true, value_hint = ValueHint::Url)]
    pub url: String,
}

pub const VERBOSE: u8 = 1;
pub const VERY_VERBOSE: u8 = 2;
