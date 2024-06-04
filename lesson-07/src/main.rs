use std::{env, thread};
use std::error::Error;
use std::io::Read;
use std::process::exit;
use std::sync::mpsc;

use crate::output::handle_err;
use crate::parallel::{input_thread, output_thread};

mod error;
mod format;
mod output;
mod input;
mod parallel;

fn main() {
    let args: Vec<_> = env::args().collect();

    if args.len() > 1 {
        // Old format, expects data passed in one line.
        let format_type = args[1].as_str();
        let input = input::read_input();


        match input {
            Ok(input) => {
                let mut user_input = input.clone();
                let result = format::format(format_type, &mut user_input);
                output::handle_result(result);
            }
            Err(e) => { handle_err(e) }
        }
        exit(2)
    }
    let (sender, receiver) = mpsc::channel();

    let output_thread = thread::spawn(move || output_thread(receiver));
    let input_thread = thread::spawn(move || input_thread(sender));

    output_thread.join().unwrap();
    input_thread.join().unwrap();
}
