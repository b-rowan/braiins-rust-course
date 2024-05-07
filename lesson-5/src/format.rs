use std::collections::HashMap;
use std::error::Error;

const FORMAT_FUNCTIONS: HashMap<&str, fn(&mut String) -> Result<String,  Box<dyn Error>>> = HashMap::from([
    ("lowercase", lowercase),
    ("uppercase", uppercase),
    ("consonants", consonants),
    ("reverse", reverse),
    ("no-spaces", no_spaces),
    ("slugify", slugify),
]);


pub(crate) fn format_string(format: &str,  input: &mut String) -> Result<String,  Box<dyn std::error::Error>> {
    let format_fn = FORMAT_FUNCTIONS.get(format).map(|x| *x);

    return if format_fn.is_some() {
        format_fn.unwrap()(input)
    } else {
        todo!()
    }
}


pub(crate) fn lowercase(input: &mut String)  -> Result<String,  Box<dyn Error>> {
    Ok(input.to_lowercase())
}

pub(crate) fn uppercase(input: &mut String)  -> Result<String,  Box<dyn Error>> {
    Ok(input.to_uppercase())
}

pub(crate) fn consonants(input: &mut String)  -> Result<String,  Box<dyn Error>> {
    let mut new_str = String::new();

    for char in input.chars() {
        if !["a", "e", "i", "o", "u"].contains(&&char.to_string()[..]) {
            new_str.push(char);
        }
    }
    Ok(new_str)
}

pub(crate) fn reverse(input: &mut String)  -> Result<String,  Box<dyn Error>> {
    Ok(input.chars().rev().collect::<String>())
}

pub(crate) fn no_spaces(input: &mut String)  -> Result<String,  Box<dyn Error>> {
    Ok(input.replace(" ", ""))
}

pub(crate) fn slugify(input: &mut String)  -> Result<String,  Box<dyn Error>> {
    Ok(slug::slugify(input))
}

pub(crate) fn table(input: &mut String)  -> Result<String,  Box<dyn Error>> {
    todo!()
}
