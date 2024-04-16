# Introduction

This is a simple HTTPS server, that logs the HTTP requests it receives to stdout. It answers with predefined content/status code combinations.
On startup the server generates a self-signed cert.

The server is intended for identifying differences in behavior of different HTTP clients.

## How to run

Options, files and status codes can be repeated, they will be zipped to create the response combinations:

```
Usage: http_headers_print [OPTIONS]

Options:
      --hostname <HOSTNAME>  [default: localhost]
      --files <FILES>
      --status <STATUS>
      --port <PORT>          [default: 8080]
  -h, --help                 Print help
```

```sh
cargo run --release -- {Options}
```
