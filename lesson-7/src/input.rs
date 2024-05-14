use std::error::Error;
use std::io;

pub(crate) fn read_input() -> Result<String, Box<dyn Error>> {
    println!("Please enter the string or file to format:");
    let mut user_input = String::new();
    io::stdin().read_line(&mut user_input)?;
    Ok(user_input.trim().to_string())
}

