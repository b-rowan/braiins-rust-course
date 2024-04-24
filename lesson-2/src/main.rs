use std::env;
use std::process;
use std::io;
use slug::slugify;

const AVAILABLE_ARGS: [&str; 6] = [
    "lowercase",
    "uppercase",
    "consonants",
    "reverse",
    "no-spaces",
    "slugify",
];

fn print_error() {
    println!("Please pass one of:");
    for arg in AVAILABLE_ARGS {
        println!("{arg}");
    }
}

trait Consonants {
    fn consonants(&self) -> Self;
}

impl Consonants for String {
    fn consonants(&self) -> Self {
        let mut new_str = String::new();

        for char in self.chars() {
            if !["a", "e", "i", "o", "u"].contains(&&char.to_string()[..]) {
                new_str.push(char);
            }
        }
        new_str
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        println!("No formatting argument passed.");
        print_error();
        process::exit(2);
    }
    let format_type = &args[1][..];

    if !AVAILABLE_ARGS.contains(&&format_type) {
        println!("Invalid formatting argument: {}", args[1]);
        print_error();
        process::exit(2);
    }

    println!("Please enter the string to format:");
    let mut user_input = String::new();
    io::stdin().read_line(& mut user_input).expect("Invalid string.");

    match format_type {
        "lowercase" => {
            println!("{}", user_input.to_lowercase());
        }
        "uppercase" => {
            println!("{}", user_input.to_uppercase());
        }
        "consonants" => {
            // Technically doesn't work with accented vowels, such as á
            println!("{}", user_input.consonants());
        }
        "no-spaces" => {
            println!("{}", user_input.replace(" ", ""));
        }
        "reverse" => {
            // Doesn't reverse properly with accented characters either
            println!("{}", user_input.chars().rev().collect::<String>());
        }
        "slugify" => {
            // Doesn't reverse properly with accented characters either
            println!("{}", slugify(user_input));
        }
        &_ => {
            panic!("Should not be possible to reach.")
        }
    }
}
