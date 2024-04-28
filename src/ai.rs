use std::{
    collections::{HashMap, HashSet},
    fs::OpenOptions,
    hash::RandomState,
    io::{BufReader, BufWriter, Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    ops::Neg,
};

use rurel::{
    mdp::{Agent, State},
    strategy::{explore::RandomExploration, learn::QLearning, terminate::SinkStates},
    AgentTrainer,
};
use serde::{Deserialize, Serialize};

use crate::{
    algorithm::{self, RestCards},
    connect,
    errors::Errors,
    protocol::{
        self, Action, BoardInfo,
        Direction::{Back, Forward},
        Evaluation, Messages, Movement, PlayAttack, PlayMovement, PlayerID, PlayerName,
    },
    read_keyboard, read_stream, send_info,
};

#[derive(PartialEq, Eq, Hash, Clone, Debug, Serialize, Deserialize)]
struct MyState {
    my_id: PlayerID,
    hands: Vec<u8>,
    cards: RestCards,
    my_position: u8,
    enemy_position: u8,
    game_end: bool,
}

impl State for MyState {
    type A = Action;
    fn reward(&self) -> f64 {
        // Negative Euclidean distance
        let distance = (self.enemy_position as i8 - self.my_position as i8).abs();
        let rokutonokyori = (6 - distance).abs();
        let point1 = rokutonokyori.neg() as f64;
        let point2 = if distance < 6 { -1.0 } else { 0.0 };
        [point1, point2].into_iter().sum()
    }
    fn actions(&self) -> Vec<Action> {
        if self.game_end {
            return Vec::new();
        }
        fn attack_cards(hands: &[u8], card: u8) -> Vec<Action> {
            let have = hands.iter().filter(|&&x| x == card).count();
            (1..=have)
                .map(|x| {
                    Action::Attack(protocol::Attack {
                        card,
                        quantity: x as u8,
                    })
                })
                .collect()
        }
        fn decide_moves(for_back: bool, for_forward: bool, card: u8) -> Vec<Action> {
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
                            self.my_position.saturating_sub(card) > 0,
                            self.my_position + card < self.enemy_position,
                            card,
                        )
                    })
                    .collect::<Vec<Action>>();

                [
                    moves,
                    attack_cards(&self.hands, self.enemy_position - self.my_position),
                ]
                .concat()
            }
            PlayerID::One => {
                let moves = set
                    .into_iter()
                    .flat_map(|card| {
                        decide_moves(
                            self.my_position + card < 23,
                            self.my_position - card > self.enemy_position,
                            card,
                        )
                    })
                    .collect::<Vec<Action>>();

                [
                    moves,
                    attack_cards(&self.hands, self.my_position - self.enemy_position),
                ]
                .concat()
            }
        }
    }
}

struct MyAgent {
    reader: BufReader<TcpStream>,
    writer: BufWriter<TcpStream>,
    state: MyState,
}

impl MyAgent {
    fn new(
        id: PlayerID,
        hands: Vec<u8>,
        position_0: u8,
        position_1: u8,
        reader: BufReader<TcpStream>,
        writer: BufWriter<TcpStream>,
    ) -> Self {
        let (my_position, enemy_position) = match id {
            PlayerID::Zero => (position_0, position_1),
            PlayerID::One => (position_1, position_0),
        };
        MyAgent {
            reader,
            writer,
            state: MyState {
                my_id: id,
                hands,
                cards: RestCards::new(),
                my_position,
                enemy_position,
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
        fn send_action(writer: &mut BufWriter<TcpStream>, action: &Action) {
            match action {
                Action::Move(m) => send_info(writer, &PlayMovement::from_info(*m)).unwrap(),
                Action::Attack(a) => send_info(writer, &PlayAttack::from_info(*a)).unwrap(),
            }
        }
        use Messages::*;
        match Messages::parse(&read_stream(&mut self.reader).unwrap()) {
            Ok(messages) => match messages {
                BoardInfo(board_info) => {
                    (self.state.my_position, self.state.enemy_position) = match self.state.my_id {
                        PlayerID::Zero => {
                            (board_info.player_position_0, board_info.player_position_1)
                        }
                        PlayerID::One => {
                            (board_info.player_position_1, board_info.player_position_0)
                        }
                    };
                }
                HandInfo(hand_info) => self.state.hands = hand_info.to_vec(),
                Accept(_) => (),
                DoPlay(_) => {
                    send_info(&mut self.writer, &Evaluation::new()).unwrap();
                    send_action(&mut self.writer, action);
                }
                ServerError(e) => {
                    println!("エラーもらった");
                    println!("{:?}", e);
                }
                Played(played) => algorithm::used_card(&mut self.state.cards, played),
                RoundEnd(round_end) => {
                    println!("ラウンド終わり! 勝者:{}", round_end.round_winner);
                    self.state.cards = RestCards::new();
                }
                GameEnd(game_end) => {
                    println!("ゲーム終わり! 勝者:{}", game_end.winner);
                    self.state.game_end = true;
                }
            },
            Err(e) => {
                println!("JSON解析できなかった");
                println!("{}", e);
            }
        }
    }
}

pub fn ai_main() -> Result<(), Errors> {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12052);
    println!("connect?");
    read_keyboard()?;
    let stream = TcpStream::connect(addr)?;
    let (mut bufreader, mut bufwriter) =
        (BufReader::new(stream.try_clone()?), BufWriter::new(stream));
    let id = connect(&mut bufreader)?;
    let player_name = PlayerName::new("qai".to_string());
    send_info(&mut bufwriter, &player_name)?;
    let _ = read_stream(&mut bufreader)?;
    let mut board_info_init = BoardInfo::new();
    let hand_info = loop {
        let message = Messages::parse(&read_stream(&mut bufreader)?)?;
        if let Messages::HandInfo(hand_info) = message {
            break hand_info;
        } else if let Messages::BoardInfo(board_info) = message {
            board_info_init = board_info;
        }
    };
    let mut agent = MyAgent::new(
        id,
        hand_info.to_vec(),
        board_info_init.player_position_0,
        board_info_init.player_position_1,
        bufreader,
        bufwriter,
    );
    let path = format!("learned{}.json", id.denote());
    let mut trainer = if let Ok(mut file) = OpenOptions::new().read(true).open(path) {
        let mut string = String::new();
        file.read_to_string(&mut string)?;
        let mut agent = AgentTrainer::new();
        let imported =
            serde_json::from_str::<HashMap<String, HashMap<String, f64>>>(string.trim())?;
        let imported = imported
            .into_iter()
            .map(|(k, v)| {
                (
                    serde_json::from_str(&k).unwrap(),
                    v.into_iter()
                        .map(|(k2, v2)| (serde_json::from_str(&k2).unwrap(), v2))
                        .collect::<HashMap<Action, f64>>(),
                )
            })
            .collect::<HashMap<MyState, _>>();
        agent.import_state(imported);
        agent
    } else {
        AgentTrainer::new()
    };
    trainer.train(
        &mut agent,
        &QLearning::new(0.2, 0.9, 0.0),
        &mut SinkStates {},
        &RandomExploration::new(),
    );
    let filename = format!("learned{}.json", id.denote());
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(filename)?;
    let exported = trainer.export_learned_values();
    let converted = exported
        .into_iter()
        .map(|(k, v)| {
            (
                serde_json::to_string(&k).unwrap(),
                v.into_iter()
                    .map(|(k2, v2)| (serde_json::to_string(&k2).unwrap(), v2))
                    .collect::<HashMap<_, _>>(),
            )
        })
        .collect::<HashMap<_, _>>();
    file.write_all(serde_json::to_string(&converted)?.as_bytes())?;

    Ok(())
}
