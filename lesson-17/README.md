# Lesson 15
#### A simple rust chat app


## Usage
There are 3 parts to this project.  Click on each link to learn more about using each.
- [Server](./server/README.md)
- [Client](./client/README.md)
- [Library](./rust_chat/README.md)

## Dependencies
This project uses a number of dependencies to make development easier.
- `tracing`/`tracing_subscriber`
  - `tracing` is used for logging, and handles logging events.  `tracing_subscriber` grabs these events, and puts them into a log file.
- `clap`
  - `clap` is used for command line argument parsing, and helps auto generate cli help.
- `serde`/`serde_cbor`
  - `serde` is the serialization and deserialization base layer for this project, while `serde_cbor` handles the specifics of converting the messages into a low level protocol.
- `image`
  - `image` is used to make image parsing easier, and handles auto conversion of photos into .png format.
- `parking_lot`
  - `parking_lot` is used for its helpful synchronization structures.
- `thiserror`
  - `thiserror` is used to simplify error creation.

## Error Handling
The approach to error handling in this project is quite different on the server VS the client.
The server should nearly never panic, except on initial startup, such as when trying to bind to the listen port.
The server will also close the connection with the client on any client related errors.
The client on the other hand, will panic on most errors, as most errors on the client side are unrecoverable.
In these cases, the client will display an error message about what the problem was, then exit.