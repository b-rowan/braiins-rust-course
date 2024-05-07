pub(crate) fn lowercase(input: &mut String) -> String {
    input.to_lowercase()
}

pub(crate) fn uppercase(input: &mut String) -> String {
    input.to_uppercase()
}

pub(crate) fn consonants(input: &mut String) -> String {
    let mut new_str = String::new();

    for char in input.chars() {
        if !["a", "e", "i", "o", "u"].contains(&&char.to_string()[..]) {
            new_str.push(char);
        }
    }
    new_str
}

pub(crate) fn reverse(input: &mut String) -> String {
    input.chars().rev().collect::<String>()
}

pub(crate) fn no_spaces(input: &mut String) -> String {
    input.replace(" ", "")
}

pub(crate) fn slugify(input: &mut String) -> String {
    slug::slugify(input)
}

pub(crate) fn table(input: &mut String) -> String {
    todo!()
}
