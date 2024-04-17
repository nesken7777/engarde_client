mod protocol;
use protocol::{
    BoardInfo, ConnectionStart, HandInfo, Messages, NameReceived, PlayerName, RequestedPlay,
};
use serde::Serialize;
use std::{
    error::Error,
    fmt::Debug,
    io::{stdout, BufRead, BufReader, BufWriter, Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
};
use Messages::*;
use RequestedPlay::*;
fn main() -> Result<(), Box<dyn Error>> {
    // IPアドレスはいつか標準入力になると思います。
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12052);
    let stream = TcpStream::connect(addr)?;
    let (mut bufreader, mut bufwriter) =
        (BufReader::new(stream.try_clone()?), BufWriter::new(stream));
    connect(&mut bufreader)?;
    {
        // ここはどうする?標準入力にする?
        println!("名前を入力");
        let name=read_keybord();
        println!("{}",name);
        let player_name = PlayerName::new(name);
        send_info(&mut bufwriter, &player_name)?;
        let string = read_stream(&mut bufreader)?;
        let name_received = serde_json::from_str::<NameReceived>(&string)?;
        println!("{:?}", name_received);
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
                            println!("どうする?");
                            let hands = hand_state.to_vec();
                            let play_mode = ();
                            let number = {
                                loop {
                                    println!("カードを選んでね");
                                    let mut string = String::new();
                                    std::io::stdin().read_line(&mut string)?;
                                    let kouho = string.trim().parse::<u8>()?;
                                    if !hands.contains(&kouho) {
                                        println!("そのカードは無いよ");
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
    println!("{:?}", connection_start);
    Ok(connection_start.client_id)
}

fn read_keybord()-> String{
    let mut word = String::new();
    std::io::stdin().read_line(&mut word).ok();
    let response = word.trim().to_string();
    response
}