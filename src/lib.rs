use std::{
    io::{self, BufRead, BufReader, BufWriter, Write},
    net::TcpStream,
};

use protocol::{ConnectionStart, PlayerID};
use serde::Serialize;

pub mod algorithm;
pub mod errors;
pub mod protocol;

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
    Ok(connection_start.client_id)
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
