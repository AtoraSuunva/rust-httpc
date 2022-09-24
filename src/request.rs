use std::{
    io::{prelude::*, BufReader},
    net::{SocketAddr, TcpStream, ToSocketAddrs},
    str::from_utf8,
};

use http::{header::HeaderName, HeaderValue, Request, Response};

// TODO: better error type...
type RequestError = Box<dyn std::error::Error>;

/// Execute an HTTP 1.1 request, then parse the response
/// This will build the request line, headers, and body (if any), then send it to the server
///
/// Note: if the server returns an incorrect content-length that's:
///   - too long: client will block until the tcp connection times out
///   - too short: the returned body will be cut short
///   - not present: content-length defaults to 0, so no body is returned
pub fn http_request(req: Request<Option<&[u8]>>) -> Result<Response<Vec<u8>>, RequestError> {
    // Connect to server via TCP
    let authority = get_authority(&req);
    let addresses: Vec<SocketAddr> = authority.to_socket_addrs()?.collect();
    let mut stream = TcpStream::connect(addresses.as_slice())?;

    // Send request
    let http_message = create_http_message(&req)?;
    stream.write_all(&http_message[..])?;

    // Then read response
    let buf_reader = BufReader::new(&stream);

    // Store the HTTP status code, also serves as a signal that we should parse headers
    let mut status_code: Option<u16> = None;
    // Length of body in bytes (from 'Content-Length' header)
    let mut content_length = 0;

    let mut response_builder = Response::builder();
    let response_headers = response_builder
        .headers_mut()
        .expect("Failed to get mut ref to headers");

    let mut byte_iter = buf_reader.bytes();

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

    // Parse the body, reading bytes until we meet content-length or end of stream
    for byte in byte_iter {
        body.push(byte.unwrap());
        if body.len() >= content_length {
            break;
        }
    }

    // Then we can just finalize the response and return it
    Ok(response_builder
        .status(status_code.unwrap())
        .body(body)
        .expect("Failed to construct response"))
}

/// Get the authority from a request
///
/// This is the host and port, e.g. www.example.com:80
fn get_authority<T>(req: &Request<T>) -> String {
    let port = req.uri().port_u16().unwrap_or(80);
    let host = req.uri().host().expect("URI has no host").to_string();
    format!("{}:{}", host, port)
}

/// Create a valid HTTP message from a Request
///
/// This will build the request line, headers, and body (if any), and return a Vec<u8> that can be sent
///
/// Note: Rust uses UTF-8 as default string encodings, so Header/Values are encoded as UTF-8.
/// In most cases you're likely using ASCII-compatible characters, so this is fine, but you might run into
/// oddities if you start sending UTF-8 characters in your headers
fn create_http_message(req: &Request<Option<&[u8]>>) -> Result<Vec<u8>, RequestError> {
    let authority = get_authority(req);
    let path_and_query = req.uri().path_and_query().unwrap();

    let mut message: Vec<u8> = Vec::new();

    // GET /path HTTP/1.1
    message
        .extend_from_slice(format!("{} {} HTTP/1.1\r\n", req.method(), path_and_query).as_bytes());
    // Host: www.example.com
    message.extend_from_slice(format!("Host: {}\r\n", authority).as_bytes());

    // Other headers
    for (name, value) in req.headers() {
        let str_value = value.to_str()?;
        message.extend_from_slice(format!("{}: {}\r\n", name, str_value).as_bytes());
    }

    // Empty line
    message.extend_from_slice(b"\r\n");

    // Body
    let body = req.body();

    if body.is_some() {
        message.extend_from_slice(body.unwrap());
    }

    Ok(message)
}
