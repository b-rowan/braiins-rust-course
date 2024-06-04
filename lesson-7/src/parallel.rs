use std::{io, thread};
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;

use crate::error::{InvalidInput, NoFormatPassed};
use crate::format;
use crate::output::{handle_err, handle_result};

pub(crate) fn input_thread(sender: Sender<String>) {
    loop {
        let mut user_input = String::new();
        println!("Please enter the format and string or file to format (<format> <data>):");
        let _ = io::stdin().read_line(&mut user_input);
        sender.send(user_input).unwrap();
        thread::sleep(Duration::from_millis(500));
    }
}

pub(crate) fn output_thread(receiver: Receiver<String>) {
    loop {
        let received =  receiver.recv().unwrap();

        let mut parts = received.splitn(2, ' ');
        match (parts.next(), parts.next()) {
            (Some(fmt), Some(data)) => {
                let mut input = String::from(data);
                let result = format::format(fmt, &mut input);
                handle_result(result);
            },
            (None, _) => {
                handle_err(NoFormatPassed.into())
            },
            _ => {
                handle_err(InvalidInput.into())
            }
        }
    }
}