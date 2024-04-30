mod algorithm;
mod errors;
mod protocol;
use algorithm::RestCards;
use errors::Errors;
use protocol::{
    Action, Attack, BoardInfo, ConnectionStart, Direction, Evaluation, Messages, Movement,
    PlayAttack, PlayMovement, PlayerID, PlayerName, PlayerProperty,
};
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

fn read_keyboard() -> io::Result<String> {
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

fn connect<T>(bufreader: &mut BufReader<T>) -> Result<PlayerID, Errors>
where
    T: Read,
{
    let string = read_stream(bufreader)?;
    let connection_start = serde_json::from_str::<ConnectionStart>(&string)?;
    Ok(connection_start.client_id)
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

fn ask_card(player: &PlayerProperty) -> io::Result<u8> {
    loop {
        print("カードはどれにする?")?;
        let Ok(card) = read_keyboard()?.parse::<u8>() else {
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

fn ask_movement(player: &PlayerProperty) -> io::Result<Action> {
    let card = ask_card(player)?;
    let direction = loop {
        print("どっち向きにする?")?;
        let string = read_keyboard()?;
        match string.as_str() {
            "F" => break Direction::Forward,
            "B" => break Direction::Back,
            _ => {
                print("その方向は無いよ")?;
            }
        }
    };
    Ok(Action::Move(Movement { card, direction }))
}

enum CantAttack {
    IO(io::Error),
    Lack,
}

impl From<io::Error> for CantAttack {
    fn from(value: io::Error) -> Self {
        Self::IO(value)
    }
}

fn ask_attack(player: &PlayerProperty, board: &BoardInfo) -> Result<Action, CantAttack> {
    use CantAttack::*;
    let card = board.distance_between_enemy();
    let have = player.hand.iter().filter(|&&x| x == card).count() as u8;
    if have == 0 {
        return Err(Lack);
    }
    let quantity = {
        loop {
            print("何枚使う?")?;
            let Ok(quantity) = read_keyboard()?.parse::<u8>() else {
                print("それ数字じゃないですよ")?;
                continue;
            };
            if quantity <= have {
                break quantity;
            } else {
                print("そんなにたくさん持っていないですよ")?;
            }
        }
    };
    Ok(Action::Attack(Attack { card, quantity }))
}

fn ask_action(player: &PlayerProperty, board: &BoardInfo) -> io::Result<Action> {
    print(
        format!(
            "p0: {}, p1: {}",
            board.player_position_0, board.player_position_1
        )
        .as_str(),
    )?;
    print(format!("手札:{:?}", player.hand).as_str())?;
    loop {
        print("どっちのアクションにする?")?;
        let string = read_keyboard()?;
        match string.as_str() {
            "M" => break ask_movement(player),
            "A" => match ask_attack(player, board) {
                Ok(attack) => break Ok(attack),
                Err(e) => match e {
                    CantAttack::IO(e) => break Err(e),
                    CantAttack::Lack => {
                        print("アタックはできないよ")?;
                    }
                },
            },
            _ => {
                print("その行動は無いよ")?;
            }
        }
    }
}

fn act(
    cards: &mut RestCards,
    my_info: &PlayerProperty,
    board_state: &BoardInfo,
    bufwriter: &mut BufWriter<TcpStream>,
) -> Result<(), Errors> {
    let evaluation = Evaluation::new();
    send_info(bufwriter, &evaluation)?;
    let action = ask_action(my_info, board_state)?;
    match action {
        Action::Move(movement) => {
            cards[(movement.card - 1) as usize] -= 1;
            send_info(bufwriter, &PlayMovement::from_info(movement))?;
        }
        Action::Attack(attack) => {
            cards[(attack.card - 1) as usize] =
                cards[(attack.card - 1) as usize].saturating_sub(attack.quantity * 2);
            send_info(bufwriter, &PlayAttack::from_info(attack))?;
        }
    }
    Ok(())
}

fn main() -> Result<(), Errors> {
    // IPアドレスはいつか標準入力になると思います。
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12052);
    print("connect?")?;
    read_keyboard()?;
    let stream = TcpStream::connect(addr)?;
    let (mut bufreader, mut bufwriter) =
        (BufReader::new(stream.try_clone()?), BufWriter::new(stream));
    let id = connect(&mut bufreader)?;
    let mut my_info = PlayerProperty::new(id);
    {
        // ここはどうする?標準入力にする?
        print("名前を入力")?;
        let name = read_keyboard()?;
        let player_name = PlayerName::new(name);
        send_info(&mut bufwriter, &player_name)?;
        let _ = read_stream(&mut bufreader)?;
    }
    {
        let mut board_state = BoardInfo::new();
        let mut cards = RestCards::new();
        loop {
            match Messages::parse(&read_stream(&mut bufreader)?) {
                Ok(messages) => match messages {
                    BoardInfo(board_info) => {
                        my_info.position = match my_info.id {
                            PlayerID::Zero => board_info.player_position_0,
                            PlayerID::One => board_info.player_position_1,
                        };
                        board_state = board_info;
                    }
                    HandInfo(hand_info) => my_info.hand = hand_info.to_vec(),
                    Accept(_) => (),
                    DoPlay(_) => act(&mut cards, &my_info, &board_state, &mut bufwriter)?,
                    ServerError(_) => {
                        print("エラーもらった")?;
                        act(&mut cards, &my_info, &board_state, &mut bufwriter)?;
                    }
                    Played(played) => algorithm::used_card(&mut cards, played),
                    RoundEnd(_round_end) => {
                        print("ラウンド終わり!")?;
                        cards = RestCards::new();
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
