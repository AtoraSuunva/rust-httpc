use std::str::FromStr;

use clap::Parser;

use cli::{Cli, Commands, VERBOSE};
use helpers::{format_response, parse_headers};
use http::{header, Method, Request, StatusCode, Uri, Version};
use request::http_request;

use crate::cli::VERY_VERBOSE;

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
                options.verbosity,
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
                options.verbosity,
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
    verbosity: u8,
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

    let request = request.body(body).expect("Request failed to build");
    let response = match http_request(request, verbosity) {
        Ok(response) => response,
        Err(err) => {
            eprintln!("Request Error: {}", err);
            std::process::exit(1);
        }
    };

    let formatted = format_response(&response, verbosity).expect("Failed to format response");

    if let Some(file) = &output {
        std::fs::write(&file, response.body()).unwrap();
    } else {
        if verbosity >= VERY_VERBOSE {
            println!("← Received")
        }
        print!("{}", formatted);
    }

    // Follow redirects
    if location && should_redirect(&response.status()) {
        if let Some(header_location) = response.headers().get(header::LOCATION) {
            let header_location = header_location.to_str().unwrap();
            let header_location = resolve_url(&Uri::from_str(uri).unwrap(), header_location);

            if verbosity >= VERBOSE {
                println!("\n↪ Redirecting to: {}\n", header_location);
            }

            do_request(
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
}

/// Check if the "Location" header has meaning
///
/// We should only redirect on 3xx or 201 status codes
///
/// https://httpwg.org/specs/rfc9110.html#field.location
fn should_redirect(code: &StatusCode) -> bool {
    code.is_redirection() || code == &StatusCode::CREATED
}

/// Attempts to resolve a url based on the location header given
///
/// This is a best-attempt to replicate the spec and what chrome/firefox do
///
/// Doesn't resolve `.` or `..` in the path
fn resolve_url(base: &Uri, url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        // http://example.com/path/to/place + Location: http://foo.com
        // http://foo.com
        url.to_string()
    } else if url.starts_with('/') {
        // <original authority>/<location>
        // http://example.com/path/to/place + Location: /foo
        // http://example.com/foo
        let scheme = base.scheme_str().unwrap_or("http");
        format!("{}://{}{}", scheme, base.authority().unwrap(), url)
    } else if url.starts_with('?') {
        // http://example.com/path/to/place + Location: ?foo=bar
        // http://example.com/path/to/place?foo=bar
        let scheme = base.scheme_str().unwrap_or("http");
        format!(
            "{}://{}{}{}",
            scheme,
            base.authority().unwrap(),
            base.path(),
            url
        )
    } else {
        // <original authority>/<original path minus last part>/<location>
        // http://example.com/path/to/place + Location: foo
        // http://example.com/path/to/foo
        let scheme = base.scheme_str().unwrap_or("http");
        let path: Vec<&str> = base.path().split('/').collect();
        let path = path[..path.len() - 1].join("/");
        format!("{}://{}{}/{}", scheme, base.authority().unwrap(), path, url)
    }
}
