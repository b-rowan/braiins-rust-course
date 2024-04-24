fn main() {
    let message = String::from("Hello, world!");
    let message_ref = &message;

    println!("{}", *message_ref)
}
