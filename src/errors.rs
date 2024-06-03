//! エラー処理を楽にする用

use crate::protocol::ParseMessageError;
use std::fmt::{Display, Formatter, Result};

/// エラー寄せ集め
#[derive(Debug)]
pub enum Errors {
    /// サーバーからのメッセージのパースに失敗したとき
    ParseMessage(ParseMessageError),
    /// JSONのパースエラー
    Serde(serde_json::Error),
    /// その他
    Other(&'static str),
}

use Errors::{Other, ParseMessage, Serde};

impl Display for Errors {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            ParseMessage(e) => write!(f, "{e}"),
            Serde(e) => write!(f, "{e}"),
            Other(e) => write!(f, "{e}"),
        }
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

impl From<&'static str> for Errors {
    fn from(value: &'static str) -> Self {
        Self::Other(value)
    }
}
