# Lesson 15 - Library
#### A simple rust chat app

## Usage
The main item provided by this library is the `Message` type, which handles parsing messages to and from the server/client.
The `Message` implements the `FromStr` trait, allowing it to be easily parsed from user input.
The intended usage is as follows:

```rust
use rust_chat::Message;

fn main() {
    // fake user input
    let user_input_text = String::from("hello world");
    
    assert!(Message::from_string(user_input_text) == Message::Text(user_input_text));
}
```

## Development
This library is shared by the server and client, and handles all the message parsing.