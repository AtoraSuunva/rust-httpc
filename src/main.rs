use clap::Parser;

use cli::{Cli, Commands};
use helpers::{format_response, parse_headers};
use http::{header, Method, Request, Version};
use request::http_request;

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
                options.location,
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
                options.location,
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
    location: bool,
) {
    let mut request = Request::builder()
        .version(Version::HTTP_11)
        .method(&method)
        .uri(uri);

    let req_headers = request.headers_mut().unwrap();

    for (name, value) in parse_headers(&headers) {
        req_headers.append(name, value);
    }

    let request = request.body(body).unwrap();
    let response = http_request(request).expect("Request failed");
    let formatted = format_response(&response, verbose).unwrap();

    if let Some(file) = &output {
        std::fs::write(&file, formatted).unwrap();
        println!("Response saved to {}", file);
    } else {
        print!("{}", formatted);
    }

    // Follow redirects
    if location {
        if let Some(header_location) = response.headers().get(header::LOCATION) {
            let header_location = header_location.to_str().unwrap();

            do_request(
                method,
                header_location,
                headers,
                body,
                verbose,
                output,
                location,
            );
        }
    }
}
