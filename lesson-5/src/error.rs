use std::error;
use std::fmt;

#[derive(Debug, Clone)]
pub struct InvalidFormatType(pub String);

impl fmt::Display for InvalidFormatType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid formatting type: {}", self.0)
    }
}

impl error::Error for InvalidFormatType {}

#[derive(Debug, Clone)]
pub struct NoFormatPassed;

impl fmt::Display for NoFormatPassed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "no formatting passed")
    }
}

impl error::Error for NoFormatPassed {}
