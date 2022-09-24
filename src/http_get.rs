use std::str::from_utf8;

use http::header::CONTENT_TYPE;
use http::{
    header::HeaderName,
    Request,
    Method,
};

use crate::cli::CommonOptions;
use crate::request::http_request;

pub fn http_get(options: CommonOptions) {
    let mut request_builder = Request::builder()
        .method(Method::GET)
        .uri(options.url);

    let headers = request_builder.headers_mut().unwrap();

    for header in options.header {
        let (key, value) = header
            .split_once(':')
            .map(|(name, value)| (name.trim(), value.trim()))
            .expect("Invalid header format");

        headers.insert(key.parse::<HeaderName>().unwrap(), value.parse().unwrap());
    }

    let request = request_builder
        .body(None)
        .unwrap();

    let response = match http_request(request) {
        Ok(response) => response,
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    };

    if options.verbose {
        println!("HTTP/1.1 {}", response.status());
        for (key, value) in response.headers() {
            println!("{}: {}", key, value.to_str().unwrap());
        }
        println!();
    }

    match response.headers().get(CONTENT_TYPE) {
        Some(content_type) => {
            let content_type = content_type.to_str().unwrap();
            if content_type.starts_with("text/") || content_type == "application/json" {
                let body = response.into_body();
                let text = from_utf8(&body).unwrap();
                println!("{}", text);
            } else {
                println!("Binary data, not displaying.");
            }
        }
        None => {
            println!("No content type header, not displaying anything.");
        }
    }
}
