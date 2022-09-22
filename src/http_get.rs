use http::{
    header::HeaderName,
    Request,
};

use crate::cli::CommonOptions;
use crate::request::http_request;

pub fn http_get(options: CommonOptions) {
    let mut request_builder = Request::builder()
        .method("GET")
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
        // TODO: this is a hack, we should be able to set the body to empty `()`
        .body("".to_string())
        .unwrap();

    let response = http_request(request);

    if options.verbose {
        println!("HTTP/1.1 {}", response.status());
        for (key, value) in response.headers() {
            println!("{}: {}", key, value.to_str().unwrap());
        }
        println!();
    }

    println!("{}", response.body());
}
