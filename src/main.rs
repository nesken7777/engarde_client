mod protocol;
use protocol::{BoardInfo, ConnectionStart, NameReceived, PlayerName};
use serde::Serialize;
use std::{
    error::Error,
    fmt::Debug,
    io::{stdout, BufRead, BufReader, BufWriter, Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
};

fn main() -> Result<(), Box<dyn Error>> {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12052);
    let stream = TcpStream::connect(addr)?;
    let (mut bufreader, mut bufwriter) =
        (BufReader::new(stream.try_clone()?), BufWriter::new(stream));
    let player_id = connect(&mut bufreader);
    {
        let player_name = PlayerName::new("abc".to_string());
        send_info(&mut bufwriter, &player_name)?;
        let string = read_stream(&mut bufreader)?;
        let name_received = serde_json::from_str::<NameReceived>(&string)?;
        print(&name_received)?;
    }
    {
        let string = read_stream(&mut bufreader)?;
        let board_info = serde_json::from_str::<BoardInfo>(&string)?;
        print(&board_info)?;
    }
    Ok(())
}

fn print<T>(obj: &T) -> Result<(), Box<dyn Error>>
where
    T: Debug,
{
    let mut out = stdout();
    out.write_all(format!("{:?}\r\n", obj).as_bytes())?;
    out.flush()?;
    Ok(())
}

fn read_stream<T>(bufreader: &mut BufReader<T>) -> Result<String, Box<dyn Error>>
where
    T: Read,
{
    let mut string = String::new();
    bufreader.read_line(&mut string)?;
    Ok(string.trim().to_string())
}

fn send_info<W, T>(writer: &mut BufWriter<W>, info: &T) -> Result<(), Box<dyn Error>>
where
    W: Write,
    T: Serialize,
{
    let string = format!("{}\r\n", serde_json::to_string(info)?);
    writer.write_all(string.as_bytes())?;
    writer.flush()?;
    Ok(())
}

fn connect<T>(bufreader: &mut BufReader<T>) -> Result<u8, Box<dyn Error>>
where
    T: Read,
{
    let string = read_stream(bufreader)?;
    let connection_start = serde_json::from_str::<ConnectionStart>(&string)?;
    print(&connection_start)?;
    Ok(connection_start.client_id)
}
