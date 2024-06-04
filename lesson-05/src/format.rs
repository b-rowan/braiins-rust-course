use std::collections::HashMap;
use std::error::Error;
use std::io::Stdin;

use csv::Reader;
use is_vowel::IsRomanceVowel;
use lazy_static::lazy_static;
use slug;

use crate::error::InvalidFormatType;


type FormattingResult = Result<String, Box<dyn Error>>;
type FormattingFunction = fn(&mut String) -> FormattingResult;

lazy_static! {
    static ref FORMAT_FUNCTIONS: HashMap<&'static str, FormattingFunction> = HashMap::from([
        ("lowercase", lowercase as FormattingFunction),
        ("uppercase", uppercase as FormattingFunction),
        ("consonants", consonants as FormattingFunction),
        ("reverse", reverse as FormattingFunction),
        ("no-spaces", no_spaces as FormattingFunction),
        ("slugify", slugify as FormattingFunction),
    ]);
}

pub(crate) fn format_string(format: &str, input: &mut String) -> FormattingResult {
    let format_fn = FORMAT_FUNCTIONS.get(format).map(|x| *x);

    format_fn
        .ok_or_else(|| InvalidFormatType(String::from(format)).into())
        .and_then(|func| func(input))
}

pub(crate) fn lowercase(input: &mut String) -> FormattingResult {
    Ok(input.to_lowercase())
}

pub(crate) fn uppercase(input: &mut String) -> FormattingResult {
    Ok(input.to_uppercase())
}

pub(crate) fn consonants(input: &mut String) -> FormattingResult {
    let mut new_str = String::new();

    for char in input.chars() {
        if !char.is_romance_vowel() {
            new_str.push(char);
        }
    }
    Ok(new_str)
}

pub(crate) fn reverse(input: &mut String) -> FormattingResult {
    Ok(input.chars().rev().collect::<String>())
}

pub(crate) fn no_spaces(input: &mut String) -> FormattingResult {
    Ok(input.replace(" ", ""))
}

pub(crate) fn slugify(input: &mut String) -> FormattingResult {
    Ok(slug::slugify(input))
}

pub(crate) fn table(mut input: Reader<Stdin>) -> Result<String, Box<dyn Error>> {
    let mut buffer = String::new();

    let headers: Vec<_> = input.headers()?.into_iter().map(|field| field.to_string().replace(" ", "")).collect();

    let mut all_rows = vec![headers];

    for record in input.records() {
        all_rows.push(record?.into_iter().map(|field| field.to_string()).collect::<Vec<_>>().to_owned())
    }

    let mut max_widths: Vec<usize> = vec![0; all_rows[0].len()];

    for row in &all_rows {
        for i in 0..max_widths.len() {
            if row[i].len() > max_widths[i] {
                max_widths[i] = row[i].len();
            }
        }
    }


    for row in all_rows {
        buffer.push_str("|");
        for (i, cell) in row.iter().enumerate() {
            let width = max_widths[i];
            buffer.push_str(&format!(" {:<width$} |", cell));
        }
        buffer.push_str("\n|");

        // Separator line
        for width in &max_widths {
            buffer.push_str(&format!("-{:-<width$}-|", ""));
        }
        buffer.push_str("\n");
    }
    Ok(buffer)
}
