mod protocol;
mod errors;
mod algorithm;
use protocol::{
    BoardInfo, ConnectionStart, Evaluation, Messages, NameReceived, PlayAttack,
    PlayMovement, PlayerName, PlayerProperty,
};
use errors::Errors;
use serde::Serialize;
use std::{
    io::{self, BufRead, BufReader, BufWriter, Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
};
use Messages::*;

fn print(string: &str) -> io::Result<()> {
    let mut stdout = std::io::stdout();
    stdout.write_all(string.as_bytes())?;
    stdout.write_all(b"\r\n")?;
    stdout.flush()
}

fn read_keybord() -> io::Result<String> {
    let mut word = String::new();
    std::io::stdin().read_line(&mut word)?;
    let response = word.trim().to_string();
    Ok(response)
}

fn read_stream<T>(bufreader: &mut BufReader<T>) -> io::Result<String>
where
    T: Read,
{
    let mut string = String::new();
    bufreader.read_line(&mut string)?;
    Ok(string.trim().to_string())
}

fn connect<T>(bufreader: &mut BufReader<T>) -> Result<u8, Errors>
where
    T: Read,
{
    let string = read_stream(bufreader)?;
    let connection_start = serde_json::from_str::<ConnectionStart>(&string)?;
    Ok(connection_start.client_id)
}

fn ask_card(player: &PlayerProperty) -> Result<u8, Errors> {
    loop {
        print("カードはどれにする?")?;
        let Ok(card) = read_keybord()?.parse::<u8>() else {
            print("それ数字じゃないだろ")?;
            continue;
        };
        if !player.hand.contains(&card) {
            print("そのカードは無いよ")?;
            continue;
        } else {
            break Ok(card);
        }
    }
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

enum Direction {
    Forward,
    Back,
}

impl Direction {
    fn to_string(&self) -> String {
        match self {
            Self::Forward => "F".to_string(),
            Self::Back => "B".to_string(),
        }
    }
}

enum Action {
    Move { card: u8, direction: Direction },
    Attack { card: u8, quantity: u8 },
}

fn ask_action(player: &PlayerProperty, board: &BoardInfo) -> Result<Action, Errors> {
    print(format!("手札:{:?}", player.hand).as_str())?;
    let action_str = {
        loop {
            print("どっちのアクションにする?")?;
            let string = read_keybord()?;
            match string.as_str() {
                "M" => break "M",
                "A" => break "A",
                _ => {
                    print("その行動は無いよ")?;
                }
            }
        }
    };
    match action_str {
        "M" => {
            let card = ask_card(player)?;
            let direction = loop {
                print("どっち向きにする?")?;
                let string = read_keybord()?;
                match string.as_str() {
                    "F" => break Direction::Forward,
                    "B" => break Direction::Back,
                    _ => {
                        print("その方向は無いよ")?;
                    }
                }
            };
            Ok(Action::Move { card, direction })
        }
        "A" => {
            let card = board.distance_between_enemy();
            let quantity = {
                print("何枚使う?")?;
                read_keybord()?.parse::<u8>()?
            };
            Ok(Action::Attack { card, quantity })
        }
        _ => unreachable!(),
    }
}

fn act(
    my_info: &PlayerProperty,
    board_state: &BoardInfo,
    bufwriter: &mut BufWriter<TcpStream>,
) -> Result<(), Errors> {
    let action = ask_action(my_info, board_state)?;
    match action {
        Action::Move { card, direction } => {
            send_info(bufwriter, &PlayMovement::from_info(card, direction))?;
            dbg!();
        }
        Action::Attack { card, quantity } => {
            send_info(bufwriter, &PlayAttack::from_info(card, quantity))?;
        }
    }
    Ok(())
}

fn main() -> Result<(), Errors> {
    // IPアドレスはいつか標準入力になると思います。
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12052);
    let stream = TcpStream::connect(addr)?;
    let (mut bufreader, mut bufwriter) =
        (BufReader::new(stream.try_clone()?), BufWriter::new(stream));
    let id = connect(&mut bufreader)?;
    let mut my_info = PlayerProperty::new(id);
    {
        // ここはどうする?標準入力にする?
        print("名前を入力")?;
        let name = read_keybord()?;
        let player_name = PlayerName::new(name);
        send_info(&mut bufwriter, &player_name)?;
        let _ = read_stream(&mut bufreader)?;
    }
    {
        let mut board_state = BoardInfo::new();
        let mut cards = vec![5; 5];

        loop {
            match Messages::parse(&read_stream(&mut bufreader)?) {
                Ok(messages) => match messages {
                    BoardInfo(board_info) => {
                        my_info.position = match my_info.id {
                            0 => board_info.player_position_0,
                            1 => board_info.player_position_1,
                            _ => unreachable!(),
                        };
                        board_state = board_info;
                    }
                    HandInfo(hand_info) => my_info.hand = hand_info.to_vec(),
                    DoPlay(_) => {
                        send_info(&mut bufwriter, &Evaluation::new())?;
                        act(&my_info, &board_state, &mut bufwriter)?;
                    }
                    ServerError(_) => {
                        print("エラーもらった")?;
                        act(&my_info, &board_state, &mut bufwriter)?;
                    }
                    Played(played) => algorithm::used_card(&mut cards, played),
                    RoundEnd(_round_end) => {
                        print("ラウンド終わり!")?;
                    }
                    GameEnd(_game_end) => {
                        break;
                    }
                },
                Err(e) => {
                    print("JSON解析できなかった")?;
                    print(format!("{}", e).as_str())?;
                }
            }
        }
    }
    Ok(())
}
