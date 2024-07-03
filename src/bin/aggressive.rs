//! ガチ恋距離戦法クライアント

use std::{
    cmp::Ordering,
    collections::HashSet,
    hash::RandomState,
    io::{self, BufReader, BufWriter},
    net::{SocketAddr, TcpStream},
};

use engarde_client::{
    get_id, print,
    protocol::{BoardInfo, Evaluation, Messages, PlayAttack, PlayMovement, PlayerID, PlayerName},
    read_stream, send_info, Action, Attack, CardID, Direction, Maisuu, Movement,
};

struct MyStateAlg {
    id: PlayerID,
    hands: Vec<CardID>,
    p0_position: u8,
    p1_position: u8,
}

impl MyStateAlg {
    fn new(id: PlayerID, hands: Vec<CardID>, p0_position: u8, p1_position: u8) -> Self {
        Self {
            id,
            hands,
            p0_position,
            p1_position,
        }
    }

    fn update_board(&mut self, board_info: &BoardInfo) {
        self.p0_position = board_info.p0_position();
        self.p1_position = board_info.p1_position();
    }

    fn update_hands(&mut self, hand_info: Vec<CardID>) {
        self.hands = hand_info;
        self.hands.sort();
    }

    fn actions(&self) -> Vec<Action> {
        fn attack_cards(hands: &[CardID], card: CardID) -> Option<Action> {
            let have = hands.iter().filter(|&&x| x == card).count();
            let have = u8::try_from(have).ok()?;
            let have = Maisuu::from_u8(have)?;
            (have > Maisuu::ZERO).then(|| Action::Attack(Attack::new(card, have)))
        }
        fn decide_moves(for_back: bool, for_forward: bool, card: CardID) -> Vec<Action> {
            use Direction::{Back, Forward};
            match (for_back, for_forward) {
                (true, true) => vec![
                    Action::Move(Movement::new(card, Back)),
                    Action::Move(Movement::new(card, Forward)),
                ],
                (true, false) => vec![Action::Move(Movement::new(card, Back))],
                (false, true) => vec![Action::Move(Movement::new(card, Forward))],
                (false, false) => {
                    vec![]
                }
            }
        }
        let set = self
            .hands
            .iter()
            .copied()
            .collect::<HashSet<_, RandomState>>();
        match self.id {
            PlayerID::Zero => {
                let moves = set
                    .into_iter()
                    .flat_map(|card| {
                        decide_moves(
                            self.p0_position.saturating_sub(card.denote()) >= 1,
                            self.p0_position + card.denote() < self.p1_position,
                            card,
                        )
                    })
                    .collect::<Vec<Action>>();
                let attack = (|| {
                    let n = self.p1_position.checked_sub(self.p0_position)?;
                    let card = CardID::from_u8(n)?;
                    attack_cards(&self.hands, card)
                })();
                [moves, attack.into_iter().collect::<Vec<_>>()].concat()
            }
            PlayerID::One => {
                let moves = set
                    .into_iter()
                    .flat_map(|card| {
                        decide_moves(
                            self.p1_position + card.denote() <= 23,
                            self.p1_position.saturating_sub(card.denote()) > self.p0_position,
                            card,
                        )
                    })
                    .collect::<Vec<Action>>();

                let attack = (|| {
                    let n = self.p1_position.checked_sub(self.p0_position)?;
                    let card = CardID::from_u8(n)?;
                    attack_cards(&self.hands, card)
                })();
                [moves, attack.into_iter().collect::<Vec<_>>()].concat()
            }
        }
    }
}

fn act(state: &MyStateAlg) -> Option<Action> {
    let mut actions = state.actions();
    actions.sort_unstable_by(|action1, action2| match action1 {
        Action::Move(movement1) => match action2 {
            Action::Move(movement2) => match movement1.direction() {
                Direction::Forward => match movement2.direction() {
                    Direction::Forward => movement2.card().cmp(&movement1.card()),
                    Direction::Back => Ordering::Less,
                },
                Direction::Back => match movement2.direction() {
                    Direction::Forward => Ordering::Greater,
                    Direction::Back => movement1.card().cmp(&movement2.card()),
                },
            },
            Action::Attack(_) => Ordering::Greater,
        },
        Action::Attack(_) => Ordering::Less,
    });
    actions.first().copied()
}

fn send_action(writer: &mut BufWriter<TcpStream>, action: Action) -> io::Result<()> {
    match action {
        Action::Move(m) => send_info(writer, &PlayMovement::from_info(m)),
        Action::Attack(a) => send_info(writer, &PlayAttack::from_info(a)),
    }
}

fn main() -> io::Result<()> {
    // IPアドレスはいつか標準入力になると思います。
    let addr = SocketAddr::from(([127, 0, 0, 1], 12052));
    let stream = TcpStream::connect(addr)?;
    let (mut bufreader, mut bufwriter) =
        (BufReader::new(stream.try_clone()?), BufWriter::new(stream));
    let id = get_id(&mut bufreader)?;
    {
        let player_name = PlayerName::new("algorithm".to_string());
        send_info(&mut bufwriter, &player_name)?;
        let _ = read_stream(&mut bufreader)?;
    }
    {
        let mut state = MyStateAlg::new(id, vec![], 1, 23);
        loop {
            match Messages::parse(&read_stream(&mut bufreader)?) {
                Ok(messages) => match messages {
                    Messages::BoardInfo(board_info) => {
                        state.update_board(&board_info);
                    }
                    Messages::HandInfo(hand_info) => {
                        state.update_hands(hand_info.to_vec());
                    }
                    Messages::Accept(_) => (),
                    Messages::DoPlay(_) => {
                        let action = act(&state).unwrap_or_else(|| panic!("行動決定不能"));
                        send_info(&mut bufwriter, &Evaluation::new())?;
                        send_action(&mut bufwriter, action)?;
                    }
                    Messages::ServerError(e) => {
                        print("エラーもらった")?;
                        print(format!("{e:?}"))?;
                        break;
                    }
                    Messages::Played(_) => {}
                    Messages::RoundEnd(_round_end) => {}
                    Messages::GameEnd(game_end) => {
                        if game_end.winner() == state.id.denote() {
                            print("aggressiveの勝ち")?;
                        }
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
