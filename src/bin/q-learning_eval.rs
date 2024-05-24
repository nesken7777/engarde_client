use num_traits::ToBytes;
use rurel::{
    mdp::{Agent, State},
    strategy::{explore::ExplorationStrategy, learn::QLearning, terminate::SinkStates},
    AgentTrainer,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    env::args,
    fs::OpenOptions,
    hash::RandomState,
    io::{self, BufReader, BufWriter, Read, Write},
    net::{SocketAddr, TcpStream},
};

use engarde_client::{
    algorithm::{self, RestCards},
    get_id, print,
    protocol::{
        self, Action, Attack, BoardInfo,
        Direction::{Back, Forward},
        Evaluation, Messages, Movement, PlayAttack, PlayMovement, PlayerID, PlayerName,
    },
    read_stream, send_info,
};

struct BestExploration(AgentTrainer<MyState>);

impl BestExploration {
     fn new(trainer: AgentTrainer<MyState>) -> BestExploration {
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
    p0_position: u8,
    p1_position: u8,
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
        let point3 = {
            let factor = self.distance_from_center() as f64 * 20.0;
            factor.powi(2) * if factor < 0.0 { 1.0 } else { -1.0 }
        };
        let point4 = (self.my_score() as f64 * 2000.0).powi(2)
            - (self.enemy_score() as f64 * 2000.0).powi(2);
        point3 + point4
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

struct LearnedValues(HashMap<MyState, HashMap<Action, f64>>);

impl LearnedValues {
    fn serialize(&self) -> Vec<u8> {
        let map_len = self.0.len();
        let state_map_bytes = self
            .0
            .iter()
            .flat_map(|(state, action_reward)| -> Vec<u8> {
                let mut hands = state.hands.clone();
                hands.resize(5, 0);
                let state_bytes = [
                    vec![state.my_id.denote()],
                    hands,
                    state.cards.to_vec(),
                    state.p0_score.to_le_bytes().to_vec(),
                    state.p1_score.to_le_bytes().to_vec(),
                    vec![state.p0_position],
                    vec![state.p1_position],
                    vec![state.game_end.into()],
                ]
                .concat();
                let act_rwd_len = action_reward.len();
                let action_reward_bytes = action_reward
                    .iter()
                    .flat_map(|(action, value)| -> Vec<u8> {
                        match action {
                            Action::Move(movement) => {
                                let action_bytes =
                                    vec![0, movement.card, movement.direction.denote()];
                                [action_bytes, value.to_le_bytes().to_vec()].concat()
                            }
                            Action::Attack(attack) => {
                                let action_bytes = vec![1, attack.card, attack.quantity];
                                [action_bytes, value.to_le_bytes().to_vec()].concat()
                            }
                        }
                    })
                    .collect::<Vec<u8>>();
                [state_bytes, vec![act_rwd_len as u8], action_reward_bytes].concat()
            })
            .collect::<Vec<u8>>();
        [map_len.to_le_bytes().to_vec(), state_map_bytes].concat()
    }
    fn deserialize(bytes: &[u8]) -> LearnedValues {
        let (map_len_bytes, state_map_bytes) = bytes.split_at(8);
        let map_len = usize::from_le_bytes(map_len_bytes.try_into().unwrap());
        let mut state_map: HashMap<MyState, HashMap<Action, f64>> = HashMap::new();
        let mut next_map = state_map_bytes;
        for _ in 0..map_len {
            //22がマジックナンバーすぎ
            let (state_bytes, next_map_) = next_map.split_at(22);
            // Stateを構築するぜ!
            let (my_id_bytes, state_rest) = state_bytes.split_at(1);
            let (hands_bytes, state_rest) = state_rest.split_at(5);
            let (cards_bytes, state_rest) = state_rest.split_at(5);
            let (p0_score_bytes, state_rest) = state_rest.split_at(4);
            let (p1_score_bytes, state_rest) = state_rest.split_at(4);
            let (p0_position_bytes, state_rest) = state_rest.split_at(1);
            let (p1_position_bytes, state_rest) = state_rest.split_at(1);
            let (game_end_bytes, _) = state_rest.split_at(1);

            let state = MyState {
                my_id: PlayerID::from_u8(my_id_bytes[0]).unwrap(),
                hands: hands_bytes
                    .iter()
                    .filter(|&&x| x != 0)
                    .copied()
                    .collect::<Vec<u8>>(),
                cards: RestCards::from_slice(cards_bytes),
                p0_score: u32::from_le_bytes(p0_score_bytes.try_into().unwrap()),
                p1_score: u32::from_le_bytes(p1_score_bytes.try_into().unwrap()),
                p0_position: p0_position_bytes[0],
                p1_position: p1_position_bytes[0],
                game_end: match game_end_bytes[0] {
                    0 => false,
                    1 => true,
                    _ => panic!(),
                },
            };

            let (act_rwd_len_bytes, next_map_) = next_map_.split_at(1);
            let act_rwd_len = act_rwd_len_bytes[0];
            let mut act_rwd_map: HashMap<Action, f64> = HashMap::new();
            next_map = next_map_;
            for _ in 0..act_rwd_len {
                let (action_bytes, next_map_) = next_map.split_at(1);
                let (card_bytes, next_map_) = next_map_.split_at(1);
                let (property_bytes, next_map_) = next_map_.split_at(1);
                let (value_bytes, next_map_) = next_map_.split_at(8);
                next_map = next_map_;
                let action = match action_bytes[0] {
                    0 => {
                        let direction = match property_bytes[0] {
                            0 => Forward,
                            1 => Back,
                            _ => unreachable!(),
                        };
                        Action::Move(Movement {
                            card: card_bytes[0],
                            direction,
                        })
                    }
                    1 => Action::Attack(Attack {
                        card: card_bytes[0],
                        quantity: property_bytes[0],
                    }),
                    _ => unreachable!(),
                };
                let value = f64::from_le_bytes(value_bytes.try_into().unwrap());
                act_rwd_map.insert(action, value);
            }
            state_map.insert(state, act_rwd_map);
        }
        LearnedValues(state_map)
    }

     fn get(self) -> HashMap<MyState, HashMap<Action, f64>> {
        self.0
    }

     fn from_map(map: HashMap<MyState, HashMap<Action, f64>>) -> Self {
        LearnedValues(map)
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

 fn main() -> io::Result<()> {
    let id = (|| args().nth(1)?.parse::<u8>().ok())().unwrap_or(0);
    // ファイル読み込み
    let path = format!("learned{}", id);
    let mut learned_values = if let Ok(mut file) = OpenOptions::new().read(true).open(path) {
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        LearnedValues::deserialize(&data).get()
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
            &QLearning::new(0.2, 0.7, 0.0),
            &mut SinkStates {},
            &BestExploration::new(trainer2),
        );
        learned_values = trainer.export_learned_values();
    }
    let learned_values = LearnedValues::from_map(learned_values);
    let bytes = learned_values.serialize();
    let filename = format!("learned{}", id);
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(filename)?;
    file.write_all(&bytes)?;

    Ok(())
}
