use std::{
    io::{BufReader, prelude::*},
    net::{TcpStream, ToSocketAddrs},
};

use http::{
    header::HeaderName,
    Request, Response,
};

pub fn http_request(req: Request<String>) -> Response<String> {
    let port = req.uri().port_u16().unwrap_or(80);
    let host = req.uri().host().unwrap().to_string();
    let authority = format!("{}:{}", host, port);

    let address = authority.to_socket_addrs().unwrap().next().unwrap();
    let mut stream = TcpStream::connect(address).unwrap();

    let mut request: Vec<String> = vec![
        // GET /path HTTP/1.1
        format!("{} {} HTTP/1.1", req.method(), req.uri().path_and_query().unwrap()),
        // Host: www.example.com
        format!("Host: {}", req.uri().authority().unwrap()),
    ];

    // Other headers
    for (name, value) in req.headers() {
        request.push(format!("{}: {}", name, value.to_str().unwrap()));
    }

    // Empty line
    request.push("\r\n".to_string());

    // Body
    let body = req.body();

    if !body.is_empty() {
        request.push(body.to_string());
    }

    // Send request
    stream.write_all(request.join("\r\n").as_bytes()).unwrap();

    // Then read response
    let buf_reader = BufReader::new(&stream);
    let mut http_response = buf_reader
        .lines()
        .map(|line| line.unwrap());

    // Length of body in bytes
    let mut content_length = 0;
    let mut parsed_length = 0;
    let mut parsing_headers = true;
    let mut body: Vec<String> = vec![];

    // "HTTP/" 1*DIGIT "." 1*DIGIT SP 3DIGIT SP
    // > HTTP/1.1 200 OK
    // Key: Value
    // > Content-Length: 123

    // ["HTTP/1.1", "200", "OK"]
    let status_line: Vec<String> = http_response
        .next()
        .expect("No input received")
        .split(' ')
        .map(|s| s.to_string())
        .collect();

    let status_code = status_line[1].parse::<u16>().unwrap();
    let mut response_builder = Response::builder();
    let response_headers = response_builder.headers_mut().unwrap();

    for line in http_response {
        if line.is_empty() {
            parsing_headers = false;
            continue;
        }

        if parsing_headers {
            // Parse headers
            let (key, value) = line.split_once(':')
                .map(|(key, value)| (key.to_lowercase().trim().to_string(), value.trim().to_string()))
                .expect("Invalid header, no ':' to split on");

            response_headers.insert(key.parse::<HeaderName>().unwrap(), value.parse().unwrap());

            if key == "content-length" {
                content_length = value.parse().unwrap();
            }
        } else {
            // Parse body
            // +1 for the newline
            parsed_length += line.len() + 1;
            body.push(line);
            if parsed_length >= content_length {
                break;
            }
        }
    }

    response_builder
        .status(status_code)
        .body(body.join("\r\n"))
        .expect("Failed to construct response")
}
