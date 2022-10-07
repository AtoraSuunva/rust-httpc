use std::{error::Error, str::FromStr};

use clap::Parser;

use cli::{Cli, Commands, VERBOSE};
use helpers::{format_response, parse_headers};
use http::{header, Method, Request, Response, Uri, Version};
use http_request::{http_request, RequestError};
use owo_colors::{OwoColorize, Style};

use crate::{
    cli::VERY_VERBOSE,
    helpers::{resolve_url, should_redirect, MColorize},
};

mod cli;
mod helpers;
mod http_request;

fn main() -> Result<(), RequestError> {
    let args = Cli::parse();
    args.color.init();

    match args.command {
        Commands::Get { options } => do_request(
            Method::GET,
            &options.url,
            options.header,
            None,
            options.verbosity,
            options.output,
            options.location,
        ),

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
                // -d '{"data": "here"}' -f ./file.txt
                (Some(_), Some(_)) => {
                    return Err(Box::<dyn Error>::from(
                        "File and data cannot be used together",
                    ))
                }
                _ => None,
            };

            do_request(
                Method::POST,
                &options.url,
                options.header,
                body.as_deref(),
                options.verbosity,
                options.output,
                options.location,
            )
        }
    }
}

fn do_request(
    method: Method,
    uri: &str,
    headers: Vec<String>,
    body: Option<&[u8]>,
    verbosity: u8,
    output: Option<String>,
    location: bool,
) -> Result<(), RequestError> {
    let mut request = Request::builder()
        .version(Version::HTTP_11)
        .method(&method)
        .uri(uri);

    let req_headers = request.headers_mut().unwrap();

    for (name, value) in parse_headers(&headers) {
        req_headers.append(name, value);
    }

    let request = request.body(body)?;
    let response = http_request(request, verbosity)?;

    // Follow redirects
    if location && should_redirect(&response.status()) {
        if let Some(header_location) = response.headers().get(header::LOCATION) {
            let header_location = header_location.to_str()?;
            let header_location = resolve_url(&Uri::from_str(uri)?, header_location);

            if verbosity >= VERBOSE {
                // Print response between redirect if verbose
                print_response(&response, verbosity)?;

                println!(
                    "\n{} {}\n",
                    "↪ Redirecting to:".out_color(|t| t.blue()),
                    header_location.out_color(|t| t.style(Style::new().blue().underline()))
                );
            }

            return do_request(
                method,
                &header_location,
                headers,
                body,
                verbosity,
                output,
                location,
            );
        }
    }

    // If we don't redirect, we can finally print (or output to file) the response

    if let Some(file) = &output {
        std::fs::write(&file, response.body())?;

        if verbosity >= VERBOSE {
            print_response(&response, verbosity)?;
            println!(
                "\n{} {}",
                "Output written to:".out_color(|t| t.bright_black()),
                file.out_color(|t| t.style(Style::new().blue().underline()))
            );
        }
    } else {
        print_response(&response, verbosity)?;
    }

    Ok(())
}

fn print_response(response: &Response<Vec<u8>>, verbosity: u8) -> Result<(), RequestError> {
    let formatted = format_response(response, verbosity)?;

    if verbosity >= VERY_VERBOSE {
        println!("{}", "← Received".out_color(|t| t.green()))
    }

    println!("{}", formatted);
    Ok(())
}
