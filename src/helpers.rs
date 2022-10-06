use std::fmt::Write;
use std::str::from_utf8;

use http::header::{HeaderName, CONTENT_TYPE};
use http::{HeaderValue, Response, StatusCode, Uri};
use owo_colors::{OwoColorize, Stream, Style, SupportsColorsDisplay};

use crate::cli::VERBOSE;

// Shortcut for <Sized>.if_supports_color(Stream::Stdout)
pub trait MColorize: Sized {
    /// Colorize only if supports color on stdout
    #[must_use]
    fn out_color<'a, Out, ApplyFn>(
        &'a self,
        apply: ApplyFn,
    ) -> SupportsColorsDisplay<'a, Self, Out, ApplyFn>
    where
        ApplyFn: Fn(&'a Self) -> Out,
    {
        self.if_supports_color(Stream::Stdout, apply)
    }
}

impl<D: Sized> MColorize for D {}

/// Parses headers from an vect of strings into a vec of (key, value) tuples
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

fn color_status(status: &StatusCode) -> Style {
    if status.is_informational() {
        Style::new().blue()
    } else if status.is_success() {
        Style::new().green()
    } else if status.is_redirection() {
        Style::new().blue()
    } else if status.is_client_error() || status.is_server_error() {
        Style::new().red()
    } else {
        Style::new().yellow()
    }
}

/// Parses and format the response as a pretty string
pub fn format_response(
    response: &Response<Vec<u8>>,
    verbosity: u8,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut formatted: String = String::new();

    // Log headers
    if verbosity >= VERBOSE {
        writeln!(
            formatted,
            "{} {}",
            "HTTP/1.1".out_color(|t| t.bright_black()),
            response
                .status()
                .out_color(|t| t.style(color_status(&response.status())))
        )?;
        for (key, value) in response.headers() {
            let value = value.to_str().unwrap();
            writeln!(
                formatted,
                "{}: {}",
                key.out_color(|t| t.cyan()),
                value.out_color(|t| t.magenta())
            )?;
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

/// Get the authority from a Uri
///
/// This is the host and port, e.g. www.example.com:80
pub fn get_authority(uri: &Uri) -> String {
    let port = uri.port_u16().unwrap_or_else(|| match uri.scheme() {
        Some(scheme) => match scheme.as_str() {
            "http" => 80,
            "https" => 443,
            _ => panic!("Unknown scheme"),
        },
        // Assume http
        None => 80,
    });

    let host = uri.host().expect("URI has no host").to_string();
    format!("{}:{}", host, port)
}

/// Check if the "Location" header has meaning
///
/// We should only redirect on 3xx or 201 status codes
///
/// https://httpwg.org/specs/rfc9110.html#field.location
pub fn should_redirect(code: &StatusCode) -> bool {
    code.is_redirection() || code == &StatusCode::CREATED
}

/// Attempts to resolve a url based on the location header given
///
/// This is a best-attempt to replicate the spec and what chrome/firefox do
///
/// Doesn't resolve `.` or `..` in the path
pub fn resolve_url(base: &Uri, url: &str) -> String {
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
