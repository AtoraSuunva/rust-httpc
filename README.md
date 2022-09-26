# rust-http

Implementing a CLI HTTP 1.1 client in Rust, using just Clap for argument parsing and Hyper for HTTP request/response
builders.

Nothing like building requests and parsing responses from scratch.

cURL-inspired, of course.

## Usage

why would you

```bash
# GET request (shows response body)
$ httpc get http://httpbin.org/get
# GET request but verbose (includes response headers)
$ httpc get -v http://httpbin.org/get
# POST request with data (Content-Length is automatically calculated and set, you only need to provide Content-Type)
$ httpc post -h 'Content-Type: application/json' -d '{"cool": 1}' http://httpbin.org/post
# POST request with data from a file
$ httpc post -h 'Content-Type: application/json' -f ./data.json http://httpbin.org/post
# GET request and save response body to a file
$ httpc get -o ./file.json http://httpbin.org/get
```

## Building

it's rust

```bash
$ cargo build
```

## Why?

School gave me this as an assignment. They suggested C, Go, Python, Java, or NodeJS. I wanted to learn Rust, so I get to
enjoy borrow-checking.
