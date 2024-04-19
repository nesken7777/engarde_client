mod protocol;
use protocol::{
    BoardInfo, ConnectionStart, HandInfo, Messages, NameReceived, PlayerName, RequestedPlay,
};
mod errors;
use errors::Errors;
use serde::Serialize;
use std::{
    error::Error,
    io::{self, BufRead, BufReader, BufWriter, Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
};
use Messages::*;
use RequestedPlay::*;

fn main() -> Result<(), Errors> {
    // IPアドレスはいつか標準入力になると思います。
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12052);
    let stream = TcpStream::connect(addr)?;
    let (mut bufreader, mut bufwriter) =
        (BufReader::new(stream.try_clone()?), BufWriter::new(stream));
    connect(&mut bufreader)?;
    {
        // ここはどうする?標準入力にする?
        print("名前を入力")?;
        let name = read_keybord()?;
        print(format!("{}", name).as_str())?;
        let player_name = PlayerName::new(name);
        send_info(&mut bufwriter, &player_name)?;
        let string = read_stream(&mut bufreader)?;
        let name_received = serde_json::from_str::<NameReceived>(&string)?;
        print(format!("{:?}", name_received).as_str())?;
    }
    {
        let mut board_state = BoardInfo::new();
        let mut hand_state = HandInfo::new();
        loop {
            match Messages::parse(&read_stream(&mut bufreader)?)? {
                BoardInfo(board_info) => {
                    board_state = board_info;
                }
                HandInfo(hand_info) => hand_state = hand_info,
                DoPlay(do_play) => {
                    let play_mode = RequestedPlay::from_id(do_play.message_id)?;
                    match play_mode {
                        NormalTurn => {
                            print("どうする?")?;
                            let hands = hand_state.to_vec();
                            let play_mode = ();
                            let number = {
                                loop {
                                    print("カードを選んでね")?;
                                    let mut string = String::new();
                                    std::io::stdin().read_line(&mut string)?;
                                    let kouho = string.trim().parse::<u8>()?;
                                    if !hands.contains(&kouho) {
                                        print("そのカードは無いよ")?;
                                    } else {
                                        break kouho;
                                    }
                                }
                            };
                        }
                        Parry => (),
                    }
                }
                RoundEnd(round_end) => (),
                GameEnd(game_end) => break,
            }
        }
    }
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

fn send_info<W, T>(writer: &mut BufWriter<W>, info: &T) -> Result<(), Errors>
where
    W: Write,
    T: Serialize,
{
    let string = format!("{}\r\n", serde_json::to_string(info)?);
    writer.write_all(string.as_bytes())?;
    writer.flush()?;
    Ok(())
}

fn connect<T>(bufreader: &mut BufReader<T>) -> Result<u8, Errors>
where
    T: Read,
{
    let string = read_stream(bufreader)?;
    let connection_start = serde_json::from_str::<ConnectionStart>(&string)?;
    dbg!(&connection_start);
    Ok(connection_start.client_id)
}

fn read_keybord() -> io::Result<String> {
    let mut word = String::new();
    std::io::stdin().read_line(&mut word)?;
    let response = word.trim().to_string();
    Ok(response)
}

fn print(string: &str) -> io::Result<()> {
    let mut stdout = std::io::stdout();
    stdout.write_all(string.as_bytes())?;
    stdout.flush()
}


fn combination(n: u64, mut r: u64) -> u64{
    let perm = permutation(n, r);
    perm / (1..=r).product::<u64>()
}

