use std::{
    collections::HashSet,
    hash::RandomState,
    io::{self, BufReader, BufWriter},
    net::{SocketAddr, TcpStream},
};

use engarde_client::{
    algorithm::ProbabilityTable,
    algorithm2::{initial_move, middle_move, AcceptableNumbers},
    get_id, print,
    protocol::{BoardInfo, HandInfo, Messages, PlayAttack, PlayMovement, PlayerID, PlayerName},
    read_stream, send_info,
    states::{used_card, Action, Attack, Direction, Movement, RestCards},
};
use rand::{seq::SliceRandom, thread_rng};

struct MyStateAlg {
    id: PlayerID,
    hands: Vec<u8>,
    cards: RestCards,
    p0_position: u8,
    p1_position: u8,
}

impl MyStateAlg {
    fn new(
        id: PlayerID,
        hands: Vec<u8>,
        cards: RestCards,
        p0_position: u8,
        p1_position: u8,
    ) -> Self {
        Self {
            id,
            hands,
            cards,
            p0_position,
            p1_position,
        }
    }

    fn update_board(&mut self, board_info: BoardInfo) {
        self.p0_position = board_info.p0_position();
        self.p1_position = board_info.p1_position();
    }

    fn update_hands(&mut self, hand_info: Vec<u8>) {
        self.hands = hand_info;
        self.hands.sort();
    }

    fn actions(&self) -> Vec<Action> {
        fn attack_cards(hands: &[u8], card: u8) -> Option<Action> {
            let have = hands.iter().filter(|&&x| x == card).count();
            if have > 0 {
                Some(Action::Attack(Attack::new(card, have as u8)))
            } else {
                None
            }
        }
        fn decide_moves(for_back: bool, for_forward: bool, card: u8) -> Vec<Action> {
            use Direction::*;
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
        let set = HashSet::<_, RandomState>::from_iter(self.hands.iter().cloned());
        match self.id {
            PlayerID::Zero => {
                let moves = set
                    .into_iter()
                    .flat_map(|card| {
                        decide_moves(
                            self.p0_position.saturating_sub(card) >= 1,
                            self.p0_position + card < self.p1_position,
                            card,
                        )
                    })
                    .collect::<Vec<Action>>();

                [
                    moves,
                    attack_cards(
                        &self.hands,
                        self.p1_position.checked_sub(self.p0_position).unwrap(),
                    )
                    .into_iter()
                    .collect::<Vec<_>>(),
                ]
                .concat()
            }
            PlayerID::One => {
                let moves = set
                    .into_iter()
                    .flat_map(|card| {
                        decide_moves(
                            self.p1_position + card <= 23,
                            self.p1_position.saturating_sub(card) > self.p0_position,
                            card,
                        )
                    })
                    .collect::<Vec<Action>>();

                [
                    moves,
                    attack_cards(
                        &self.hands,
                        self.p1_position.checked_sub(self.p0_position).unwrap(),
                    )
                    .into_iter()
                    .collect::<Vec<_>>(),
                ]
                .concat()
            }
        }
    }
}

fn act(state: &MyStateAlg) -> Action {
    let card_map = card_map_from_hands(&state.hands);
    let distance = state.p1_position - state.p0_position;
    let acceptable = AcceptableNumbers::new(&card_map, &state.cards, distance);
    let table = ProbabilityTable::new(25 - state.cards.iter().sum::<u8>(), &state.cards);
    let initial = initial_move(&card_map, distance, acceptable).ok();
    let middle = middle_move(&card_map, distance, &state.cards, &table);
    let det = initial.or(middle);
    det.unwrap_or({
        let mut rng = thread_rng();
        let actions = state.actions();
        actions.choose(&mut rng).copied().unwrap()
    })
}

/// 手札からカード番号-枚数表にする
fn card_map_from_hands(hands: &[u8]) -> [u8; 5] {
    (0..5)
        .map(|x| hands.iter().filter(|&&y| x == y).count() as u8)
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}

fn send_action(writer: &mut BufWriter<TcpStream>, action: &Action) -> io::Result<()> {
    match action {
        Action::Move(m) => send_info(writer, &PlayMovement::from_info(*m)),
        Action::Attack(a) => send_info(writer, &PlayAttack::from_info(*a)),
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
        let mut state = MyStateAlg::new(id, vec![], RestCards::new(), 1, 23);
        loop {
            match Messages::parse(&read_stream(&mut bufreader)?) {
                Ok(messages) => match messages {
                    Messages::BoardInfo(board_info) => {
                        state.update_board(board_info);
                    }
                    Messages::HandInfo(hand_info) => {
                        state.update_hands(hand_info.to_vec());
                    }
                    Messages::Accept(_) => (),
                    Messages::DoPlay(_) => {
                        let action = act(&state);
                        send_action(&mut bufwriter, &action)?;
                        used_card(&mut state.cards, action);
                    }
                    Messages::ServerError(e) => {
                        print("エラーもらった")?;
                        print(format!("{:?}", e).as_str())?;
                        break;
                    }
                    Messages::Played(played) => used_card(&mut state.cards, played.to_action()),
                    Messages::RoundEnd(_round_end) => {
                        print("ラウンド終わり!")?;
                        state.cards = RestCards::new();
                    }
                    Messages::GameEnd(_game_end) => {
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
