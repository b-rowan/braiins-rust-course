fn main() {
    let message = String::from("Hello, world!");
    let message_ref = &message;

    println!("{}", message_ref);
    println!("{}", message_ref.to_uppercase());
    println!("{}", message_ref.to_lowercase());
    println!("{}", message_ref.replace("Hello", "Goodbye"));
}
