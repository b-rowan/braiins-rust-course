use std::env;
use std::io;
use std::process;

mod format;

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

fn read_input() -> String{
    println!("Please enter the string to format:");
    let mut user_input = String::new();
    io::stdin().read_line(& mut user_input).expect("Invalid string.");

    user_input
}

fn validate_args(args: Vec<String>) -> String {
        if args.len() <= 1 {
        println!("No formatting argument passed.");
        print_error();
        process::exit(2);
    }
    let format_type = args[1].as_str();

    if !AVAILABLE_ARGS.contains(&format_type) {
        println!("Invalid formatting argument: {}", args[1]);
        print_error();
        process::exit(2);
    }

    format_type.to_owned()
}


fn main() {
    let args: Vec<String> = env::args().collect();

    let format_type = validate_args(args);

    let mut user_input = read_input();

    match format_type.as_str() {
        "lowercase" => {
            println!("{}", format::lowercase(&mut user_input));
        }
        "uppercase" => {
            println!("{}", format::uppercase(&mut user_input));
        }
        "consonants" => {
            // Technically doesn't work with accented vowels, such as á
            println!("{}", format::consonants(&mut user_input));
        }
        "no-spaces" => {
            println!("{}", format::no_spaces(&mut user_input));
        }
        "reverse" => {
            // Doesn't reverse properly with accented characters either
            println!("{}", format::reverse(&mut user_input));
        }
        "slugify" => {
            println!("{}", format::slugify(&mut user_input));
        }
        &_ => {
            panic!("Should not be possible to reach.")
        }
    }
}
