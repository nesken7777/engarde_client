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
    strategy::{explore::ExplorationStrategy, learn::QLearning, terminate::SinkStates},
    AgentTrainer,
};
use serde::{Deserialize, Serialize};

use crate::{
    algorithm::{self, RestCards},
    connect,
    errors::Errors,
    print,
    protocol::{
        self, Action, BoardInfo,
        Direction::{Back, Forward},
        Evaluation, Messages, Movement, PlayAttack, PlayMovement, PlayerID, PlayerName,
    },
    read_keyboard, read_stream, send_info,
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
                            self.my_position.saturating_sub(card) >= 1,
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
                            self.my_position + card <= 23,
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
        fn send_action(writer: &mut BufWriter<TcpStream>, action: &Action) -> Result<(), Errors> {
            match action {
                Action::Move(m) => send_info(writer, &PlayMovement::from_info(*m)),
                Action::Attack(a) => send_info(writer, &PlayAttack::from_info(*a)),
            }
        }
        use Messages::*;
        //selfキャプチャしたいからクロージャで書いてる
        let mut take_action_result = || -> Result<(), Errors> {
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
                            self.state.hands = hand_info.to_vec();
                            break;
                        }
                        Accept(_) => {
                            break;
                        }
                        DoPlay(_) => {
                            send_info(&mut self.writer, &Evaluation::new())?;
                            send_action(&mut self.writer, action)?;
                            break;
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
                            print(
                                format!("ラウンド終わり! 勝者:{}", round_end.round_winner).as_str(),
                            )?;
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
                        print("JSON解析できなかった")?;
                        print(format!("{}", e).as_str())?;
                        break;
                    }
                }
            }
            Ok(())
        };
        match take_action_result() {
            Ok(_) => (),
            Err(e) => {
                print(format!("エラー発生:{}", e).as_str()).ok();
            }
        }
    }
}

pub fn ai_main() -> Result<(), Errors> {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12052);
    print("connect?")?;
    read_keyboard()?;
    let stream = TcpStream::connect(addr)?;
    let (mut bufreader, mut bufwriter) =
        (BufReader::new(stream.try_clone()?), BufWriter::new(stream));
    let id = connect(&mut bufreader)?;
    let player_name = PlayerName::new("qai".to_string());
    send_info(&mut bufwriter, &player_name)?;
    let _ = read_stream(&mut bufreader)?;

    // ここは、最初に自分が持ってる手札を取得するために、AIの行動じゃなしに情報を得なならん
    let mut board_info_init = BoardInfo::new();
    let hand_info = loop {
        let message = Messages::parse(&read_stream(&mut bufreader)?)?;
        if let Messages::HandInfo(hand_info) = message {
            break hand_info;
        } else if let Messages::BoardInfo(board_info) = message {
            board_info_init = board_info;
        }
    };

    // AI用エージェント作成
    let mut agent = MyAgent::new(
        id,
        hand_info.to_vec(),
        board_info_init.player_position_0,
        board_info_init.player_position_1,
        bufreader,
        bufwriter,
    );

    // ファイル読み込み
    let path = format!("learned{}.json", id.denote());
    let mut trainer = if let Ok(mut file) = OpenOptions::new().read(true).open(path) {
        let mut string = String::new();
        file.read_to_string(&mut string)?;
        let mut agent = AgentTrainer::new();
        let imported =
            serde_json::from_str::<HashMap<String, HashMap<String, f64>>>(string.trim())?;

        // ごめん、ここは後述の、文字列化したキーを構造体に戻す作業をしてます
        let imported = imported
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
            .collect::<Result<HashMap<MyState, _>, serde_json::Error>>()?;
        agent.import_state(imported);
        agent
    } else {
        AgentTrainer::new()
    };
    //トレーニング開始

    let mut trainer2 = AgentTrainer::new();
    trainer2.import_state(trainer.export_learned_values());
    trainer.train(
        &mut agent,
        &QLearning::new(0.2, 0.9, 0.0),
        &mut SinkStates {},
        &BestExploration::new(trainer2),
    );
    let exported = trainer.export_learned_values();

    // ごめん、ここはね、HashMapのままだとキーが文字列じゃないからjsonにできないんで、構造体のまま文字列化する処理です
    let converted = exported
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
    let filename = format!("learned{}.json", id.denote());
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(filename)?;
    file.write_all(serde_json::to_string(&converted)?.as_bytes())?;

    Ok(())
}
