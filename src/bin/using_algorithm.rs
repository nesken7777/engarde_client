//! アルゴリズムによって動くクライアント

use std::{
    collections::HashSet,
    hash::RandomState,
    io::{self, BufReader, BufWriter},
    net::{SocketAddr, TcpStream},
};

use engarde_client::{
    algorithm::{card_map_from_hands, ProbabilityTable},
    algorithm2::{initial_move, middle_move, AcceptableNumbers},
    get_id, print,
    protocol::{BoardInfo, Messages, PlayAttack, PlayMovement, PlayerID, PlayerName},
    read_stream, send_info, Action, Attack, CardID, Direction, Maisuu, Movement, RestCards,
};
use rand::{seq::SliceRandom, thread_rng};

struct MyStateAlg {
    id: PlayerID,
    hands: Vec<CardID>,
    cards: RestCards,
    p0_position: u8,
    p1_position: u8,
}

impl MyStateAlg {
    fn new(
        id: PlayerID,
        hands: Vec<CardID>,
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
            let have = Maisuu::new(have)?;
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

fn act(state: &MyStateAlg) -> Action {
    let card_map = card_map_from_hands(&state.hands);
    let distance = state.p1_position - state.p0_position;
    let acceptable = AcceptableNumbers::new(card_map, state.cards, distance);
    let table = ProbabilityTable::new(
        25 - state
            .cards
            .iter()
            .map(engarde_client::Maisuu::denote)
            .sum::<u8>(),
        &state.cards,
    );
    let initial = initial_move(&card_map, distance, &acceptable).ok();
    let middle = middle_move(&state.hands, distance, state.cards, &table);
    let det = initial.or(middle);
    det.unwrap_or({
        let mut rng = thread_rng();
        let actions = state.actions();
        actions.choose(&mut rng).copied().unwrap()
    })
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
        let mut state = MyStateAlg::new(id, vec![], RestCards::new(), 1, 23);
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
                        let action = act(&state);
                        send_action(&mut bufwriter, action)?;
                        state.cards.used_card(action);
                    }
                    Messages::ServerError(e) => {
                        print("エラーもらった")?;
                        print(format!("{e:?}").as_str())?;
                        break;
                    }
                    Messages::Played(played) => state.cards.used_card(played.to_action()),
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
                    print(format!("{e}").as_str())?;
                }
            }
        }
    }
    Ok(())
}
