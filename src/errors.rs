use crate::protocol::ParseMessageError;
use std::{fmt::Display, io, num::ParseIntError};

#[derive(Debug)]
pub enum Errors {
    IO(io::Error),
    ParseMessage(ParseMessageError),
    ParseInt(ParseIntError),
    Serde(serde_json::Error),
    Other(&'static str),
}

use Errors::*;

impl Display for Errors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const ERROR_MESSAGE: &str = "エラー発生した:";
        match self {
            IO(e) => write!(f, "{} {}", ERROR_MESSAGE, e),
            ParseMessage(e) => write!(f, "{} {}", ERROR_MESSAGE, e),
            ParseInt(e) => write!(f, "{} {}", ERROR_MESSAGE, e),
            Serde(e) => write!(f, "{} {}", ERROR_MESSAGE, e),
            Other(e) => write!(f, "{} {}", ERROR_MESSAGE, e),
        }
    }
}

impl From<io::Error> for Errors {
    fn from(value: io::Error) -> Self {
        IO(value)
    }
}

impl From<ParseMessageError> for Errors {
    fn from(value: ParseMessageError) -> Self {
        ParseMessage(value)
    }
}

impl From<serde_json::Error> for Errors {
    fn from(value: serde_json::Error) -> Self {
        Serde(value)
    }
}

impl From<ParseIntError> for Errors {
    fn from(value: ParseIntError) -> Self {
        ParseInt(value)
    }
}

impl From<&'static str> for Errors {
    fn from(value: &'static str) -> Self {
        Self::Other(value)
    }
}
