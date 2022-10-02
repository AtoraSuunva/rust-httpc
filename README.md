# rust-http

Implementing a CLI HTTP 1.1 client from (mostly) scratch in Rust, using:

- Clap for argument parsing
- Hyper for HTTP request/response builders
- OpenSSL (via bindings) for HTTPS support

Building request messages and parsing response messages is all manually done for fun.

cURL-inspired, of course.

## Usage

why would you

```bash
# GET request (shows response body)
$ httpc get https://httpbin.org/get
# GET request but verbose (includes response headers)
$ httpc get -v https://httpbin.org/get
# POST request with data (Content-Length is automatically calculated and set, you only need to provide Content-Type)
$ httpc post -h 'Content-Type: application/json' -d '{"cool": 1}' https://httpbin.org/post
# POST request with data from a file
$ httpc post -h 'Content-Type: application/json' -f ./data.json https://httpbin.org/post
# GET request and save response body to a file
$ httpc get -o ./file.json https://httpbin.org/get
# GET request and follow redirects
$ httpc get -lv https://httpbin.org/redirect/3
```

## Building

it's rust

```bash
cargo build
```

## Why?

School gave me this as an assignment. They suggested C, Go, Python, Java, or NodeJS. I wanted to learn Rust, so I get to enjoy borrow-checking.

## TODO

- Reuse connection for following redirects? (map host -> connection?)
- color?
- `Transfer-Encoding: chunked`
- Return `Response<SomeKindOfStream>`? Display headers while the body loads (or even incrementally display body?)
- Progress bar? `-p`
