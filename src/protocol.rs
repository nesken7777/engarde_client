use serde::{Deserialize, Serialize};
use serde_aux::prelude::*;

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
    typ: String,
    #[serde(rename = "From")]
    from: String,
    #[serde(rename = "To")]
    to: String,
    #[serde(rename = "Name")]
    name: String,
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
    player_score_0: u8,
    #[serde(
        rename = "PlayerScore_1",
        deserialize_with = "deserialize_number_from_string"
    )]
    player_score_1: u8,
    #[serde(
        rename = "NumofDeck",
        deserialize_with = "deserialize_number_from_string"
    )]
    num_of_deck: u8,
    #[serde(
        rename = "CurrentPlayer",
        deserialize_with = "deserialize_number_from_string"
    )]
    current_player: u8,
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
    hand1: u8,
    #[serde(rename = "Hand2", deserialize_with = "deserialize_number_from_string")]
    hand2: u8,
    #[serde(rename = "Hand3", deserialize_with = "deserialize_number_from_string")]
    hand3: u8,
    #[serde(rename = "Hand4", deserialize_with = "deserialize_number_from_string")]
    hand4: u8,
    #[serde(rename = "Hand5", deserialize_with = "deserialize_number_from_string")]
    hand5: u8,
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
    message: String,
}

#[derive(Serialize)]
pub struct Evaluation {
    #[serde(rename = "Type")]
    typ: String,
    #[serde(rename = "From")]
    from: String,
    #[serde(rename = "To")]
    to: String,
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

#[derive(Serialize)]
pub struct Movement {
    #[serde(rename = "Type")]
    typ: String,
    #[serde(rename = "From")]
    from: String,
    #[serde(rename = "To")]
    to: String,
    #[serde(rename = "MessageID")]
    message_id: String,
    #[serde(rename = "PlayCard")]
    play_card: String,
    #[serde(rename = "Direction")]
    direction: String,
}

#[derive(Serialize)]
pub struct Attack {
    #[serde(rename = "Type")]
    typ: String,
    #[serde(rename = "From")]
    from: String,
    #[serde(rename = "To")]
    to: String,
    #[serde(rename = "MessageID")]
    message_id: String,
    #[serde(rename = "PlayCard")]
    play_card: String,
    #[serde(rename = "NumOfCard")]
    num_of_card: String,
}

#[derive(Serialize)]
pub struct Parry {
    #[serde(rename = "Type")]
    typ: String,
    #[serde(rename = "From")]
    from: String,
    #[serde(rename = "To")]
    to: String,
    #[serde(rename = "MessageID")]
    message_id: String,
    #[serde(rename = "PlayCard")]
    play_card: String,
    #[serde(rename = "NumOfCard")]
    num_of_card: String,
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
    winner: u8,
    #[serde(rename = "Score0", deserialize_with = "deserialize_number_from_string")]
    score_0: u8,
    #[serde(rename = "Score1", deserialize_with = "deserialize_number_from_string")]
    score_1: u8,
    #[serde(rename = "Message")]
    message: String,
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
    winner: u8,
    #[serde(rename = "Score0", deserialize_with = "deserialize_number_from_string")]
    score_0: u8,
    #[serde(rename = "Score1", deserialize_with = "deserialize_number_from_string")]
    score_1: u8,
    #[serde(rename = "Message")]
    message: String,
}
