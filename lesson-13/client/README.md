# Lesson 13 - Client
#### A simple rust chat app

## Usage
`cargo run`

You can also run with arguments, the binary includes help information, just run `client --help` in the build directory.

## Development
All the message parsing is handled by the shared library `rust_chat`.

The client itself is a single file, which uses `async` features, using `tokio`.