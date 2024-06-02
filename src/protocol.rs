use std::{error::Error, fmt::Display};

use serde::{Deserialize, Serialize};
use serde_aux::prelude::*;
use serde_json::Value;
use serde_with::skip_serializing_none;

use crate::errors::Errors;

use crate::states::{Attack, Movement};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum PlayerID {
    Zero,
    One,
}

impl PlayerID {
    pub fn denote(&self) -> u8 {
        match self {
            PlayerID::Zero => 0,
            PlayerID::One => 1,
        }
    }
    pub fn from_u8(id: u8) -> Option<PlayerID> {
        match id {
            0 => Some(PlayerID::Zero),
            1 => Some(PlayerID::One),
            _ => None,
        }
    }
}

impl<'de> Deserialize<'de> for PlayerID {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct MyEnumVisitor;

        impl<'de> serde::de::Visitor<'de> for MyEnumVisitor {
            type Value = PlayerID;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("0 or 1 or Zero or One")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match v.trim() {
                    "0" => Ok(PlayerID::Zero),
                    "1" => Ok(PlayerID::One),
                    "Zero" => Ok(PlayerID::Zero),
                    "One" => Ok(PlayerID::One),
                    _ => Err(E::invalid_value(serde::de::Unexpected::Str(v), &self)),
                }
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                self.visit_str(v.as_str())
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match v {
                    0 => Ok(PlayerID::Zero),
                    1 => Ok(PlayerID::One),
                    _ => Err(E::invalid_value(serde::de::Unexpected::Unsigned(v), &self)),
                }
            }
        }

        deserializer.deserialize_any(MyEnumVisitor)
    }
}

/// カード番号を示す。
#[derive(Debug)]
pub enum CardID {
    /// 番号1
    One,
    /// 番号2
    Two,
    /// 番号3
    Three,
    /// 番号4
    Four,
    /// 番号5
    Five,
}

impl CardID {
    /// `u8`上の表現を返します
    pub fn denote(&self) -> u8 {
        use CardID::{Five, Four, One, Three, Two};
        match self {
            One => 1,
            Two => 2,
            Three => 3,
            Four => 4,
            Five => 5,
        }
    }

    /// `u8`から`CardId`を作成します
    pub fn from_u8(n: u8) -> Option<CardID> {
        use CardID::{Five, Four, One, Three, Two};
        match n {
            n @ (1..=5) => Some(match n {
                1 => One,
                2 => Two,
                3 => Three,
                4 => Four,
                5 => Five,
                _ => unreachable!(),
            }),
            _ => None,
        }
    }
}

#[derive(Deserialize, Debug)]
struct BoardInfoJson {
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
    player_position_0: u8,
    #[serde(
        rename = "PlayerPosition_1",
        deserialize_with = "deserialize_number_from_string"
    )]
    player_position_1: u8,
    #[serde(
        rename = "PlayerScore_0",
        deserialize_with = "deserialize_number_from_string"
    )]
    player_score_0: u32,
    #[serde(
        rename = "PlayerScore_1",
        deserialize_with = "deserialize_number_from_string"
    )]
    player_score_1: u32,
    #[serde(
        rename = "NumofDeck",
        deserialize_with = "deserialize_number_from_string"
    )]
    num_of_deck: u8,
    #[serde(rename = "CurrentPlayer", default)]
    current_player: Option<PlayerID>,
}

impl BoardInfoJson {
    fn p0_position(&self) -> u8 {
        self.player_position_0
    }

    fn p1_position(&self) -> u8 {
        self.player_position_1
    }

    fn p0_score(&self) -> u32 {
        self.player_score_0
    }

    fn p1_score(&self) -> u32 {
        self.player_score_1
    }

    fn num_of_deck(&self) -> u8 {
        self.num_of_deck
    }

    fn current_player(&self) -> Option<PlayerID> {
        self.current_player
    }
}

#[derive(Debug, Clone)]
pub struct BoardInfo {
    p0_position: u8,
    p1_position: u8,
    p0_score: u32,
    p1_score: u32,
    num_of_deck: u8,
    current_player: Option<PlayerID>,
}

impl BoardInfo {
    fn from_deserialized(info_json: &BoardInfoJson) -> Self {
        Self {
            p0_position: info_json.p0_position(),
            p1_position: info_json.p1_position(),
            p0_score: info_json.p0_score(),
            p1_score: info_json.p1_score(),
            num_of_deck: info_json.num_of_deck(),
            current_player: info_json.current_player(),
        }
    }

    pub fn p0_position(&self) -> u8 {
        self.p0_position
    }

    pub fn p1_position(&self) -> u8 {
        self.p1_position
    }

    pub fn distance_between_enemy(&self) -> u8 {
        self.p1_position - self.p0_position
    }

    pub fn new() -> Self {
        Self {
            p0_position: 0,
            p1_position: 23,
            p0_score: 0,
            p1_score: 0,
            num_of_deck: 25,
            current_player: Some(PlayerID::Zero),
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct HandInfo {
    #[serde(rename = "Type")]
    typ: String,
    #[serde(rename = "From")]
    from: String,
    #[serde(rename = "To")]
    to: String,
    #[serde(rename = "Hand1", deserialize_with = "deserialize_number_from_string")]
    hand1: u8,
    #[serde(rename = "Hand2", deserialize_with = "deserialize_number_from_string")]
    hand2: u8,
    #[serde(rename = "Hand3", deserialize_with = "deserialize_number_from_string")]
    hand3: u8,
    #[serde(
        rename = "Hand4",
        default,
        deserialize_with = "deserialize_option_number_from_string"
    )]
    hand4: Option<u8>,
    #[serde(
        rename = "Hand5",
        default,
        deserialize_with = "deserialize_option_number_from_string"
    )]
    hand5: Option<u8>,
}

impl HandInfo {
    pub fn to_vec(&self) -> Vec<u8> {
        match (self.hand4, self.hand5) {
            (Some(hand4), Some(hand5)) => vec![self.hand1, self.hand2, self.hand3, hand4, hand5],
            (Some(hand4), None) => vec![self.hand1, self.hand2, self.hand3, hand4],
            (None, Some(hand5)) => vec![self.hand1, self.hand2, self.hand3, hand5],
            (None, None) => vec![self.hand1, self.hand2, self.hand3],
        }
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
    message_id: u8,
    #[serde(rename = "Message")]
    message: String,
}

#[derive(Deserialize)]
pub struct Accept {
    #[serde(rename = "Type")]
    typ: String,
    #[serde(rename = "From")]
    from: String,
    #[serde(rename = "To")]
    to: String,
    #[serde(rename = "MessageID")]
    message_id: String,
}

#[derive(Deserialize, Debug)]
pub struct PlayedMoveMent {
    #[serde(rename = "Type")]
    typ: String,
    #[serde(rename = "From")]
    from: String,
    #[serde(rename = "To")]
    to: String,
    #[serde(rename = "MessageID")]
    message_id: String,
    #[serde(
        rename = "PlayCard",
        deserialize_with = "deserialize_number_from_string"
    )]
    play_card: u8,
    #[serde(rename = "Direction")]
    direction: String,
}

impl PlayedMoveMent {
    pub fn play_card(&self) -> u8 {
        self.play_card
    }

    pub fn direction(&self) -> &str {
        &self.direction
    }
}

#[derive(Deserialize, Debug)]
pub struct PlayedAttack {
    #[serde(rename = "Type")]
    typ: String,
    #[serde(rename = "From")]
    from: String,
    #[serde(rename = "To")]
    to: String,
    #[serde(rename = "MessageID")]
    message_id: String,
    #[serde(
        rename = "PlayCard",
        deserialize_with = "deserialize_number_from_string"
    )]
    play_card: u8,
    #[serde(
        rename = "NumOfCard",
        deserialize_with = "deserialize_number_from_string"
    )]
    num_of_card: u8,
}

impl PlayedAttack {
    pub fn play_card(&self) -> u8 {
        self.play_card
    }

    pub fn num_of_card(&self) -> u8 {
        self.num_of_card
    }
}

#[derive(Debug)]
pub enum Played {
    MoveMent(PlayedMoveMent),
    Attack(PlayedAttack),
}

#[derive(Deserialize, Debug)]
pub struct RoundEnd {
    #[serde(rename = "Type")]
    typ: String,
    #[serde(rename = "From")]
    from: String,
    #[serde(rename = "To")]
    to: String,
    #[serde(
        rename = "RWinner",
        deserialize_with = "deserialize_number_from_string"
    )]
    round_winner: i8,
    #[serde(rename = "Score0", deserialize_with = "deserialize_number_from_string")]
    score_0: u32,
    #[serde(rename = "Score1", deserialize_with = "deserialize_number_from_string")]
    score_1: u32,
    #[serde(rename = "Message")]
    message: String,
}

impl RoundEnd {
    pub fn round_winner(&self) -> i8 {
        self.round_winner
    }
}

#[derive(Deserialize, Debug)]
pub struct GameEnd {
    #[serde(rename = "Type")]
    typ: String,
    #[serde(rename = "From")]
    from: String,
    #[serde(rename = "To")]
    to: String,
    #[serde(rename = "Winner", deserialize_with = "deserialize_number_from_string")]
    winner: u8,
    #[serde(rename = "Score0", deserialize_with = "deserialize_number_from_string")]
    score_0: u32,
    #[serde(rename = "Score1", deserialize_with = "deserialize_number_from_string")]
    score_1: u32,
    #[serde(rename = "Message")]
    message: String,
}

impl GameEnd {
    pub fn winner(&self) -> u8 {
        self.winner
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename = "Error")]
pub struct ServerError {
    #[serde(rename = "Type")]
    typ: String,
    #[serde(rename = "From")]
    from: String,
    #[serde(rename = "To")]
    to: String,
    #[serde(rename = "Message")]
    message: String,
    #[serde(rename = "MessageID")]
    message_id: String,
}

//ゲーム中に繰り返し受信されるJSON達
// ConnectionStartとNameReceivedは最初しか来ないので除外
pub enum Messages {
    BoardInfo(BoardInfo),
    HandInfo(HandInfo),
    DoPlay(DoPlay),
    Accept(Accept),
    Played(Played),
    RoundEnd(RoundEnd),
    GameEnd(GameEnd),
    ServerError(ServerError),
}

#[derive(Debug)]
pub struct ParseMessageError {
    invalid_info: String,
}

impl Display for ParseMessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ParseMessageError, json is {}", self.invalid_info)
    }
}

impl Error for ParseMessageError {}

impl Messages {
    pub fn parse(json: &str) -> Result<Messages, Errors> {
        let obj = serde_json::from_str::<Value>(json)?;
        let typ = obj
            .get("Type")
            .ok_or("Typeキー無し")?
            .as_str()
            .ok_or("Typeが文字列ではない")?;
        match typ {
            "BoardInfo" => {
                let board_info = serde_json::from_str(json)?;
                Ok(Self::BoardInfo(BoardInfo::from_deserialized(&board_info)))
            }
            "HandInfo" => Ok(Self::HandInfo(serde_json::from_str(json)?)),
            "DoPlay" => Ok(Self::DoPlay(serde_json::from_value(obj)?)),
            "Accept" => Ok(Self::Accept(serde_json::from_value(obj)?)),
            "RoundEnd" => Ok(Self::RoundEnd(serde_json::from_value(obj)?)),
            "GameEnd" => Ok(Self::GameEnd(serde_json::from_value(obj)?)),
            "Played" => {
                let message_id = obj
                    .get("MessageID")
                    .ok_or("MessageID無し")?
                    .as_str()
                    .ok_or("MessageIDが文字列ではない")?;
                match message_id {
                    "101" => Ok(Self::Played(Played::MoveMent(serde_json::from_value(obj)?))),
                    "102" => Ok(Self::Played(Played::Attack(serde_json::from_value(obj)?))),
                    _ => Err(ParseMessageError {
                        invalid_info: json.to_string(),
                    })?,
                }
            }
            "Error" => Ok(Self::ServerError(serde_json::from_value(obj)?)),
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
    #[serde(rename = "ClientID")]
    client_id: PlayerID,
}

impl ConnectionStart {
    pub fn client_id(&self) -> PlayerID {
        self.client_id
    }
}

#[derive(Serialize)]
pub struct PlayerName {
    #[serde(rename = "Type")]
    typ: &'static str,
    #[serde(rename = "From")]
    from: &'static str,
    #[serde(rename = "To")]
    to: &'static str,
    #[serde(rename = "Name")]
    name: String,
}

impl PlayerName {
    pub fn new(name: String) -> Self {
        PlayerName {
            typ: "PlayerName",
            from: "Client",
            to: "Server",
            name,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct NameReceived {
    #[serde(rename = "Type")]
    typ: &'static str,
    #[serde(rename = "From")]
    from: &'static str,
    #[serde(rename = "To")]
    to: &'static str,
}

#[skip_serializing_none]
#[derive(Serialize)]
pub struct Evaluation {
    #[serde(rename = "Type")]
    typ: &'static str,
    #[serde(rename = "From")]
    from: &'static str,
    #[serde(rename = "To")]
    to: &'static str,
    #[serde(rename = "1F")]
    eval_1f: Option<String>,
    #[serde(rename = "1B")]
    eval_1b: Option<String>,
    #[serde(rename = "2F")]
    eval_2f: Option<String>,
    #[serde(rename = "2B")]
    eval_2b: Option<String>,
    #[serde(rename = "3F")]
    eval_3f: Option<String>,
    #[serde(rename = "3B")]
    eval_3b: Option<String>,
    #[serde(rename = "4F")]
    eval_4f: Option<String>,
    #[serde(rename = "4B")]
    eval_4b: Option<String>,
    #[serde(rename = "5F")]
    eval_5f: Option<String>,
    #[serde(rename = "5B")]
    eval_5b: Option<String>,
}

impl Evaluation {
    pub fn new() -> Self {
        Self {
            typ: "Evaluation",
            from: "Client",
            to: "Server",
            eval_1f: Some("1.0".to_string()),
            eval_1b: None,
            eval_2f: None,
            eval_2b: None,
            eval_3f: None,
            eval_3b: None,
            eval_4f: None,
            eval_4b: None,
            eval_5f: None,
            eval_5b: None,
        }
    }
}

#[derive(Serialize)]
pub struct PlayMovement {
    #[serde(rename = "Type")]
    typ: &'static str,
    #[serde(rename = "From")]
    from: &'static str,
    #[serde(rename = "To")]
    to: &'static str,
    #[serde(rename = "MessageID")]
    message_id: &'static str,
    #[serde(rename = "PlayCard")]
    play_card: String,
    #[serde(rename = "Direction")]
    direction: String,
}

impl PlayMovement {
    pub fn from_info(info: Movement) -> Self {
        PlayMovement {
            typ: "Play",
            from: "Client",
            to: "Server",
            message_id: "101",
            play_card: info.card().to_string(),
            direction: info.direction().to_string(),
        }
    }
}

#[derive(Serialize)]
pub struct PlayAttack {
    #[serde(rename = "Type")]
    typ: &'static str,
    #[serde(rename = "From")]
    from: &'static str,
    #[serde(rename = "To")]
    to: &'static str,
    #[serde(rename = "MessageID")]
    message_id: &'static str,
    #[serde(rename = "PlayCard")]
    play_card: String,
    #[serde(rename = "NumOfCard")]
    num_of_card: String,
}

impl PlayAttack {
    pub fn from_info(info: Attack) -> Self {
        Self {
            typ: "Play",
            from: "Client",
            to: "Server",
            message_id: "102",
            play_card: info.card().to_string(),
            num_of_card: info.quantity().to_string(),
        }
    }
}
