use std::env;
use std::error::Error;
use std::io;
use std::process;
use crate::error::NoFormatPassed;

mod error;
mod format;

fn read_input() -> Result<String, Box<dyn Error>> {
    println!("Please enter the string to format:");
    let mut user_input = String::new();
    io::stdin().read_line(&mut user_input)?;
    Ok(user_input)
}

fn validate_args(args: Vec<String>) -> Result<String, Box<dyn Error>> {
    if args.len() <= 1 {
        return Err(NoFormatPassed.into());
    }
    let format_type = args[1].as_str();

    Ok(format_type.to_owned())
}

fn handle_err(error: Box<dyn Error>) {
    let err_val = error.to_string();
    eprintln!("{}", err_val);
    process::exit(2);
}

fn main() {
    let args = env::args().collect();

    let format_type = validate_args(args).unwrap_or_else(|e| {
        handle_err(e);
        String::new()
    });

    let mut user_input = read_input().unwrap_or_else(|e| {
        handle_err(e);
        String::new()
    });


    let result = format::format_string(&format_type.to_string(), &mut user_input).unwrap_or_else(|e| {
        handle_err(e);
        String::new()
    });


    println!("Result: {result}")
}
