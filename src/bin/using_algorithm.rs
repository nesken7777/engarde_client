//! アルゴリズムによって動くクライアント

use std::{
    cmp::Ordering,
    collections::HashSet,
    hash::RandomState,
    io::{self, BufReader, BufWriter},
    net::{SocketAddr, SocketAddrV4, TcpStream},
};

use engarde_client::{
    algorithm::{card_map_from_hands, safe_possibility, ProbabilityTable},
    algorithm2::{initial_move, middle_move, AcceptableNumbers},
    get_id, print,
    protocol::{BoardInfo, Evaluation, Messages, PlayAttack, PlayMovement, PlayerID, PlayerName},
    read_stream, send_info, Action, Attack, CardID, Direction, Maisuu, Movement, UsedCards,
};

use clap::Parser;
use num_rational::Ratio;
use num_traits::Zero;

struct MyStateAlg {
    id: PlayerID,
    hands: Vec<CardID>,
    used: UsedCards,
    p0_position: u8,
    p1_position: u8,
}

impl MyStateAlg {
    fn new(
        id: PlayerID,
        hands: Vec<CardID>,
        used: UsedCards,
        p0_position: u8,
        p1_position: u8,
    ) -> Self {
        Self {
            id,
            hands,
            used,
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

    fn distance_opposite(&self) -> u8 {
        self.p1_position - self.p0_position
    }

    fn to_evaluation(&self) -> Evaluation {
        let actions = self
            .actions()
            .into_iter()
            .filter(|action| !matches!(action, Action::Attack(_)))
            .collect::<Vec<Action>>();
        let card_map = card_map_from_hands(&self.hands).expect("安心して");
        let distance = self.distance_opposite();
        let rest_cards = self.used.to_restcards(card_map);
        let hands = &self.hands;
        let table = &ProbabilityTable::new(&self.used.to_restcards(card_map));
        let safe_sum = actions
            .iter()
            .map(|&action| {
                safe_possibility(distance, rest_cards, hands, table, action)
                    .unwrap_or(Ratio::<u64>::zero())
            })
            .sum::<Ratio<u64>>();
        let mut evaluation_set = Evaluation::new();
        if safe_sum == Ratio::zero() {
            return evaluation_set;
        }
        actions
            .into_iter()
            .map(|action| {
                (
                    action,
                    safe_possibility(distance, rest_cards, hands, table, action)
                        .unwrap_or(Ratio::zero())
                        / safe_sum,
                )
            })
            .for_each(|(action, eval)| evaluation_set.update(action, eval));
        evaluation_set
    }
}

fn act(state: &MyStateAlg) -> Option<Action> {
    let card_map = card_map_from_hands(&state.hands)?;
    let distance = state.p1_position - state.p0_position;
    let restcard = state.used.to_restcards(card_map);
    let acceptable = AcceptableNumbers::new(card_map, restcard, distance);
    let table = ProbabilityTable::new(&restcard);
    let initial = initial_move(&card_map, distance, &acceptable).ok();
    let middle = middle_move(&state.hands, distance, restcard, &table);
    let det = initial.or(middle);
    Some(det.unwrap_or({
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
        actions.first().copied()?
    }))
}

fn send_action(writer: &mut BufWriter<TcpStream>, action: Action) -> io::Result<()> {
    match action {
        Action::Move(m) => send_info(writer, &PlayMovement::from_info(m)),
        Action::Attack(a) => send_info(writer, &PlayAttack::from_info(a)),
    }
}

#[derive(Parser, Debug)]
struct Arguments {
    #[arg(long, short, default_value_t = String::from("127.0.0.1"))]
    ip: String,
    #[arg(long, short, default_value_t = String::from("12052"))]
    port: String,
}

fn main() -> io::Result<()> {
    let args = Arguments::parse();
    let ip = format!("{}:{}", args.ip, args.port)
        .parse::<SocketAddrV4>()
        .expect("有効なIPアドレスではありません");
    // IPアドレスはいつか標準入力になると思います。
    // let addr = SocketAddr::from(([127, 0, 0, 1], 12052));
    let addr = ip;
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
        let mut state = MyStateAlg::new(id, vec![], UsedCards::new(), 1, 23);
        loop {
            let messages = Messages::parse(&read_stream(&mut bufreader)?).expect("JSON解析失敗");
            match messages {
                Messages::BoardInfo(board_info) => {
                    state.update_board(&board_info);
                }
                Messages::HandInfo(hand_info) => {
                    state.update_hands(hand_info.to_vec());
                }
                Messages::Accept(_) => (),
                Messages::DoPlay(_) => {
                    let action = act(&state).unwrap_or_else(|| panic!("行動決定不能"));
                    send_info(&mut bufwriter, &state.to_evaluation())?;
                    send_action(&mut bufwriter, action)?;
                    state.used.used_action(action);
                }
                Messages::ServerError(e) => {
                    print("エラーもらった")?;
                    print(format!("{e:?}"))?;
                    break;
                }
                Messages::Played(played) => state.used.used_action(played.to_action()),
                Messages::RoundEnd(_round_end) => {
                    state.used = UsedCards::new();
                }
                Messages::GameEnd(game_end) => {
                    if game_end.winner() == state.id.denote() {
                        print("algorithmの勝ち")?;
                    }
                    break;
                }
            }
        }
    }
    Ok(())
}
