use std::error::Error;

pub(crate) fn handle_err(error: Box<dyn Error>) {
    let err_val = error.to_string();
    eprintln!("{}", err_val);
}

pub(crate) fn handle_result(result: Result<String, Box<dyn Error>>) {
    match result {
        Ok(res) => {
            println!("Result:");
            println!("{}", res);
        }
        Err(e) => {handle_err(e);}
    }
}
