use std::{
    fmt::Write,
    io::{self, prelude::*, BufReader},
    net::{SocketAddr, TcpStream, ToSocketAddrs},
    str::from_utf8,
};

use http::{
    header::{self, HeaderName},
    HeaderMap, HeaderValue, Request, Response, Uri,
};
use native_tls::TlsConnector;
use owo_colors::{OwoColorize, Style};

use crate::{
    cli::VERY_VERBOSE,
    helpers::{get_authority, MColorize},
};

// TODO: better error type...
pub type RequestError = Box<dyn std::error::Error>;

/// Execute an HTTP 1.1 request, then parse the response
/// This will build the request line, headers, and body (if any), then send it to the server
///
/// Note: if the server returns an incorrect content-length that's:
///   - too long: client will block until the tcp connection times out
///   - too short: the returned body will be cut short
///   - not present: content-length defaults to 0, so no body is returned
pub fn http_request(
    req: Request<Option<&[u8]>>,
    verbosity: u8,
) -> Result<Response<Vec<u8>>, RequestError> {
    // Create HTTP request we'll send
    let http_message = create_http_message(&req)?;

    if verbosity >= VERY_VERBOSE {
        let (message, body) = http_message.to_parts(&RequestStyles::colorized())?;
        let display_body = if !body.is_empty() {
            match from_utf8(body.as_slice()) {
                Ok(body) => format!("{}\n\n", body),
                Err(_) => String::from("[Invalid UTF-8]"),
            }
        } else {
            String::new()
        };

        print!(
            "{}\n{}{}",
            "→ Sending".out_color(|t| t.yellow()),
            message,
            display_body,
        );
    }

    // Connect to server via TCP, using TLS for https
    let mut stream = tcp_connect(req.uri())?;

    // Send request
    let (message, body) = http_message.to_parts(&RequestStyles::default())?;
    stream.write_all(message.as_bytes())?;
    stream.write_all(body.as_slice())?;

    // Read & Parse response
    let buf_reader = BufReader::new(stream);
    parse_http_response(buf_reader)
}

trait ReadAndWrite: io::Read + io::Write {}

impl<T: io::Read + io::Write> ReadAndWrite for T {}

/// Connects to a server via TCP, using TLS for https
fn tcp_connect(uri: &Uri) -> Result<Box<dyn ReadAndWrite>, RequestError> {
    let authority = get_authority(uri);
    let addresses: Vec<SocketAddr> = authority.to_socket_addrs()?.collect();
    let stream = TcpStream::connect(addresses.as_slice())?;

    if uri.scheme_str() == Some("https") {
        // We need to setup a TLS connector to handle HTTPS for us
        // I am not implementing crypto myself, so this uses native_tls
        // Which binds to native implementations for us
        // (openssl on linux, schannel on windows, security-framework on macos)
        let connector = TlsConnector::new().unwrap();
        let stream = connector.connect(uri.host().unwrap(), stream)?;
        Ok(Box::new(stream))
    } else {
        Ok(Box::new(stream))
    }
}

#[derive(Debug, Default)]
struct RequestStyles {
    method: Style,
    abs_path: Style,
    version: Style,
    header_name: Style,
    header_value: Style,
}

impl RequestStyles {
    fn colorized() -> Self {
        Self {
            method: Style::new().green(),
            abs_path: Style::new().blue(),
            version: Style::new().bright_black(),
            header_name: Style::new().cyan(),
            header_value: Style::new().purple(),
        }
    }
}

struct HttpMessage {
    method: String,
    abs_path: String,
    version: String,
    headers: HeaderMap,
    body: Option<Vec<u8>>,
}

impl HttpMessage {
    fn to_parts(&self, styles: &RequestStyles) -> Result<(String, Vec<u8>), std::fmt::Error> {
        let mut message = String::new();

        write!(
            message,
            "{} {} {}\r\n",
            self.method.style(styles.method),
            self.abs_path.style(styles.abs_path),
            self.version.style(styles.version),
        )?;

        for (name, value) in &self.headers {
            write!(
                message,
                "{}: {}\r\n",
                name.style(styles.header_name),
                value.to_str().unwrap().style(styles.header_value),
            )?;
        }

        message.push_str("\r\n");

        Ok((message, self.body.clone().unwrap_or_default()))
    }
}

impl From<&Request<Option<&[u8]>>> for HttpMessage {
    fn from(req: &Request<Option<&[u8]>>) -> Self {
        let method = req.method().to_string();
        let abs_path = req.uri().path_and_query().unwrap().to_string();
        let version = format!("{:?}", req.version());
        let headers = req.headers().to_owned();
        let body = req.body().map(|b| b.to_vec());

        Self {
            method,
            abs_path,
            version,
            headers,
            body,
        }
    }
}

/// Create a valid HTTP message from a Request
///
/// This will add any missing required/"strongly suggested" headers (Host, User-Agent, Connection, Content-Length) if not already defined
/// and then build an HttpMessage that can be turned into a string (to send) or a colored string (to display)
///
/// Note: Rust uses UTF-8 as default string encodings, so Header/Values are encoded as UTF-8.
/// In most cases you're likely using ASCII-compatible characters, so this is fine, but you might run into
/// oddities if you start sending UTF-8 characters in your headers
fn create_http_message(req: &Request<Option<&[u8]>>) -> Result<HttpMessage, RequestError> {
    let authority = get_authority(req.uri());
    let mut added_headers = HeaderMap::new();

    // Host: www.example.com
    if !req.headers().contains_key(header::HOST) {
        added_headers.insert(header::HOST, authority.parse()?);
    }

    // Set a default UA
    if !req.headers().contains_key(header::USER_AGENT) {
        added_headers.insert(
            header::USER_AGENT,
            format!("httpc/{}", env!("CARGO_PKG_VERSION")).parse()?,
        );
    }

    // Set a default connection header
    // We don't reuse the connection, so just tell the server to close
    if !req.headers().contains_key(header::CONNECTION) {
        added_headers.insert(header::CONNECTION, "close".parse()?);
    }

    // Calculate content-length
    // Can't chain this, see https://github.com/rust-lang/rust/issues/53667
    if !req.headers().contains_key(header::CONTENT_LENGTH) {
        if let Some(body) = req.body() {
            added_headers.insert(header::CONTENT_LENGTH, body.len().to_string().parse()?);
        }
    }

    let mut message = HttpMessage::from(req);
    message.headers.extend(added_headers);

    Ok(message)
}

/// Parse an HTTP response into a rust Response
fn parse_http_response<T: Read>(reader: BufReader<T>) -> Result<Response<Vec<u8>>, RequestError> {
    // Store the HTTP status code, also serves as a signal that we should parse headers
    let mut status_code: Option<u16> = None;
    // Length of body in bytes (from 'Content-Length' header)
    let mut content_length = 0;
    // Is the content body chunked
    let mut chunked = false;

    let mut response_builder = Response::builder();
    let response_headers = response_builder
        .headers_mut()
        .expect("Failed to get mut ref to headers");

    let mut byte_iter = reader.bytes();

    // Parse the metadata: status code & headers
    loop {
        // We need to read up to next line
        // Lines end with \r\n so we collect bytes up to \r\n and then parse the line
        let mut line: Vec<u8> = vec![];
        loop {
            // We won't deal with invalid bytes
            let byte = byte_iter.next().unwrap()?;
            line.push(byte);
            if line.ends_with(b"\r\n") {
                break;
            }
        }

        if line == b"\r\n" {
            // We've reached the end of the HTTP headers
            break;
        } else if status_code.is_none() {
            // First line is status code
            let status_code_str = from_utf8(&line).unwrap();
            let status_code_str = status_code_str.split_whitespace().nth(1).unwrap();
            let status_code_u16 = status_code_str.parse::<u16>()?;
            status_code = Some(status_code_u16);
        } else {
            // Other lines are headers
            let header = from_utf8(&line).unwrap();
            let header = header.split_once(':').unwrap();
            let header_name = header.0.trim();
            let header_value = header.1.trim();

            if header_name.to_lowercase() == "content-length" {
                content_length = header_value.parse::<usize>()?;
            }

            if header_name.to_lowercase() == "transfer-encoding"
                && header_value.to_lowercase().contains("chunked")
            {
                chunked = true;
            }

            response_headers.insert(
                header_name.parse::<HeaderName>()?,
                header_value.parse::<HeaderValue>()?,
            );
        }
    }

    // We hit the empty line that says we've reached the body of the message
    // Make sure we received a status code (which needs to be there for a valid message)
    // And then continue on to parse the body

    if status_code.is_none() {
        return Err("No status code found".into());
    }

    // The body we've received
    let mut body: Vec<u8> = Vec::with_capacity(content_length);

    if !chunked {
        if content_length > 0 {
            // Parse the body, reading bytes until we meet content-length or end of stream
            for byte in byte_iter {
                body.push(byte.unwrap());
                if body.len() >= content_length {
                    break;
                }
            }
        }
    } else {
        loop {
            // Read the chunk "head"
            // [hex octets]*(;ext-name=ext-val)\r\n
            // We need the num of octects in the chunk, but can ignore the chunk-ext
            // We don't recognize any chunk extensions, so we MUST ignore them

            // Read octets
            let mut octets: Vec<u8> = vec![];
            loop {
                let byte = byte_iter.next().unwrap()?;
                if byte == b';' || byte == b'\r' {
                    break;
                }
                octets.push(byte);
            }

            // Read until end of line
            loop {
                let byte = byte_iter.next().unwrap()?;
                if byte == b'\n' {
                    break;
                }
            }

            let octets = usize::from_str_radix(from_utf8(&octets).unwrap(), 16)?;

            if octets == 0 {
                // We've reached the end of the chunked body
                // Technically there's trailing headers, but since we don't send "TE: trailers"
                // the server knows we might just discard the trailers
                // so we can just discard the trailers and still respect the spec 😎
                break;
            }

            // Read the chunk
            for _ in 0..octets {
                body.push(byte_iter.next().unwrap()?);
            }

            // Read the chunk end
            loop {
                let byte = byte_iter.next().unwrap()?;
                if byte == b'\r' && byte_iter.next().unwrap()? == b'\n' {
                    break;
                }
            }
        }
    }

    // Then we can just finalize the response and return it
    Ok(response_builder
        .status(status_code.unwrap())
        .body(body)
        .expect("Failed to construct response"))
}
