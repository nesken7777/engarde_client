//! 対話環境で遊ぶクライアント

use engarde_client::{
    get_id, print,
    protocol::{BoardInfo, Evaluation, Messages, PlayerID, PlayerName},
    read_stream, send_info, Action, Attack, CardID, Direction, Maisuu, Movement, RestCards,
};
use std::{
    io::{self, stdin, BufReader, BufWriter},
    net::{SocketAddr, TcpStream},
};

fn read_keyboard() -> io::Result<String> {
    let mut word = String::new();
    stdin().read_line(&mut word)?;
    let response = word.trim().to_string();
    Ok(response)
}

fn ask_card(player: &PlayerProperty) -> io::Result<CardID> {
    loop {
        print("カードはどれにする?")?;
        let Ok(card) = read_keyboard()?.parse::<u8>() else {
            print("それ数字じゃないだろ")?;
            continue;
        };
        let Some(card) = CardID::from_u8(card) else {
            print("カード番号の範囲外だ")?;
            continue;
        };
        if player.hand.contains(&card) {
            break Ok(card);
        }
        print("そのカードは無いよ")?;
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
    Ok(Action::Move(Movement::new(card, direction)))
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
    use CantAttack::Lack;
    let card = CardID::from_u8(board.distance_between_enemy()).ok_or(Lack)?;
    let have =
        u8::try_from(player.hand.iter().filter(|&&x| x == card).count()).expect("u8の境界内");
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
            }
            print("そんなにたくさん持っていないですよ")?;
        }
    };
    Ok(Action::Attack(Attack::new(
        card,
        Maisuu::from_u8(quantity).expect("Maisuuの境界内"),
    )))
}

fn ask_action(player: &PlayerProperty, board: &BoardInfo) -> io::Result<Action> {
    print(format!("p0: {}, p1: {}", board.p0_position(), board.p1_position()))?;
    print(format!("手札:{:?}", player.hand))?;
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
) -> io::Result<()> {
    let evaluation = Evaluation::new();
    send_info(bufwriter, &evaluation)?;
    let action = ask_action(my_info, board_state)?;
    match action {
        Action::Move(movement) => {
            let i: usize = (movement.card().denote() - 1).into();
            cards[i] = cards[i].saturating_sub(Maisuu::ONE);
            // send_info(bufwriter, &PlayMovement::from_info(movement))?;
        }
        Action::Attack(attack) => {
            let i: usize = (attack.card().denote() - 1).into();
            cards[i] = cards[i].saturating_sub(attack.quantity().saturating_mul(2));
            // send_info(bufwriter, &PlayAttack::from_info(attack))?;
        }
    }
    Ok(())
}

struct PlayerProperty {
    pub id: PlayerID,
    pub hand: Vec<CardID>,
    pub position: u8,
}

impl PlayerProperty {
    pub fn new(id: PlayerID) -> Self {
        Self {
            id,
            hand: Vec::new(),
            position: 0,
        }
    }
}

fn main() -> io::Result<()> {
    // IPアドレスはいつか標準入力になると思います。
    let addr = SocketAddr::from(([127, 0, 0, 1], 12052));
    print("connect?")?;
    read_keyboard()?;
    let stream = TcpStream::connect(addr)?;
    let (mut bufreader, mut bufwriter) =
        (BufReader::new(stream.try_clone()?), BufWriter::new(stream));
    let id = get_id(&mut bufreader)?;
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
                    Messages::BoardInfo(board_info) => {
                        my_info.position = match my_info.id {
                            PlayerID::Zero => board_info.p0_position(),
                            PlayerID::One => board_info.p1_position(),
                        };
                        board_state = board_info;
                    }
                    Messages::HandInfo(hand_info) => my_info.hand = hand_info.to_vec(),
                    Messages::Accept(_) => (),
                    Messages::DoPlay(_) => act(&mut cards, &my_info, &board_state, &mut bufwriter)?,
                    Messages::ServerError(_) => {
                        print("エラーもらった")?;
                        act(&mut cards, &my_info, &board_state, &mut bufwriter)?;
                    }
                    Messages::Played(played) => cards.used_card(played.to_action()),
                    Messages::RoundEnd(_round_end) => {
                        print("ラウンド終わり!")?;
                        cards = RestCards::new();
                    }
                    Messages::GameEnd(_game_end) => {
                        break;
                    }
                },
                Err(e) => {
                    print("JSON解析できなかった")?;
                    print(format!("{e}"))?;
                }
            }
        }
    }
    Ok(())
}
