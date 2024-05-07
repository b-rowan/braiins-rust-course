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

    let result = format::format_string(&format_type.to_string(), &mut user_input);
    dbg!(result);
}
