//! En Gardeのクライアント用ライブラリ

use std::{
    fmt::{self, Display, Formatter},
    io::{self, stdout, BufRead, BufReader, BufWriter, Write},
    net::TcpStream,
    ops::{Deref, Index, IndexMut},
    str::FromStr,
};

use protocol::{ConnectionStart, PlayerID};
use serde::{Deserialize, Serialize};

pub mod algorithm;
pub mod algorithm2;
pub mod errors;
pub mod protocol;
pub mod states;

/// カード番号を示します。
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum CardID {
    /// 番号1
    One = 1,
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
    /// カード番号の最大値です
    pub const MAX: usize = 5;

    /// `u8`上の表現を返します
    pub const fn denote(&self) -> u8 {
        use CardID::{Five, Four, One, Three, Two};
        match self {
            One => 1,
            Two => 2,
            Three => 3,
            Four => 4,
            Five => 5,
        }
    }

    /// `usize`上の表現を返します
    pub const fn denote_usize(&self) -> usize {
        use CardID::{Five, Four, One, Three, Two};
        match self {
            One => 1,
            Two => 2,
            Three => 3,
            Four => 4,
            Five => 5,
        }
    }

    /// `u8`から`CardID`を作成します
    pub const fn from_u8(n: u8) -> Option<CardID> {
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

    /// `usize`から`CardID`を作成します
    pub const fn from_usize(n: usize) -> Option<CardID> {
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

/// ある番号の上でのカードの枚数を示します。
/// 0～5の値が許可されます。
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Maisuu(u8);

impl Maisuu {
    /// 最大値です。多くの場合5です。
    pub const MAX: Maisuu = Maisuu::FIVE;
    /// 必ず倒せる枚数です。3枚です。
    pub const SOKUSHI: Maisuu = Maisuu::THREE;
    /// 0枚です。
    pub const ZERO: Maisuu = Maisuu(0);
    /// 1枚です。
    pub const ONE: Maisuu = Maisuu(1);
    /// 2枚です。
    pub const TWO: Maisuu = Maisuu(2);
    /// 3枚です。
    pub const THREE: Maisuu = Maisuu(3);
    /// 4枚です。
    pub const FOUR: Maisuu = Maisuu(4);
    /// 5枚です。
    pub const FIVE: Maisuu = Maisuu(5);
    /// カード枚数を作成します。
    /// 0～5の値までが許容され、それ以外は`None`となります。
    pub fn from_u8(n: u8) -> Option<Maisuu> {
        (n <= 5).then_some(Maisuu(n))
    }

    /// `usize`からカード枚数を作成します。
    /// /// 0～5の値までが許容され、それ以外は`None`となります。
    pub fn from_usize(n: usize) -> Option<Maisuu> {
        let n: u8 = n.try_into().ok()?;
        (n <= 5).then_some(Maisuu(n))
    }

    /// カード枚数を`u8`の表現にします。
    pub const fn denote(&self) -> u8 {
        self.0
    }

    /// カード枚数を`usize`の表現にします。
    // clippyごめん
    #[allow(clippy::as_conversions)]
    pub const fn denote_usize(&self) -> usize {
        self.0 as usize
    }

    /// 内部で使用します。
    fn new_unchecked(n: u8) -> Maisuu {
        Maisuu(n)
    }

    /// カード枚数同士を足します。
    /// `Maisuu::MAX`を超える場合、`None`となります。
    pub fn checked_add(&self, other: Maisuu) -> Option<Maisuu> {
        let n = self.0 + other.0;
        Maisuu::from_u8(n)
    }

    /// カード枚数同士を足します。
    /// `Maisuu::MAX`を超える場合、`Maisuu::MAX`になります。
    #[must_use]
    pub fn saturating_add(&self, other: Maisuu) -> Maisuu {
        let n = self.0 + other.0;
        if n <= 5 {
            Maisuu::new_unchecked(n)
        } else {
            Maisuu::MAX
        }
    }

    /// カードを減算します。0以下は全て0となります。
    #[must_use]
    pub fn saturating_sub(&self, other: Maisuu) -> Maisuu {
        let n = self.0.saturating_sub(other.0);
        Maisuu::new_unchecked(n)
    }

    /// カードを`n`倍します。
    /// `Maisuu::MAX`を超えた場合、`Maisuu::MAX`になります。
    #[must_use]
    pub fn saturating_mul(&self, n: u8) -> Maisuu {
        let n = self.0.saturating_mul(n);
        if n <= 5 {
            Maisuu::new_unchecked(n)
        } else {
            Maisuu::MAX
        }
    }
}

/// 自分と相手は通常5枚を手持ちに入れているはずです。
pub const HANDS_DEFAULT_U8: u8 = 5;

/// `HANDS_DEFAULT_U8`の`u64`版です。
pub const HANDS_DEFAULT_U64: u64 = 5;

/// 文字列を出力します。
/// # Errors
/// 何かの問題で出力に失敗したときエラーを返します。
pub fn print<S: AsRef<str>>(string: S) -> io::Result<()> {
    fn print_internal(string: &str) -> io::Result<()> {
        let mut stdout = stdout();
        stdout.write_all(string.as_bytes())?;
        stdout.write_all(b"\r\n")?;
        stdout.flush()
    }
    print_internal(string.as_ref())
}

/// 通信を1行読み取ります。
/// # Errors
/// 何らかのの問題で通信エラーが発生した場合エラーを返します。
pub fn read_stream(bufreader: &mut BufReader<TcpStream>) -> io::Result<String> {
    let mut string = String::new();
    bufreader.read_line(&mut string)?;
    Ok(string.trim().to_string())
}

/// 通信内容からIDを取得します。
/// # Errors
/// 何らかのの問題で通信エラーが発生した場合エラーを返します。
/// # Panics
/// サーバーから送られてくるものが`ConnectionStart`ではない場合パニックします。
pub fn get_id(bufreader: &mut BufReader<TcpStream>) -> io::Result<PlayerID> {
    let string = read_stream(bufreader)?;
    let connection_start = serde_json::from_str::<ConnectionStart>(&string)
        .expect("来たものがConnectionStartじゃない");
    Ok(connection_start.client_id())
}

/// サーバーへ情報を送ります。
/// # Errors
/// 何らかのの問題で通信エラーが発生した場合エラーを返します。
pub fn send_info<W, T>(writer: &mut BufWriter<W>, info: &T) -> io::Result<()>
where
    W: Write,
    T: Serialize,
{
    let string = format!("{}\r\n", serde_json::to_string(info)?);
    writer.write_all(string.as_bytes())?;
    writer.flush()?;
    Ok(())
}

/// 残りのカード枚数(カード番号ごと)
#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct RestCards {
    cards: [Maisuu; CardID::MAX],
}

impl Default for RestCards {
    fn default() -> Self {
        Self::new()
    }
}

impl RestCards {
    /// 初期値を返します。
    pub fn new() -> Self {
        Self {
            cards: [Maisuu::MAX; CardID::MAX],
        }
    }

    /// スライスから作成します
    /// # Panics
    /// スライスの長さが5以外の場合パニックします。
    pub fn from_slice(slice: &[Maisuu]) -> RestCards {
        RestCards {
            cards: slice.try_into().expect("スライスの長さが5ではない"),
        }
    }
    /// `action`から残りのカード枚数を減らします。
    pub fn used_card(&mut self, action: Action) {
        match action {
            Action::Move(movement) => {
                let i = movement.card.denote_usize();
                self[i - 1] = self[i - 1].saturating_sub(Maisuu::ONE);
            }
            Action::Attack(attack) => {
                let i = attack.card.denote_usize();
                self[i - 1] = self[i - 1].saturating_sub(attack.quantity.saturating_mul(2));
            }
        }
    }
}

impl Index<usize> for RestCards {
    type Output = Maisuu;
    fn index(&self, index: usize) -> &Self::Output {
        self.cards.get(index).expect("out of bound")
    }
}

impl IndexMut<usize> for RestCards {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.cards.get_mut(index).expect("out of bound")
    }
}

impl Deref for RestCards {
    type Target = [Maisuu];
    fn deref(&self) -> &Self::Target {
        &self.cards
    }
}

/// 「動き」の方向です
#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Direction {
    /// 前
    Forward,
    /// 後ろ
    Back,
}

impl Direction {
    /// `u8`での表現を表します。
    pub fn denote(&self) -> u8 {
        match self {
            Self::Forward => 0,
            Self::Back => 1,
        }
    }
}

impl FromStr for Direction {
    type Err = &'static str;
    /// `"F"`、`"B"`といった文字列から方向を生成します。
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "F" => Ok(Self::Forward),
            "B" => Ok(Self::Back),
            _ => Err("有効な方向ではないです"),
        }
    }
}

impl Display for Direction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Forward => write!(f, "F"),
            Self::Back => write!(f, "B"),
        }
    }
}

/// 「動き」を表します。
#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct Movement {
    card: CardID,
    direction: Direction,
}

impl Movement {
    /// カード番号と方向から生成します。
    pub fn new(card: CardID, direction: Direction) -> Self {
        Self { card, direction }
    }

    /// カード番号を返します。
    pub fn card(&self) -> CardID {
        self.card
    }

    /// 方向を返します。
    pub fn direction(&self) -> Direction {
        self.direction
    }
}

/// 「攻撃」を表現します。
#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub struct Attack {
    card: CardID,
    quantity: Maisuu,
}

impl Attack {
    /// カード番号と枚数から生成します。
    pub fn new(card: CardID, quantity: Maisuu) -> Self {
        Self { card, quantity }
    }

    /// カード番号を返します。
    pub fn card(&self) -> CardID {
        self.card
    }

    /// 枚数を返します。
    pub fn quantity(&self) -> Maisuu {
        self.quantity
    }
}

/// 「動き」もしくは「攻撃」のセットです。
#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub enum Action {
    /// 動き
    Move(Movement),
    /// 攻撃
    Attack(Attack),
}

impl Action {
    /// 配列の添え字で表現したときのインデックスを返します。
    pub fn as_index(&self) -> usize {
        match self {
            Action::Move(movement) => {
                let &Movement { card, direction } = movement;
                match direction {
                    Direction::Forward => card.denote_usize() - 1,
                    Direction::Back => 5 + (card.denote_usize() - 1),
                }
            }
            Action::Attack(attack) => {
                let &Attack { card, quantity } = attack;
                5 * 2 + 5 * (card.denote_usize() - 1) + (quantity.denote_usize() - 1)
            }
        }
    }

    /// 配列表現でのインデックスから行動を生成します。
    /// # Panics
    /// 絶対にパニックしません。
    pub fn from_index(idx: usize) -> Action {
        match idx {
            x @ 0..=4 => Action::Move(Movement {
                card: CardID::from_usize(x + 1).expect("CardIDの境界内"),
                direction: Direction::Forward,
            }),
            x @ 5..=9 => Action::Move(Movement {
                card: CardID::from_usize(x - 5 + 1).expect("CardIDの境界内"),
                direction: Direction::Back,
            }),
            x @ 10..=34 => {
                let x = x - 10;
                Action::Attack(Attack {
                    card: CardID::from_usize(x / 5 + 1).expect("CardIDの境界内"),
                    quantity: Maisuu::from_usize(x % 5 + 1).expect("Maisuuの境界内"),
                })
            }
            _ => unreachable!(),
        }
    }

    /// 「動き」であると確信している場合に使います。
    fn get_movement(self) -> Option<Movement> {
        match self {
            Action::Move(movement) => Some(movement),
            Action::Attack(_) => None,
        }
    }
}

impl From<Action> for [f32; 35] {
    fn from(value: Action) -> Self {
        let mut arr = [0_f32; 35];
        arr[value.as_index()] = 1.0;
        arr
    }
}

impl From<[f32; 35]> for Action {
    fn from(value: [f32; 35]) -> Self {
        let idx = value
            .into_iter()
            .enumerate()
            .max_by(|&(_, x), &(_, y)| x.total_cmp(&y))
            .map(|(i, _)| i)
            .expect("必ず最大値が存在する");
        Action::from_index(idx)
    }
}
