use std::error::Error;

use csv::Reader;
use is_vowel::IsRomanceVowel;

pub(crate) fn lowercase(input: &mut String) -> Result<String, Box<dyn Error>> {
    Ok(input.to_lowercase())
}

pub(crate) fn uppercase(input: &mut String) -> Result<String, Box<dyn Error>> {
    Ok(input.to_uppercase())
}

pub(crate) fn consonants(input: &mut String) -> Result<String, Box<dyn Error>> {
    let mut new_str = String::new();

    for char in input.chars() {
        if !char.is_romance_vowel() {
            new_str.push(char);
        }
    }
    Ok(new_str)
}

pub(crate) fn reverse(input: &mut String) -> Result<String, Box<dyn Error>> {
    Ok(input.chars().rev().collect::<String>())
}

pub(crate) fn no_spaces(input: &mut String) -> Result<String, Box<dyn Error>> {
    Ok(input.replace(" ", ""))
}

pub(crate) fn slugify(input: &mut String) -> Result<String, Box<dyn Error>> {
    Ok(slug::slugify(input))
}

pub(crate) fn format_csv(file: &mut String) -> Result<String, Box<dyn Error>> {
    let mut reader = Reader::from_path(file)?;

    let mut buffer = String::new();

    let headers: Vec<_> = reader
        .headers()?
        .into_iter()
        .map(|field| field.to_string().replace(" ", ""))
        .collect();

    let mut all_rows = vec![headers];

    for record in reader.records() {
        all_rows.push(
            record?
                .into_iter()
                .map(|field| field.to_string())
                .collect::<Vec<_>>()
                .to_owned(),
        )
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
