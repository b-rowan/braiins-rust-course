# Lesson 16
#### A simple rust chat app


## Usage
This project runs a web server on the specified port and address (defaults to 0.0.0.0:11111), which can be accessed at the root to use the chat.

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

## Metrics
This server exposes metrics at `/metrics` for parsing by prometheus.

The metrics exposed are
- total messages sent,
- text messages sent,
- photos sent,
- files sent.