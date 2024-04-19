use std::{error::Error, fmt::Display};

use serde::{Deserialize, Serialize};
use serde_aux::prelude::*;
use serde_json::Value;

use crate::errors::Errors;

//ゲーム中に繰り返し受信されるJSON達
// ConnectionStartとNameReceivedは最初しか来ないので除外
pub enum Messages {
    BoardInfo(BoardInfo),
    HandInfo(HandInfo),
    DoPlay(DoPlay),
    RoundEnd(RoundEnd),
    GameEnd(GameEnd),
    Played(Played),
}

#[derive(Debug)]
pub struct ParseMessageError {
    invalid_info: String,
}

impl Display for ParseMessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "MessageParseError, json is {}", self.invalid_info)
    }
}

impl Error for ParseMessageError {}

impl Messages {
    pub fn parse(json: &str) -> Result<Messages, Errors> {
        let obj = serde_json::from_str::<Value>(json)?;
        let typ = obj
            .get("Type")
            .expect("Typeキー無し")
            .as_str()
            .expect("Typeが文字列ではない");
        match typ {
            "BoardInfo" => Ok(Self::BoardInfo(serde_json::from_value(obj)?)),
            "HandInfo" => Ok(Self::HandInfo(serde_json::from_value(obj)?)),
            "DoPlay" => Ok(Self::DoPlay(serde_json::from_value(obj)?)),
            "RoundEnd" => Ok(Self::RoundEnd(serde_json::from_value(obj)?)),
            "GameEnd" => Ok(Self::GameEnd(serde_json::from_value(obj)?)),
            "Played"=> Ok(Self::Played(serde_json::from_value(obj)?)),
            _ => Err(ParseMessageError {
                invalid_info: json.to_string(),
            })?,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct ConnectionStart {
    #[serde(rename = "Type")]
    typ: String,
    #[serde(rename = "From")]
    from: String,
    #[serde(rename = "To")]
    to: String,
    #[serde(
        rename = "ClientID",
        deserialize_with = "deserialize_number_from_string"
    )]
    pub client_id: u8,
}

#[derive(Serialize)]
pub struct PlayerName {
    #[serde(rename = "Type")]
    pub typ: String,
    #[serde(rename = "From")]
    pub from: String,
    #[serde(rename = "To")]
    pub to: String,
    #[serde(rename = "Name")]
    pub name: String,
}

impl PlayerName {
    pub fn new(name: String) -> Self {
        PlayerName {
            typ: "PlayerName".to_string(),
            from: "Client".to_string(),
            to: "Server".to_string(),
            name,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct NameReceived {
    #[serde(rename = "Type")]
    typ: String,
    #[serde(rename = "From")]
    from: String,
    #[serde(rename = "To")]
    to: String,
}

#[derive(Deserialize, Debug)]
pub struct BoardInfo {
    #[serde(rename = "Type")]
    typ: String,
    #[serde(rename = "From")]
    from: String,
    #[serde(rename = "To")]
    to: String,
    #[serde(
        rename = "PlayerPosition_0",
        deserialize_with = "deserialize_number_from_string"
    )]
    pub player_position_0: u8,
    #[serde(
        rename = "PlayerPosition_1",
        deserialize_with = "deserialize_number_from_string"
    )]
    pub player_position_1: u8,
    #[serde(
        rename = "PlayerScore_0",
        deserialize_with = "deserialize_number_from_string"
    )]
    pub player_score_0: u8,
    #[serde(
        rename = "PlayerScore_1",
        deserialize_with = "deserialize_number_from_string"
    )]
    pub player_score_1: u8,
    #[serde(
        rename = "NumofDeck",
        deserialize_with = "deserialize_number_from_string"
    )]
    pub num_of_deck: u8,
    #[serde(
        rename = "CurrentPlayer",
        deserialize_with = "deserialize_number_from_string"
    )]
    pub current_player: u8,
}

impl BoardInfo {
    pub fn new() -> Self {
        BoardInfo {
            typ: String::new(),
            from: String::new(),
            to: String::new(),
            player_position_0: 0,
            player_position_1: 23,
            player_score_0: 0,
            player_score_1: 0,
            num_of_deck: 15,
            current_player: 0,
        }
    }
}

#[derive(Deserialize)]
pub struct HandInfo {
    #[serde(rename = "Type")]
    typ: String,
    #[serde(rename = "From")]
    from: String,
    #[serde(rename = "To")]
    to: String,
    #[serde(rename = "Hand1", deserialize_with = "deserialize_number_from_string")]
    pub hand1: u8,
    #[serde(rename = "Hand2", deserialize_with = "deserialize_number_from_string")]
    pub hand2: u8,
    #[serde(rename = "Hand3", deserialize_with = "deserialize_number_from_string")]
    pub hand3: u8,
    #[serde(rename = "Hand4", deserialize_with = "deserialize_number_from_string")]
    pub hand4: u8,
    #[serde(rename = "Hand5", deserialize_with = "deserialize_number_from_string")]
    pub hand5: u8,
}

impl HandInfo {
    pub fn new() -> Self {
        HandInfo {
            typ: String::from("HandInfo"),
            from: String::from("Server"),
            to: String::from("Client"),
            hand1: 0,
            hand2: 0,
            hand3: 0,
            hand4: 0,
            hand5: 0,
        }
    }

    pub fn to_vec(&self) -> Vec<u8> {
        vec![self.hand1, self.hand2, self.hand3, self.hand4, self.hand5]
    }
}

#[derive(Deserialize)]
pub struct DoPlay {
    #[serde(rename = "Type")]
    typ: String,
    #[serde(rename = "From")]
    from: String,
    #[serde(rename = "To")]
    to: String,
    #[serde(
        rename = "MessageID",
        deserialize_with = "deserialize_number_from_string"
    )]
    pub message_id: u8,
    message: String,
}

pub enum RequestedPlay {
    NormalTurn,
    Parry,
}

#[derive(Debug)]
pub struct InvalidPlayId;

impl Display for InvalidPlayId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "InvalidPlayId")
    }
}

impl Error for InvalidPlayId {}

impl RequestedPlay {
    pub fn from_id(message_id: u8) -> Result<Self, InvalidPlayId> {
        match message_id {
            101 => Ok(Self::NormalTurn),
            102 => Ok(Self::Parry),
            _ => Err(InvalidPlayId),
        }
    }
}

#[derive(Serialize)]
pub struct Evaluation {
    #[serde(rename = "Type")]
    pub typ: String,
    #[serde(rename = "From")]
    pub from: String,
    #[serde(rename = "To")]
    pub to: String,
    #[serde(rename = "1F")]
    pub eval_1f: Option<String>,
    #[serde(rename = "1B")]
    pub eval_1b: Option<String>,
    #[serde(rename = "2F")]
    pub eval_2f: Option<String>,
    #[serde(rename = "2B")]
    pub eval_2b: Option<String>,
    #[serde(rename = "3F")]
    pub eval_3f: Option<String>,
    #[serde(rename = "3B")]
    pub eval_3b: Option<String>,
    #[serde(rename = "4F")]
    pub eval_4f: Option<String>,
    #[serde(rename = "4B")]
    pub eval_4b: Option<String>,
    #[serde(rename = "5F")]
    pub eval_5f: Option<String>,
    #[serde(rename = "5B")]
    pub eval_5b: Option<String>,
}

#[derive(Serialize)]
pub struct PlayMovement {
    #[serde(rename = "Type")]
    pub typ: String,
    #[serde(rename = "From")]
    pub from: String,
    #[serde(rename = "To")]
    pub to: String,
    #[serde(rename = "MessageID")]
    pub message_id: String,
    #[serde(rename = "PlayCard")]
    pub play_card: String,
    #[serde(rename = "Direction")]
    pub direction: String,
}

#[derive(Serialize)]
pub struct PlayAttack {
    #[serde(rename = "Type")]
    pub typ: String,
    #[serde(rename = "From")]
    pub from: String,
    #[serde(rename = "To")]
    pub to: String,
    #[serde(rename = "MessageID")]
    pub message_id: String,
    #[serde(rename = "PlayCard")]
    pub play_card: String,
    #[serde(rename = "NumOfCard")]
    pub num_of_card: String,
}

#[derive(Serialize)]
pub struct PlayParry {
    #[serde(rename = "Type")]
    pub typ: String,
    #[serde(rename = "From")]
    pub from: String,
    #[serde(rename = "To")]
    pub to: String,
    #[serde(rename = "MessageID")]
    pub message_id: String,
    #[serde(rename = "PlayCard")]
    pub play_card: String,
    #[serde(rename = "NumOfCard")]
    pub num_of_card: String,
}

#[derive(Deserialize)]
pub struct RoundEnd {
    #[serde(rename = "Type")]
    typ: String,
    #[serde(rename = "From")]
    from: String,
    #[serde(rename = "To")]
    to: String,
    #[serde(rename = "Winner", deserialize_with = "deserialize_number_from_string")]
    pub winner: u8,
    #[serde(rename = "Score0", deserialize_with = "deserialize_number_from_string")]
    pub score_0: u8,
    #[serde(rename = "Score1", deserialize_with = "deserialize_number_from_string")]
    pub score_1: u8,
    #[serde(rename = "Message")]
    pub message: String,
}

#[derive(Deserialize)]
pub struct GameEnd {
    #[serde(rename = "Type")]
    typ: String,
    #[serde(rename = "From")]
    from: String,
    #[serde(rename = "To")]
    to: String,
    #[serde(rename = "Winner", deserialize_with = "deserialize_number_from_string")]
    pub winner: u8,
    #[serde(rename = "Score0", deserialize_with = "deserialize_number_from_string")]
    pub score_0: u8,
    #[serde(rename = "Score1", deserialize_with = "deserialize_number_from_string")]
    pub score_1: u8,
    #[serde(rename = "Message")]
    pub message: String,
}


#[derive(Deserialize)]
pub struct Played {
    #[serde(rename = "Type")]
    pub typ: String,
    #[serde(rename = "From")]
    pub from: String,
    #[serde(rename = "To")]
    pub to: String,
    #[serde(rename = "MessageID")]
    pub message_id: String,
    #[serde(rename = "PlayCard")]
    pub play_card: String,
    #[serde(rename = "Direction")]
    pub direction: String,
}