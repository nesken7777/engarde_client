//! 通信プロトコル

use std::fmt::{self, Formatter};
use std::str::FromStr;
use std::{error::Error, fmt::Display};

use serde::de::{self, Unexpected, Visitor};
use serde::{Deserialize, Serialize};
use serde_aux::prelude::*;
use serde_json::Value;
use serde_with::skip_serializing_none;

use crate::errors::Errors;

use crate::{Action, Attack, CardID, Direction, Maisuu, Movement};

/// サーバーから送られてくるプレイヤーIDを示します。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum PlayerID {
    /// プレイヤー0
    Zero,
    /// プレイヤー1
    One,
}

impl PlayerID {
    /// プレイヤーIDを`u8`上の表現にします。
    pub fn denote(&self) -> u8 {
        match self {
            PlayerID::Zero => 0,
            PlayerID::One => 1,
        }
    }

    /// `u8`からプレイヤーIDを生成します。
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

        impl<'de> Visitor<'de> for MyEnumVisitor {
            type Value = PlayerID;

            fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
                formatter.write_str("0 or 1 or Zero or One")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match v.trim() {
                    "0" | "Zero" => Ok(PlayerID::Zero),
                    "1" | "One" => Ok(PlayerID::One),
                    _ => Err(E::invalid_value(Unexpected::Str(v), &self)),
                }
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                self.visit_str(v.as_str())
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match v {
                    0 => Ok(PlayerID::Zero),
                    1 => Ok(PlayerID::One),
                    _ => Err(E::invalid_value(Unexpected::Unsigned(v), &self)),
                }
            }
        }

        deserializer.deserialize_any(MyEnumVisitor)
    }
}

#[derive(Deserialize, Debug)]
struct BoardInfoJson {
    #[serde(rename = "Type")]
    _typ: String,
    #[serde(rename = "From")]
    _from: String,
    #[serde(rename = "To")]
    _to: String,
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

/// サーバーから送られてくるボード情報を表します。
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

    /// プレイヤー0の位置を返します。
    pub fn p0_position(&self) -> u8 {
        self.p0_position
    }

    /// プレイヤー1の位置を返します。
    pub fn p1_position(&self) -> u8 {
        self.p1_position
    }

    /// プレイヤー間の距離を返します。
    pub fn distance_between_enemy(&self) -> u8 {
        self.p1_position - self.p0_position
    }

    /// プレイヤー0の点数を返します。
    pub fn p0_score(&self) -> u32 {
        self.p0_score
    }

    /// プレイヤー1の点数を返します。
    pub fn p1_score(&self) -> u32 {
        self.p1_score
    }

    /// 山札の枚数を返します。
    pub fn num_of_deck(&self) -> u8 {
        self.num_of_deck
    }

    /// 現在のプレイヤーを返します。
    pub fn current_player(&self) -> Option<PlayerID> {
        self.current_player
    }

    /// 初期化できなくて困ったときに使います。
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

/// サーバーから送られてくる手札の情報を表します。
#[derive(Deserialize, Debug, Clone)]
pub struct HandInfo {
    #[serde(rename = "Type")]
    _typ: String,
    #[serde(rename = "From")]
    _from: String,
    #[serde(rename = "To")]
    _to: String,
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
    /// ベクタに変換します。
    #[allow(clippy::similar_names)]
    pub fn to_vec(&self) -> Vec<CardID> {
        let hand1 = CardID::from_u8(self.hand1);
        let hand2 = CardID::from_u8(self.hand2);
        let hand3 = CardID::from_u8(self.hand3);
        let hand4 = (|| CardID::from_u8(self.hand4?))();
        let hand5 = (|| CardID::from_u8(self.hand5?))();
        let mut hands = vec![hand1, hand2, hand3, hand4, hand5]
            .into_iter()
            .flatten()
            .collect::<Vec<CardID>>();
        hands.sort();
        hands
    }
}

/// サーバーからの攻撃の指示を表します。
#[derive(Deserialize, Debug)]
pub struct DoPlay {
    #[serde(rename = "Type")]
    _typ: String,
    #[serde(rename = "From")]
    _from: String,
    #[serde(rename = "To")]
    _to: String,
    #[serde(
        rename = "MessageID",
        deserialize_with = "deserialize_number_from_string"
    )]
    _message_id: u8,
    #[serde(rename = "Message")]
    _message: String,
}

/// サーバーが`Evaluation`を承認する際に送られる情報を表します。
#[derive(Deserialize, Debug)]
pub struct Accept {
    #[serde(rename = "Type")]
    _typ: String,
    #[serde(rename = "From")]
    _from: String,
    #[serde(rename = "To")]
    _to: String,
    #[serde(rename = "MessageID")]
    _message_id: String,
}

#[derive(Deserialize, Debug)]
struct PlayedMoveMentJson {
    #[serde(rename = "Type")]
    _typ: String,
    #[serde(rename = "From")]
    _from: String,
    #[serde(rename = "To")]
    _to: String,
    #[serde(rename = "MessageID")]
    _message_id: String,
    #[serde(
        rename = "PlayCard",
        deserialize_with = "deserialize_number_from_string"
    )]
    play_card: u8,
    #[serde(rename = "Direction")]
    direction: String,
}

impl PlayedMoveMentJson {
    fn play_card(&self) -> u8 {
        self.play_card
    }

    fn direction(&self) -> &str {
        &self.direction
    }
}

/// 相手が動いたときにサーバーから送られてくる情報を表します。
#[derive(Debug)]
pub struct PlayedMoveMent {
    play_card: CardID,
    direction: Direction,
}

impl PlayedMoveMent {
    fn from_deserialized(json: &PlayedMoveMentJson) -> Self {
        Self {
            play_card: CardID::from_u8(json.play_card()).expect("CardIDの境界内"),
            direction: Direction::from_str(json.direction()).expect("正しい方向"),
        }
    }

    /// 相手が使用したカード番号を返します。
    pub fn play_card(&self) -> CardID {
        self.play_card
    }

    /// 相手が動いた方向を返します。
    pub fn direction(&self) -> Direction {
        self.direction
    }
}

#[derive(Deserialize, Debug)]
struct PlayedAttackJson {
    #[serde(rename = "Type")]
    _typ: String,
    #[serde(rename = "From")]
    _from: String,
    #[serde(rename = "To")]
    _to: String,
    #[serde(rename = "MessageID")]
    _message_id: String,
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

impl PlayedAttackJson {
    pub fn play_card(&self) -> u8 {
        self.play_card
    }

    pub fn num_of_card(&self) -> u8 {
        self.num_of_card
    }
}

/// 相手が攻撃したときにサーバーから送られてくる情報を表します。
#[derive(Debug)]
pub struct PlayedAttack {
    play_card: CardID,
    num_of_card: Maisuu,
}

impl PlayedAttack {
    fn from_deserialized(json: &PlayedAttackJson) -> Self {
        Self {
            play_card: CardID::from_u8(json.play_card()).expect("CardIDの境界内"),
            num_of_card: Maisuu::from_u8(json.num_of_card()).expect("Maisuuの境界内"),
        }
    }

    /// 相手が使用したカード番号を返します。
    pub fn play_card(&self) -> CardID {
        self.play_card
    }

    /// 相手が何枚カードを使用したかを返します。
    pub fn num_of_card(&self) -> Maisuu {
        self.num_of_card
    }
}

/// 相手が行動したときの動きもしくは攻撃のセットです。
#[derive(Debug)]
pub enum Played {
    /// 相手が動いた場合こちらになります。
    MoveMent(PlayedMoveMent),
    /// 相手が攻撃した場合こちらになります。
    Attack(PlayedAttack),
}

impl Played {
    /// `Action`に変換します。
    pub fn to_action(&self) -> Action {
        match self {
            Played::MoveMent(movement) => Action::Move(Movement {
                card: movement.play_card(),
                direction: movement.direction(),
            }),
            Played::Attack(attack) => Action::Attack(Attack {
                card: attack.play_card(),
                quantity: attack.num_of_card(),
            }),
        }
    }
}

/// ラウンドが終了したときにサーバーから送られてくる情報を表します。
#[derive(Deserialize, Debug)]
pub struct RoundEnd {
    #[serde(rename = "Type")]
    _typ: String,
    #[serde(rename = "From")]
    _from: String,
    #[serde(rename = "To")]
    _to: String,
    #[serde(
        rename = "RWinner",
        deserialize_with = "deserialize_number_from_string"
    )]
    round_winner: i8,
    #[serde(rename = "Score0", deserialize_with = "deserialize_number_from_string")]
    _score_0: u32,
    #[serde(rename = "Score1", deserialize_with = "deserialize_number_from_string")]
    _score_1: u32,
    #[serde(rename = "Message")]
    _message: String,
}

impl RoundEnd {
    /// そのラウンドの勝者を返します。
    pub fn round_winner(&self) -> i8 {
        self.round_winner
    }
}

/// 試合全体が終了したときにサーバーから送られてくる情報を表します。
#[derive(Deserialize, Debug)]
pub struct GameEnd {
    #[serde(rename = "Type")]
    _typ: String,
    #[serde(rename = "From")]
    _from: String,
    #[serde(rename = "To")]
    _to: String,
    #[serde(rename = "Winner", deserialize_with = "deserialize_number_from_string")]
    winner: u8,
    #[serde(rename = "Score0", deserialize_with = "deserialize_number_from_string")]
    _score_0: u32,
    #[serde(rename = "Score1", deserialize_with = "deserialize_number_from_string")]
    _score_1: u32,
    #[serde(rename = "Message")]
    _message: String,
}

impl GameEnd {
    /// その試合の勝者を返します。
    pub fn winner(&self) -> u8 {
        self.winner
    }
}

/// サーバーからエラーが来たときの情報を表します。
#[derive(Deserialize, Debug)]
#[serde(rename = "Error")]
pub struct ServerError {
    #[serde(rename = "Type")]
    _typ: String,
    #[serde(rename = "From")]
    _from: String,
    #[serde(rename = "To")]
    _to: String,
    #[serde(rename = "Message")]
    _message: String,
    #[serde(rename = "MessageID")]
    _message_id: String,
}

/// ゲーム中に繰り返し受信されるJSON達
/// `ConnectionStart`と`NameReceived`は最初しか来ないので除外
#[derive(Debug)]
pub enum Messages {
    /// ボード情報
    BoardInfo(BoardInfo),
    /// 手札情報
    HandInfo(HandInfo),
    /// 行動指示
    DoPlay(DoPlay),
    /// `Evaluation`承認
    Accept(Accept),
    /// 相手が行動
    Played(Played),
    /// ラウンド終了
    RoundEnd(RoundEnd),
    /// 試合全体の終了
    GameEnd(GameEnd),
    /// サーバーからのエラー
    ServerError(ServerError),
}

/// メッセージのパースに失敗したときのエラーです。
#[derive(Debug)]
pub struct ParseMessageError {
    invalid_info: String,
}

impl Display for ParseMessageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "ParseMessageError, json is {}", self.invalid_info)
    }
}

impl Error for ParseMessageError {}

impl Messages {
    /// サーバーから送られてくるメッセージをパースします
    /// # Errors
    /// パースに失敗した場合にエラーを返します。
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
                    "101" => {
                        let played_movement_info: PlayedMoveMentJson = serde_json::from_value(obj)?;
                        Ok(Self::Played(Played::MoveMent(
                            PlayedMoveMent::from_deserialized(&played_movement_info),
                        )))
                    }
                    "102" => {
                        let played_attack_info = serde_json::from_value(obj)?;
                        Ok(Self::Played(Played::Attack(
                            PlayedAttack::from_deserialized(&played_attack_info),
                        )))
                    }
                    _ => Err(ParseMessageError {
                        invalid_info: json.to_string(),
                    }
                    .into()),
                }
            }
            "Error" => Ok(Self::ServerError(serde_json::from_value(obj)?)),
            _ => Err(ParseMessageError {
                invalid_info: json.to_string(),
            }
            .into()),
        }
    }
}

/// 通信開始に初めに送られてくる情報です。
#[derive(Deserialize, Debug)]
pub struct ConnectionStart {
    #[serde(rename = "Type")]
    _typ: String,
    #[serde(rename = "From")]
    _from: String,
    #[serde(rename = "To")]
    _to: String,
    #[serde(rename = "ClientID")]
    client_id: PlayerID,
}

impl ConnectionStart {
    /// 自分のプレイヤーIDとなります。
    pub fn client_id(&self) -> PlayerID {
        self.client_id
    }
}

/// クライアント側から送る名前情報です。
#[derive(Debug, Serialize)]
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
    /// 名前を作成します。
    pub fn new(name: String) -> Self {
        PlayerName {
            typ: "PlayerName",
            from: "Client",
            to: "Server",
            name,
        }
    }
}

/// 名前をサーバーが受け取った際に送られてきます。
#[derive(Deserialize, Debug)]
pub struct NameReceived {
    #[serde(rename = "Type")]
    _typ: &'static str,
    #[serde(rename = "From")]
    _from: &'static str,
    #[serde(rename = "To")]
    _to: &'static str,
}

/// サーバーに送る評価値のセットを表します。
#[skip_serializing_none]
#[derive(Debug, Serialize)]
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

impl Default for Evaluation {
    fn default() -> Self {
        Self::new()
    }
}

impl Evaluation {
    /// とりあえず生成したいときに使います。
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

/// サーバーへ送る「動き」の情報を表します。
#[derive(Debug, Serialize)]
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
    /// `Movement`から情報を作ります。
    pub fn from_info(info: Movement) -> Self {
        PlayMovement {
            typ: "Play",
            from: "Client",
            to: "Server",
            message_id: "101",
            play_card: info.card().denote().to_string(),
            direction: info.direction().to_string(),
        }
    }
}

/// サーバーへ送る「攻撃」の情報を表します。
#[derive(Debug, Serialize)]
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
    /// `Attack`から情報を作ります。
    pub fn from_info(info: Attack) -> Self {
        Self {
            typ: "Play",
            from: "Client",
            to: "Server",
            message_id: "102",
            play_card: info.card().denote().to_string(),
            num_of_card: info.quantity().denote().to_string(),
        }
    }
}
