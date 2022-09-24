use std::{
    io::{BufReader, prelude::*},
    net::{TcpStream, ToSocketAddrs, SocketAddr}, str::from_utf8,
};

use http::{
    header::HeaderName,
    Request, Response, HeaderValue,
};

// TODO: better error type...
/// Execute an HTTP 1.1 request, then parse the response
/// This will build the request line, headers, and body (if any), then send it to the server
///
/// Note: if the server returns an incorrect content-length that's:
///   - too long: client will block until the tcp connection is closed
///   - too short: the returned body will be cut short
///   - not present: content-length defaults to 0, so no body is returned
pub fn http_request(req: Request<Option<&String>>)
    -> Result<Response<Vec<u8>>, Box<dyn std::error::Error>> {
    let port = req.uri().port_u16().unwrap_or(80);
    let host = req.uri().host().expect("URI has no host").to_string();
    let authority = format!("{}:{}", host, port);

    let addresses: Vec<SocketAddr> = authority
        .to_socket_addrs()?
        .collect();

    let mut stream = TcpStream::connect(addresses.as_slice())?;

    let path_and_query = req
        .uri()
        .path_and_query()
        .unwrap();

    let mut request: Vec<String> = vec![
        // GET /path HTTP/1.1
        format!("{} {} HTTP/1.1", req.method(), path_and_query),
        // Host: www.example.com
        format!("Host: {}", authority),
    ];

    // Other headers
    for (name, value) in req.headers() {
        let str_value = value.to_str()?;
        request.push(format!("{}: {}", name, str_value));
    }

    // Empty line
    request.push("\r\n".to_string());

    // Body
    let body = req.body();

    if body.is_some() {
        request.push(body.unwrap().to_owned());
    }

    // Send request
    stream.write_all(
        request.join("\r\n").as_bytes()
    )?;

    // Then read response
    let buf_reader = BufReader::new(&stream);

    let mut status_code: Option<u16> = None;
    // Length of body in bytes (from 'Content-Length' header)
    let mut content_length = 0;
    // The body we've received
    let mut body: Vec<u8> = vec![];

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
            // We've reached the end of the HTTP response
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
                header_value.parse::<HeaderValue>()?
            );
        }
    }

    if status_code.is_none() {
        return Err("No status code found".into());
    }

    // Parse the body, reading bytes until we meet content-length
    loop {
        let byte = byte_iter.next().unwrap()?;
        body.push(byte);
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
