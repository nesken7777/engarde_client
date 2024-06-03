//! En Gardeのクライアント用ライブラリ

use std::{
    io::{self, BufRead, BufReader, BufWriter, Write},
    net::TcpStream,
};

use protocol::{ConnectionStart, PlayerID};
use serde::Serialize;

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
    pub fn new(n: u8) -> Option<Maisuu> {
        (n <= 5).then_some(Maisuu(n))
    }

    /// カード枚数を`u8`の表現にします。
    pub fn denote(&self) -> u8 {
        self.0
    }

    /// カード枚数を`usize`の表現にします。
    // clippyごめｎ
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
        Maisuu::new(n)
    }

    /// カード枚数同士を足します。
    /// `Maisuu::MAX`を超える場合、`Maisuu::MAX`になります。
    pub fn saturating_add(&self, other: Maisuu) -> Maisuu {
        let n = self.0 + other.0;
        if n <= 5 {
            Maisuu::new_unchecked(n)
        } else {
            Maisuu::MAX
        }
    }

    /// カードを減算します。0以下は全て0となります。
    pub fn saturating_sub(&self, other: Maisuu) -> Maisuu {
        let n = self.0.saturating_sub(other.0);
        Maisuu::new_unchecked(n)
    }

    /// カードを`n`倍します。
    /// `Maisuu::MAX`を超えた場合、`Maisuu::MAX`になります。
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

pub fn print(string: &str) -> io::Result<()> {
    let mut stdout = std::io::stdout();
    stdout.write_all(string.as_bytes())?;
    stdout.write_all(b"\r\n")?;
    stdout.flush()
}

pub fn read_stream(bufreader: &mut BufReader<TcpStream>) -> io::Result<String> {
    let mut string = String::new();
    bufreader.read_line(&mut string)?;
    Ok(string.trim().to_string())
}

pub fn get_id(bufreader: &mut BufReader<TcpStream>) -> io::Result<PlayerID> {
    let string = read_stream(bufreader)?;
    let connection_start = serde_json::from_str::<ConnectionStart>(&string)
        .expect("来たものがConnectionStartじゃない");
    Ok(connection_start.client_id())
}

pub fn send_info<W, T>(writer: &mut BufWriter<W>, info: &T) -> io::Result<()>
where
    W: Write,
    T: Serialize,
{
    let string = format!("{}\r\n", serde_json::to_string(info).unwrap());
    writer.write_all(string.as_bytes())?;
    writer.flush()?;
    Ok(())
}
