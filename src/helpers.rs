use std::fmt::Write;
use std::str::from_utf8;

use http::header::{HeaderName, CONTENT_TYPE};
use http::{HeaderValue, Response};

use crate::cli::VERBOSE;

pub fn parse_headers(header_strings: &Vec<String>) -> Vec<(HeaderName, HeaderValue)> {
    let mut headers: Vec<(HeaderName, HeaderValue)> = Vec::new();

    for header in header_strings {
        let (key, value) = header
            .split_once(':')
            .map(|(name, value)| (name.trim(), value.trim()))
            .expect("Invalid header format");

        headers.push((key.parse::<HeaderName>().unwrap(), value.parse().unwrap()));
    }

    headers
}

/// Parses and format the response as a pretty string
pub fn format_response(
    response: &Response<Vec<u8>>,
    verbosity: u8,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut formatted: String = String::new();

    // Log headers
    if verbosity >= VERBOSE {
        writeln!(formatted, "HTTP/1.1 {}", response.status())?;
        for (key, value) in response.headers() {
            writeln!(formatted, "{}: {}", key, value.to_str().unwrap())?;
        }
        formatted.push('\n');
    }

    match response.headers().get(CONTENT_TYPE) {
        Some(content_type) => {
            let content_type = content_type.to_str().unwrap();
            if content_type.starts_with("text/") || content_type == "application/json" {
                let body = response.body();
                let text = from_utf8(body).unwrap();

                if !text.is_empty() {
                    writeln!(formatted, "{}", text)?;
                }
            } else {
                writeln!(formatted, "Binary data, not displaying.")?;
            }
        }
        None => {
            writeln!(
                formatted,
                "No content type header, not displaying anything."
            )?;
        }
    }

    Ok(formatted)
}
