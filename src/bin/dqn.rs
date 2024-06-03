//! DQNのAIクライアント

use std::{
    fs::create_dir_all,
    io::{self, BufReader, BufWriter},
    net::{SocketAddr, TcpStream},
};

use clap::{Parser, ValueEnum};
use dfdx::{
    nn::modules::{Linear, ReLU},
    shapes::Const,
    tensor::{Cpu, NoneTape, Tensor, ZerosTensor},
};
use rurel::{
    dqn::DQNAgentTrainer,
    mdp::{Agent, State},
    strategy::{
        explore::{ExplorationStrategy, RandomExploration},
        terminate::SinkStates,
    },
};

use engarde_client::{
    get_id,
    protocol::{BoardInfo, Messages, PlayerName},
    read_stream, send_info,
    states::{Action, MyAgent, MyState},
};

struct BestExplorationDqn(DQNAgentTrainer<MyState, 16, 35, 32>);

impl BestExplorationDqn {
    fn new(trainer: DQNAgentTrainer<MyState, 16, 35, 32>) -> Self {
        BestExplorationDqn(trainer)
    }
}

impl ExplorationStrategy<MyState> for BestExplorationDqn {
    fn pick_action(&self, agent: &mut dyn Agent<MyState>) -> <MyState as State>::A {
        let current_state = agent.current_state();

        // 行動していいアクション"のインデックス"のリストを取得
        let available_action_indicies = current_state
            .actions()
            .into_iter()
            .map(|action| action.as_index())
            .collect::<Vec<usize>>();

        // 評価値のリストを取得
        let expected_values = self.0.expected_value(current_state);

        // 有効なアクションと評価値のリストを取得
        let available_actions = expected_values
            .into_iter()
            .enumerate()
            .filter(|(i, _)| available_action_indicies.contains(i))
            .collect::<Vec<(usize, f32)>>();

        // 評価値が最大のインデックスを取得
        let action_index = available_actions
            .into_iter()
            .max_by(|(_, value), (_, other_value)| value.total_cmp(other_value))
            .unwrap()
            .0;

        // そのインデックスでアクションに変換
        let action = Action::from_index(action_index);

        // 行動
        agent.take_action(&action);
        action
    }
}

fn files_name(id: u8) -> (String, String, String, String, String, String) {
    (
        format!("learned_dqn/{id}/weight0.npy"),
        format!("learned_dqn/{id}/bias0.npy"),
        format!("learned_dqn/{id}/weight1.npy"),
        format!("learned_dqn/{id}/bias1.npy"),
        format!("learned_dqn/{id}/weight2.npy"),
        format!("learned_dqn/{id}/bias2.npy"),
    )
}

fn dqn_train() -> io::Result<()> {
    let mut trainer = DQNAgentTrainer::<MyState, 16, 35, 32>::new(1.0, 0.2);
    let addr = SocketAddr::from(([127, 0, 0, 1], 12052));
    let stream = loop {
        if let Ok(stream) = TcpStream::connect(addr) {
            break stream;
        }
    };
    let (mut bufreader, mut bufwriter) =
        (BufReader::new(stream.try_clone()?), BufWriter::new(stream));
    let id = get_id(&mut bufreader)?;
    let player_name = PlayerName::new("dqnai".to_string());
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
        board_info_init.p0_position(),
        board_info_init.p1_position(),
        bufreader,
        bufwriter,
    );
    let past_exp = {
        let cpu = Cpu::default();
        let mut weight0: Tensor<(Const<32>, Const<16>), f32, Cpu> = cpu.zeros();
        let mut bias0: Tensor<(Const<32>,), f32, Cpu> = cpu.zeros();
        let mut weight1: Tensor<(Const<32>, Const<32>), f32, Cpu, NoneTape> = cpu.zeros();
        let mut bias1: Tensor<(Const<32>,), f32, Cpu> = cpu.zeros();
        let mut weight2: Tensor<(Const<35>, Const<32>), f32, Cpu> = cpu.zeros();
        let mut bias2: Tensor<(Const<35>,), f32, Cpu> = cpu.zeros();
        let files = files_name(id.denote());
        (|| {
            weight0.load_from_npy(files.0).ok()?;
            bias0.load_from_npy(files.1).ok()?;
            weight1.load_from_npy(files.2).ok()?;
            bias1.load_from_npy(files.3).ok()?;
            weight2.load_from_npy(files.4).ok()?;
            bias2.load_from_npy(files.5).ok()?;
            Some(())
        })()
        .map_or(trainer.export_learned_values(), |()| {
            (
                (
                    Linear {
                        weight: weight0,
                        bias: bias0,
                    },
                    ReLU,
                ),
                (
                    Linear {
                        weight: weight1,
                        bias: bias1,
                    },
                    ReLU,
                ),
                Linear {
                    weight: weight2,
                    bias: bias2,
                },
            )
        })
    };
    trainer.import_model(past_exp);
    trainer.train(&mut agent, &mut SinkStates {}, &RandomExploration);
    {
        let learned_values = trainer.export_learned_values();
        let linear0 = learned_values.0 .0;
        let weight0 = linear0.weight;
        let bias0 = linear0.bias;
        let linear1 = learned_values.1 .0;
        let weight1 = linear1.weight;
        let bias1 = linear1.bias;
        let linear2 = learned_values.2;
        let weight2 = linear2.weight;
        let bias2 = linear2.bias;
        let files = files_name(id.denote());
        let _ = create_dir_all(format!("learned_dqn/{}", id.denote()));
        weight0.save_to_npy(files.0)?;
        bias0.save_to_npy(files.1)?;
        weight1.save_to_npy(files.2)?;
        bias1.save_to_npy(files.3)?;
        weight2.save_to_npy(files.4)?;
        bias2.save_to_npy(files.5)?;
    }
    Ok(())
}

fn dqn_eval() -> io::Result<()> {
    let mut trainer = DQNAgentTrainer::<MyState, 16, 35, 32>::new(0.99, 0.2);

    let addr = SocketAddr::from(([127, 0, 0, 1], 12052));
    let stream = loop {
        if let Ok(stream) = TcpStream::connect(addr) {
            break stream;
        }
    };
    let (mut bufreader, mut bufwriter) =
        (BufReader::new(stream.try_clone()?), BufWriter::new(stream));
    let id = get_id(&mut bufreader)?;
    let player_name = PlayerName::new("dqnai".to_string());
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
        board_info_init.p0_position(),
        board_info_init.p1_position(),
        bufreader,
        bufwriter,
    );
    let past_exp = {
        let cpu = Cpu::default();
        let mut weight0: Tensor<(Const<32>, Const<16>), f32, Cpu> = cpu.zeros();
        let mut bias0: Tensor<(Const<32>,), f32, Cpu> = cpu.zeros();
        let mut weight1: Tensor<(Const<32>, Const<32>), f32, Cpu, NoneTape> = cpu.zeros();
        let mut bias1: Tensor<(Const<32>,), f32, Cpu> = cpu.zeros();
        let mut weight2: Tensor<(Const<35>, Const<32>), f32, Cpu> = cpu.zeros();
        let mut bias2: Tensor<(Const<35>,), f32, Cpu> = cpu.zeros();
        let files = files_name(id.denote());
        (|| {
            weight0.load_from_npy(files.0).ok()?;
            bias0.load_from_npy(files.1).ok()?;
            weight1.load_from_npy(files.2).ok()?;
            bias1.load_from_npy(files.3).ok()?;
            weight2.load_from_npy(files.4).ok()?;
            bias2.load_from_npy(files.5).ok()?;
            Some(())
        })()
        .map_or(trainer.export_learned_values(), |()| {
            (
                (
                    Linear {
                        weight: weight0,
                        bias: bias0,
                    },
                    ReLU,
                ),
                (
                    Linear {
                        weight: weight1,
                        bias: bias1,
                    },
                    ReLU,
                ),
                Linear {
                    weight: weight2,
                    bias: bias2,
                },
            )
        })
    };
    trainer.import_model(past_exp.clone());
    let mut trainer2 = DQNAgentTrainer::new(0.99, 0.2);
    trainer2.import_model(past_exp);
    trainer.train(
        &mut agent,
        &mut SinkStates {},
        &BestExplorationDqn::new(trainer2),
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
}

fn main() -> io::Result<()> {
    let args = Arguments::parse();
    match args.mode {
        Mode::Train => dqn_train(),
        Mode::Eval => dqn_eval(),
    }
}
