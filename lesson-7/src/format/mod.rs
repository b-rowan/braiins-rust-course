use std::error::Error;
use std::str::FromStr;

use slug;

use crate::error::InvalidFormatType;

mod func;

pub(crate) fn format(format: &str, input: &mut String) -> Result<String, Box<dyn Error>> {
    FormattingFunction::from_str(format)?.format(input)
}

enum FormattingFunction {
    Lowercase,
    Uppercase,
    Consonants,
    Reverse,
    NoSpaces,
    Slugify,
    CSV,
}

impl FromStr for FormattingFunction {
    type Err = InvalidFormatType;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "lowercase" => Ok(Self::Lowercase),
            "uppercase" => Ok(Self::Uppercase),
            "consonants" => Ok(Self::Consonants),
            "reverse" => Ok(Self::Reverse),
            "no_spaces" => Ok(Self::NoSpaces),
            "slugify" => Ok(Self::Slugify),
            "csv" => Ok(Self::CSV),
            _ => Err(InvalidFormatType(s.to_owned())),
        }
    }
}

impl FormattingFunction {
    fn format(&self, input: &mut String) -> Result<String, Box<dyn Error>> {
        match self {
            FormattingFunction::Lowercase => func::lowercase(input),
            FormattingFunction::Uppercase => func::uppercase(input),
            FormattingFunction::Consonants => func::consonants(input),
            FormattingFunction::Reverse => func::reverse(input),
            FormattingFunction::NoSpaces => func::no_spaces(input),
            FormattingFunction::Slugify => func::slugify(input),
            FormattingFunction::CSV => func::format_csv(input),
        }
    }
}
