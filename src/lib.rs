use std::{
    io::{self, BufRead, BufReader, BufWriter, Write},
    net::TcpStream,
};

use protocol::{ConnectionStart, PlayerID};
use serde::Serialize;

pub mod states;
pub mod algorithm;
pub mod errors;
pub mod protocol;
pub mod algorithm2;

/// カード番号を示す。
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
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
