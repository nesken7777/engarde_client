use std::{
    collections::HashSet,
    fmt::Display,
    hash::RandomState,
    io::{self, BufReader, BufWriter},
    net::TcpStream,
};

use rurel::mdp::{Agent, State};
use serde::{Deserialize, Serialize};

use crate::{
    algorithm::{self, RestCards},
    print,
    protocol::{Evaluation, Messages, PlayAttack, PlayMovement, PlayerID},
    read_stream, send_info,
};

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Direction {
    Forward,
    Back,
}

impl Direction {
    pub fn denote(&self) -> u8 {
        match self {
            Self::Forward => 0,
            Self::Back => 1,
        }
    }
}

impl Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Forward => write!(f, "F"),
            Self::Back => write!(f, "B"),
        }
    }
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Movement {
    pub card: u8,
    pub direction: Direction,
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Attack {
    pub card: u8,
    pub quantity: u8,
}

#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Action {
    Move(Movement),
    Attack(Attack),
}

impl Action {
    pub fn as_index(&self) -> usize {
        match self {
            Action::Move(movement) => {
                let &Movement { card, direction } = movement;
                match direction {
                    Direction::Forward => card as usize - 1,
                    Direction::Back => 5 + (card as usize - 1),
                }
            }
            Action::Attack(attack) => {
                let &Attack { card, quantity } = attack;
                5 * 2 + 5 * (card as usize - 1) + (quantity as usize - 1)
            }
        }
    }
    pub fn from_index(idx: usize) -> Action {
        match idx {
            x @ 0..=4 => Action::Move(Movement {
                card: (x + 1) as u8,
                direction: Direction::Forward,
            }),
            x @ 5..=9 => Action::Move(Movement {
                card: (x - 5 + 1) as u8,
                direction: Direction::Back,
            }),
            x @ 10..=34 => {
                let x = x - 10;
                Action::Attack(Attack {
                    card: (x / 5 + 1) as u8,
                    quantity: (x % 5 + 1) as u8,
                })
            }
            _ => unreachable!(),
        }
    }
    pub fn get_movement(self) -> Option<Movement> {
        match self {
            Action::Move(movement) => Some(movement),
            Action::Attack(_) => None,
        }
    }
}

impl From<Action> for [f32; 35] {
    fn from(value: Action) -> Self {
        let mut arr = [0f32; 35];
        arr[value.as_index()] = 1.0;
        arr
    }
}

impl From<[f32; 35]> for Action {
    fn from(value: [f32; 35]) -> Self {
        match value
            .into_iter()
            .enumerate()
            .max_by(|&(_, x), &(_, y)| x.total_cmp(&y))
            .map(|(i, _)| i)
            .unwrap()
        {
            x @ 0..=4 => Action::Move(Movement {
                card: (x + 1) as u8,
                direction: Direction::Forward,
            }),
            x @ 5..=9 => Action::Move(Movement {
                card: (x - 5 + 1) as u8,
                direction: Direction::Back,
            }),
            x @ 10..=34 => {
                let x = x - 10;
                Action::Attack(Attack {
                    card: (x / 5 + 1) as u8,
                    quantity: (x % 5 + 1) as u8,
                })
            }
            _ => unreachable!(),
        }
    }
}

// Stateは、結果状態だけからその評価と次できる行動のリストを与える。
#[derive(PartialEq, Eq, Hash, Clone, Debug, Serialize, Deserialize)]
pub struct MyState {
    my_id: PlayerID,
    hands: Vec<u8>,
    cards: RestCards,
    p0_score: u32,
    p1_score: u32,
    p0_position: u8,
    p1_position: u8,
    game_end: bool,
}

impl MyState {
    pub fn hands(&self) -> &[u8] {
        &self.hands
    }

    pub fn my_id(&self) -> PlayerID {
        self.my_id
    }

    pub fn rest_cards(&self) -> RestCards {
        self.cards
    }

    pub fn p0_score(&self) -> u32 {
        self.p0_score
    }

    pub fn p1_score(&self) -> u32 {
        self.p1_score
    }

    pub fn p0_position(&self) -> u8 {
        self.p0_position
    }

    pub fn p1_position(&self) -> u8 {
        self.p1_position
    }

    pub fn game_end(&self) -> bool {
        self.game_end
    }

    // いやごめんてclippy
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        my_id: PlayerID,
        hands: Vec<u8>,
        cards: RestCards,
        p0_score: u32,
        p1_score: u32,
        p0_position: u8,
        p1_position: u8,
        game_end: bool,
    ) -> Self {
        Self {
            my_id,
            hands,
            cards,
            p0_score,
            p1_score,
            p0_position,
            p1_position,
            game_end,
        }
    }

    fn my_score(&self) -> u32 {
        match self.my_id {
            PlayerID::Zero => self.p0_score,
            PlayerID::One => self.p1_score,
        }
    }

    fn enemy_score(&self) -> u32 {
        match self.my_id {
            PlayerID::Zero => self.p1_score,
            PlayerID::One => self.p0_score,
        }
    }

    fn my_position(&self) -> u8 {
        match self.my_id {
            PlayerID::Zero => self.p0_position,
            PlayerID::One => self.p1_position,
        }
    }

    fn enemy_position(&self) -> u8 {
        match self.my_id {
            PlayerID::Zero => self.p1_position,
            PlayerID::One => self.p0_position,
        }
    }

    fn distance_from_center(&self) -> i8 {
        match self.my_id {
            PlayerID::Zero => 12 - self.p0_position as i8,
            PlayerID::One => self.p1_position as i8 - 12,
        }
    }
}

impl State for MyState {
    type A = Action;
    fn reward(&self) -> f64 {
        (self.my_score() as f64) - (self.enemy_score() as f64)
    }
    fn actions(&self) -> Vec<Action> {
        if self.game_end {
            return Vec::new();
        }
        fn attack_cards(hands: &[u8], card: u8) -> Option<Action> {
            let have = hands.iter().filter(|&&x| x == card).count();
            if have > 0 {
                Some(Action::Attack(Attack {
                    card,
                    quantity: have as u8,
                }))
            } else {
                None
            }
        }
        fn decide_moves(for_back: bool, for_forward: bool, card: u8) -> Vec<Action> {
            use Direction::*;
            match (for_back, for_forward) {
                (true, true) => vec![
                    Action::Move(Movement {
                        card,
                        direction: Back,
                    }),
                    Action::Move(Movement {
                        card,
                        direction: Forward,
                    }),
                ],
                (true, false) => vec![Action::Move(Movement {
                    card,
                    direction: Back,
                })],
                (false, true) => vec![Action::Move(Movement {
                    card,
                    direction: Forward,
                })],
                (false, false) => {
                    vec![]
                }
            }
        }
        let set = HashSet::<_, RandomState>::from_iter(self.hands.iter().cloned());
        match self.my_id {
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
// struct MyState {
//     my_id: PlayerID,
//     hands: Vec<u8>,
//     cards: RestCards,
//     p0_score: u32,
//     p1_score: u32,
//     my_position: u8,
//     enemy_position: u8,
//     game_end: bool,
// }
impl From<MyState> for [f32; 16] {
    fn from(value: MyState) -> Self {
        let id = vec![value.my_id.denote() as f32];
        let mut hands = value
            .hands
            .into_iter()
            .map(|x| x as f32)
            .collect::<Vec<f32>>();
        hands.resize(5, 0.0);
        let cards = value.cards.iter().map(|&x| x as f32).collect::<Vec<f32>>();
        let p0_score = vec![value.p0_score as f32];
        let p1_score = vec![value.p1_score as f32];
        let my_position = vec![value.p0_position as f32];
        let enemy_position = vec![value.p1_position as f32];
        let game_end = vec![value.game_end as u8 as f32];
        [
            id,
            hands,
            cards,
            p0_score,
            p1_score,
            my_position,
            enemy_position,
            game_end,
        ]
        .concat()
        .try_into()
        .unwrap()
    }
}

// エージェントは、先ほどの「できる行動のリスト」からランダムで選択されたアクションを実行し、状態(先ほどのState)を変更する。
pub struct MyAgent {
    reader: BufReader<TcpStream>,
    writer: BufWriter<TcpStream>,
    state: MyState,
}

impl MyAgent {
    pub fn new(
        id: PlayerID,
        hands: Vec<u8>,
        position_0: u8,
        position_1: u8,
        reader: BufReader<TcpStream>,
        writer: BufWriter<TcpStream>,
    ) -> Self {
        MyAgent {
            reader,
            writer,
            state: MyState {
                my_id: id,
                hands,
                cards: RestCards::new(),
                p0_score: 0,
                p1_score: 0,
                p0_position: position_0,
                p1_position: position_1,
                game_end: false,
            },
        }
    }
}

impl Agent<MyState> for MyAgent {
    fn current_state(&self) -> &MyState {
        &self.state
    }
    fn take_action(&mut self, action: &Action) {
        fn send_action(writer: &mut BufWriter<TcpStream>, action: &Action) -> io::Result<()> {
            match action {
                Action::Move(m) => send_info(writer, &PlayMovement::from_info(*m)),
                Action::Attack(a) => send_info(writer, &PlayAttack::from_info(*a)),
            }
        }
        use Messages::*;
        //selfキャプチャしたいからクロージャで書いてる
        let mut take_action_result = || -> io::Result<()> {
            loop {
                match Messages::parse(&read_stream(&mut self.reader)?) {
                    Ok(messages) => match messages {
                        BoardInfo(board_info) => {
                            (self.state.p0_position, self.state.p1_position) =
                                (board_info.player_position_0, board_info.player_position_1);
                        }
                        HandInfo(hand_info) => {
                            let mut hand_vec = hand_info.to_vec();
                            hand_vec.sort();
                            self.state.hands = hand_vec;
                            break;
                        }
                        Accept(_) => {}
                        DoPlay(_) => {
                            send_info(&mut self.writer, &Evaluation::new())?;
                            send_action(&mut self.writer, action)?;
                        }
                        ServerError(e) => {
                            print("エラーもらった")?;
                            print(format!("{:?}", e).as_str())?;
                            break;
                        }
                        Played(played) => {
                            algorithm::used_card(&mut self.state.cards, played);
                            break;
                        }
                        RoundEnd(round_end) => {
                            // print(
                            //     format!("ラウンド終わり! 勝者:{}", round_end.round_winner).as_str(),
                            // )?;
                            match round_end.round_winner {
                                0 => self.state.p0_score += 1,
                                1 => self.state.p1_score += 1,
                                _ => {}
                            }
                            self.state.cards = RestCards::new();
                            break;
                        }
                        GameEnd(game_end) => {
                            print(format!("ゲーム終わり! 勝者:{}", game_end.winner).as_str())?;
                            self.state.game_end = true;
                            break;
                        }
                    },
                    Err(e) => {
                        panic!("JSON解析できなかった {}", e);
                    }
                }
            }
            Ok(())
        };
        take_action_result().unwrap();
    }
}