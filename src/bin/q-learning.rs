use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::{self, BufReader, BufWriter, Read, Write},
    net::{SocketAddr, TcpStream},
};

use clap::{Parser, ValueEnum};
use rurel::{
    mdp::{Agent, State},
    strategy::{
        explore::{ExplorationStrategy, RandomExploration},
        learn::QLearning,
        terminate::SinkStates,
    },
    AgentTrainer,
};

use engarde_client::{
    get_id,
    protocol::{BoardInfo, CardID, Messages, PlayerID, PlayerName},
    read_stream, send_info,
    states::{Action, Attack, Direction, Movement, MyAgent, MyState, RestCards},
};

const DESERIALIZE_ERROR_MESSAGE: &str = "デシリアライズ失敗:数がCardIDの範囲外";

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

struct LearnedValues(HashMap<MyState, HashMap<Action, f64>>);

impl LearnedValues {
    fn serialize(&self) -> Vec<u8> {
        let map_len = self.0.len();
        let state_map_bytes = self
            .0
            .iter()
            .flat_map(|(state, action_reward)| -> Vec<u8> {
                let mut hands = state
                    .hands()
                    .to_vec()
                    .into_iter()
                    .map(|x| x.denote())
                    .collect::<Vec<u8>>();
                hands.resize(5, 0);
                let state_bytes = [
                    vec![state.my_id().denote()],
                    hands,
                    state.rest_cards().to_vec(),
                    state.p0_score().to_le_bytes().to_vec(),
                    state.p1_score().to_le_bytes().to_vec(),
                    vec![state.p0_position()],
                    vec![state.p1_position()],
                    vec![state.game_end().into()],
                ]
                .concat();
                let act_rwd_len = action_reward.len();
                let action_reward_bytes = action_reward
                    .iter()
                    .flat_map(|(action, value)| -> Vec<u8> {
                        match action {
                            Action::Move(movement) => {
                                let action_bytes = vec![
                                    0,
                                    movement.card().denote(),
                                    movement.direction().denote(),
                                ];
                                [action_bytes, value.to_le_bytes().to_vec()].concat()
                            }
                            Action::Attack(attack) => {
                                let action_bytes =
                                    vec![1, attack.card().denote(), attack.quantity()];
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

            let state = MyState::new(
                PlayerID::from_u8(my_id_bytes[0]).unwrap(),
                hands_bytes
                    .iter()
                    .filter(|&&n| n != 0)
                    .copied()
                    .map(|n| CardID::from_u8(n).expect(DESERIALIZE_ERROR_MESSAGE))
                    .collect::<Vec<CardID>>(),
                RestCards::from_slice(cards_bytes),
                u32::from_le_bytes(p0_score_bytes.try_into().unwrap()),
                u32::from_le_bytes(p1_score_bytes.try_into().unwrap()),
                p0_position_bytes[0],
                p1_position_bytes[0],
                match game_end_bytes[0] {
                    0 => false,
                    1 => true,
                    _ => unreachable!("デシリアライズ失敗"),
                },
            );

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
                            0 => Direction::Forward,
                            1 => Direction::Back,
                            _ => unreachable!(),
                        };
                        Action::Move(Movement::new(
                            CardID::from_u8(card_bytes[0]).expect(DESERIALIZE_ERROR_MESSAGE),
                            direction,
                        ))
                    }
                    1 => Action::Attack(Attack::new(
                        CardID::from_u8(card_bytes[0]).expect(DESERIALIZE_ERROR_MESSAGE),
                        property_bytes[0],
                    )),
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

fn q_train(loop_count: usize, id: u8) -> io::Result<()> {
    // ファイル読み込み
    let path = format!("learned{}", id);
    let mut learned_values = if let Ok(mut file) = OpenOptions::new().read(true).open(path.as_str())
    {
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        LearnedValues::deserialize(&data).get()
    } else {
        HashMap::new()
    };

    for _ in 0..loop_count {
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
        let (board_info_init, hand_info) = {
            let (mut board_info_init, mut hand_info_init) = (None, None);
            loop {
                match Messages::parse(&read_stream(&mut bufreader)?) {
                    Ok(Messages::BoardInfo(board_info)) => {
                        board_info_init = Some(board_info);
                    }
                    Ok(Messages::HandInfo(hand_info)) => {
                        hand_info_init = Some(hand_info);
                    }
                    Ok(_) | Err(_) => {}
                }
                // ここどうにかなりませんか?
                if let (Some(board_info_init), Some(hand_info_init)) =
                    (&board_info_init, &hand_info_init)
                {
                    break (board_info_init.clone(), hand_info_init.clone());
                }
            }
        };
        let mut hand_vec = hand_info.to_vec();
        // AI用エージェント作成
        let mut agent = MyAgent::new(
            id,
            hand_vec,
            board_info_init.p0_position(),
            board_info_init.p1_position(),
            bufreader,
            bufwriter,
        );

        //トレーニング開始
        trainer.train(
            &mut agent,
            &QLearning::new(0.2, 0.7, 0.0),
            &mut SinkStates {},
            &RandomExploration,
        );
        learned_values = trainer.export_learned_values();
    }
    let bytes = LearnedValues::from_map(learned_values).serialize();
    let filename = format!("learned{}", id);
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(filename)?;
    file.write_all(&bytes)?;
    Ok(())
}

fn q_eval(id: u8) -> io::Result<()> {
    // ファイル読み込み
    let path = format!("learned{}", id);
    let learned_values = if let Ok(mut file) = OpenOptions::new().read(true).open(path) {
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        LearnedValues::deserialize(&data).get()
    } else {
        HashMap::new()
    };

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
    let (board_info_init, hand_info) = {
        let (mut board_info_init, mut hand_info_init) = (None, None);
        loop {
            match Messages::parse(&read_stream(&mut bufreader)?) {
                Ok(Messages::BoardInfo(board_info)) => {
                    board_info_init = Some(board_info);
                }
                Ok(Messages::HandInfo(hand_info)) => {
                    hand_info_init = Some(hand_info);
                }
                Ok(_) | Err(_) => {}
            }
            if let (Some(board_info_init), Some(hand_info_init)) =
                (&board_info_init, &hand_info_init)
            {
                break (board_info_init.clone(), hand_info_init.clone());
            }
        }
    };

    let mut hand_vec = hand_info.to_vec();
    // AI用エージェント作成
    let mut agent = MyAgent::new(
        id,
        hand_vec,
        board_info_init.p0_position(),
        board_info_init.p1_position(),
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

    Ok(())
}

#[derive(Debug, Clone, ValueEnum)]
enum Mode {
    Train,
    Eval,
}

#[derive(Parser, Debug)]
struct Arguments {
    #[arg(long, short)]
    mode: Mode,
    #[arg(long, short, default_value_t = 0)]
    id: u8,
    #[arg(long, short, default_value_t = 1)]
    loop_count: usize,
}
fn main() -> io::Result<()> {
    let args = Arguments::parse();
    match args.mode {
        Mode::Train => q_train(args.loop_count, args.id),
        Mode::Eval => q_eval(args.id),
    }
}
