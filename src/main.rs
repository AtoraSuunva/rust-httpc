use clap::Parser;

use cli::{Cli, Commands};
use helpers::{parse_headers, send_request_get_response};
use http::{Method, Request, Version};

mod cli;
mod helpers;
mod request;

fn main() {
    let args = Cli::parse();

    match args.command {
        Commands::Get { options } => {
            do_request(
                Method::GET,
                &options.url,
                options.header,
                None,
                options.verbose,
                options.output,
            );
        }
        Commands::Post {
            options,
            data,
            file,
        } => {
            let body: Option<Vec<u8>> = match (data, file) {
                // -d '{"data": "here"}'
                (Some(data), None) => Some(data.into_bytes()),
                // -f ./file.txt
                (None, Some(file)) => Some(std::fs::read(file).unwrap()),
                _ => None,
            };

            do_request(
                Method::POST,
                &options.url,
                options.header,
                body.as_deref(),
                options.verbose,
                options.output,
            );
        }
    }
}

fn do_request(
    method: Method,
    uri: &str,
    headers: Vec<String>,
    body: Option<&[u8]>,
    verbose: bool,
    output: Option<String>,
) {
    let mut request = Request::builder()
        .version(Version::HTTP_11)
        .method(method)
        .uri(uri);

    let req_headers = request.headers_mut().unwrap();

    for (name, value) in parse_headers(headers) {
        req_headers.append(name, value);
    }

    let request = request.body(body).unwrap();
    let response = send_request_get_response(request, verbose).unwrap();

    if let Some(file) = output {
        std::fs::write(&file, response).unwrap();
        println!("Response saved to {}", file);
    } else {
        print!("{}", response);
    }
}
