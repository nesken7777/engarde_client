use std::{
    collections::{HashMap, HashSet},
    env::args,
    fs::OpenOptions,
    hash::RandomState,
    io::{self, BufReader, BufWriter, Read, Write},
    net::{SocketAddr, TcpStream},
    ops::Neg,
};

use rurel::{
    mdp::{Agent, State},
    strategy::{explore::ExplorationStrategy, learn::QLearning, terminate::SinkStates},
    AgentTrainer,
};
use serde::{Deserialize, Serialize};

use crate::{
    algorithm::{self, RestCards},
    get_id, print,
    protocol::{
        self, Action, BoardInfo,
        Direction::{Back, Forward},
        Evaluation, Messages, Movement, PlayAttack, PlayMovement, PlayerID, PlayerName,
    },
    read_stream, send_info,
};

struct BestExploration(AgentTrainer<MyState>);

impl BestExploration {
    pub fn new(trainer: AgentTrainer<MyState>) -> BestExploration {
        BestExploration(trainer)
    }
}

impl ExplorationStrategy<MyState> for BestExploration {
    fn pick_action(&self, agent: &mut dyn Agent<MyState>) -> <MyState as State>::A {
        match self.0.best_action(agent.current_state()) {
            None => agent.pick_random_action(),
            Some(action) => {
                println!("AIが決めた");
                agent.take_action(&action);
                action
            }
        }
    }
}

// Stateは、結果状態だけからその評価と次できる行動のリストを与える。
#[derive(PartialEq, Eq, Hash, Clone, Debug, Serialize, Deserialize)]
struct MyState {
    my_id: PlayerID,
    hands: Vec<u8>,
    cards: RestCards,
    p0_score: u32,
    p1_score: u32,
    my_position: u8,
    enemy_position: u8,
    game_end: bool,
}

impl MyState {
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

    fn distance_from_center(&self) -> i8 {
        match self.my_id {
            PlayerID::Zero => 12 - self.my_position as i8,
            PlayerID::One => self.my_position as i8 - 12,
        }
    }
}

impl State for MyState {
    type A = Action;
    fn reward(&self) -> f64 {
        // Negative Euclidean distance
        let distance = (self.enemy_position as i8 - self.my_position as i8).abs();
        let rokutonokyori = (6 - distance).abs();
        let point1 = (rokutonokyori as f64 * 20.0).powi(2).neg();
        let point2 = if distance < 6 { -100.0 } else { 0.0 };
        let point3 = {
            let factor = self.distance_from_center() as f64 * 10.0;
            factor.powi(2) * if factor < 0.0 { 1.0 } else { -1.0 }
        };
        print!("[{point1}, {point2}, {point3}]\r\n");
        let point4 = self.my_score() as f64 * 2000.0 - self.enemy_score() as f64 * 2000.0;
        point1 + point2 + point3 + point4
    }
    fn actions(&self) -> Vec<Action> {
        if self.game_end {
            return Vec::new();
        }
        fn attack_cards(hands: &[u8], card: u8) -> Option<Action> {
            let have = hands.iter().filter(|&&x| x == card).count();
            if have > 0 {
                Some(Action::Attack(protocol::Attack {
                    card,
                    quantity: have as u8,
                }))
            } else {
                None
            }
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
                            self.my_position.saturating_sub(card) >= 1,
                            self.my_position + card < self.enemy_position,
                            card,
                        )
                    })
                    .collect::<Vec<Action>>();

                [
                    moves,
                    attack_cards(
                        &self.hands,
                        self.enemy_position.checked_sub(self.my_position).unwrap(),
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
                            self.my_position + card <= 23,
                            self.my_position.saturating_sub(card) > self.enemy_position,
                            card,
                        )
                    })
                    .collect::<Vec<Action>>();

                [
                    moves,
                    attack_cards(
                        &self.hands,
                        self.my_position.checked_sub(self.enemy_position).unwrap(),
                    )
                    .into_iter()
                    .collect::<Vec<_>>(),
                ]
                .concat()
            }
        }
    }
}

// エージェントは、先ほどの「できる行動のリスト」からランダムで選択されたアクションを実行し、状態(先ほどのState)を変更する。
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
                p0_score: 0,
                p1_score: 0,
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
                            (self.state.my_position, self.state.enemy_position) =
                                match self.state.my_id {
                                    PlayerID::Zero => {
                                        (board_info.player_position_0, board_info.player_position_1)
                                    }
                                    PlayerID::One => {
                                        (board_info.player_position_1, board_info.player_position_0)
                                    }
                                };
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

pub fn ai_main() -> io::Result<()> {
    let id = (|| args().nth(1)?.parse::<u8>().ok())().unwrap_or(0);
    // ファイル読み込み
    let path = format!("learned{}.json", id);
    let mut learned_values = if let Ok(mut file) = OpenOptions::new().read(true).open(path) {
        let mut string = String::new();
        file.read_to_string(&mut string)?;
        let imported =
            serde_json::from_str::<HashMap<String, HashMap<String, f64>>>(string.trim())?;

        // ごめん、ここは後述の、文字列化したキーを構造体に戻す作業をしてます

        imported
            .into_iter()
            .map(|(k, v)| {
                let state = serde_json::from_str(&k)?;
                let action_map = v
                    .into_iter()
                    .map(|(k2, v2)| {
                        let action = serde_json::from_str(&k2)?;
                        Ok((action, v2))
                    })
                    .collect::<Result<HashMap<Action, f64>, serde_json::Error>>()?;
                Ok((state, action_map))
            })
            .collect::<Result<HashMap<MyState, _>, serde_json::Error>>()?
    } else {
        HashMap::new()
    };

    let loop_kaisuu = (|| args().nth(2)?.parse::<usize>().ok())().unwrap_or(1);

    for _ in 0..loop_kaisuu {
        let mut trainer = AgentTrainer::new();
        trainer.import_state(learned_values.clone());

        // 吐き出された学習内容を取り込む
        let mut trainer2 = AgentTrainer::new();
        trainer2.import_state(learned_values);

        let addr = SocketAddr::from(([127, 0, 0, 1], 12052));
        let stream = loop {
            if let Ok(stream) = TcpStream::connect(addr) {
                break stream;
            }
        };
        let (mut bufreader, mut bufwriter) =
            (BufReader::new(stream.try_clone()?), BufWriter::new(stream));
        let id = get_id(&mut bufreader)?;
        let player_name = PlayerName::new("qai".to_string());
        send_info(&mut bufwriter, &player_name)?;
        let _ = read_stream(&mut bufreader)?;

        // ここは、最初に自分が持ってる手札を取得するために、AIの行動じゃなしに情報を得なならん
        let mut board_info_init = BoardInfo::new();

        let hand_info = loop {
            match Messages::parse(&read_stream(&mut bufreader)?) {
                Ok(Messages::BoardInfo(board_info)) => {
                    board_info_init = board_info;
                }
                Ok(Messages::HandInfo(hand_info)) => {
                    break hand_info;
                }
                Ok(_) | Err(_) => {}
            }
        };
        let mut hand_vec = hand_info.to_vec();
        hand_vec.sort();
        // AI用エージェント作成
        let mut agent = MyAgent::new(
            id,
            hand_vec,
            board_info_init.player_position_0,
            board_info_init.player_position_1,
            bufreader,
            bufwriter,
        );

        //トレーニング開始
        trainer.train(
            &mut agent,
            &QLearning::new(0.2, 0.9, 0.0),
            &mut SinkStates {},
            &BestExploration::new(trainer2),
        );
        learned_values = trainer.export_learned_values();
    }
    // ごめん、ここはね、HashMapのままだとキーが文字列じゃないからjsonにできないんで、構造体のまま文字列化する処理です
    let converted = learned_values
        .into_iter()
        .map(|(k, v)| {
            let state_str = serde_json::to_string(&k)?;
            let action_str_map = v
                .into_iter()
                .map(|(k2, v2)| {
                    let action_str = serde_json::to_string(&k2)?;
                    Ok((action_str, v2))
                })
                .collect::<Result<HashMap<String, f64>, serde_json::Error>>()?;
            Ok((state_str, action_str_map))
        })
        .collect::<Result<HashMap<String, _>, serde_json::Error>>()?;
    let filename = format!("learned{}.json", id);
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(filename)?;
    file.write_all(serde_json::to_string(&converted)?.as_bytes())?;

    Ok(())
}
