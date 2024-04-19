use std::{error, fmt::Display, io, num::ParseIntError};
use crate::protocol::{InvalidPlayId, ParseMessageError};

#[derive(Debug)]
pub enum  Errors {
    IO(io::Error),
    MessageParse(ParseMessageError),
    ParseInt(ParseIntError),
    Serde(serde_json::Error),
    InvalidPlayId(InvalidPlayId)
}

use Errors::*;

// どう考えても見ずらい表示するはずなので後で変える
impl Display for Errors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<io::Error> for Errors {
    fn from(value: io::Error) -> Self {
        IO(value)
    }
}

impl From<ParseMessageError> for Errors {
    fn from(value: ParseMessageError) -> Self {
        MessageParse(value)
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

impl From<InvalidPlayId> for Errors {
    fn from(value: InvalidPlayId) -> Self {
        InvalidPlayId(value)
    }
}
