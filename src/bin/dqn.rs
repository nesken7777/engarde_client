//! DQNのAIクライアント

use std::{
    cmp::Ordering,
    fs::create_dir_all,
    io::{self, BufReader, BufWriter},
    net::{SocketAddrV4, TcpStream},
    path::PathBuf,
    str::FromStr,
};

use apply::Also;
use clap::{Parser, ValueEnum};
#[allow(clippy::wildcard_imports)]
use dfdx::nn::modules::*;
use dfdx::{
    shapes::Const,
    tensor::{Cpu, NoneTape, Tensor, ZerosTensor},
};
use rand::{thread_rng, Rng};
use rurel::{
    dqn::DQNAgentTrainer,
    mdp::{Agent, State},
    strategy::{
        explore::{ExplorationStrategy, RandomExploration},
        terminate::{SinkStates, TerminationStrategy},
    },
};

use engarde_client::{
    get_id,
    protocol::{BoardInfo, Messages, PlayerName},
    read_stream, send_info,
    states::{MyAgent, MyState},
    Action, CardID, Direction,
};

const STATE_SIZE: usize = 13;
const INNER_DISCREATE: usize = 64;
const ACTION_SIZE_DISCREATE: usize = 35;
const INNER_CONTINUOUS: usize = 64;
const ACTION_SIZE_CONTINUOUS: usize = 3;
const INNER_KAZU: usize = 1;
const NORM_KAZU: usize = 2;

type DQNAgentTrainerDiscreate =
    DQNAgentTrainer<MyState, STATE_SIZE, ACTION_SIZE_DISCREATE, INNER_DISCREATE>;
type DQNAgentTrainerContinuous =
    DQNAgentTrainer<MyState, STATE_SIZE, ACTION_SIZE_CONTINUOUS, INNER_CONTINUOUS>;
type WeightInTensor = Tensor<(Const<INNER_CONTINUOUS>, Const<STATE_SIZE>), f32, Cpu>;
type BiasInTensor = Tensor<(Const<INNER_CONTINUOUS>,), f32, Cpu>;
type WeightInnerTensor =
    Tensor<(Const<INNER_CONTINUOUS>, Const<INNER_CONTINUOUS>), f32, Cpu, NoneTape>;
type BiasInnerTensor = Tensor<(Const<INNER_CONTINUOUS>,), f32, Cpu>;
type WeightOutTensorContinuous =
    Tensor<(Const<ACTION_SIZE_CONTINUOUS>, Const<INNER_CONTINUOUS>), f32, Cpu>;
type WeightOutTensorDiscreate =
    Tensor<(Const<ACTION_SIZE_DISCREATE>, Const<INNER_DISCREATE>), f32, Cpu>;
type BiasOutTensorContinuous = Tensor<(Const<ACTION_SIZE_CONTINUOUS>,), f32, Cpu>;
type BiasOutTensorDiscreate = Tensor<(Const<ACTION_SIZE_DISCREATE>,), f32, Cpu>;

const DISCOUNT_RATE: f32 = 0.99;
const LEARNING_RATE: f32 = 0.00025;

/// ベストに近いアクションを返す
#[allow(dead_code, clippy::too_many_lines)]
fn neary_best_action(state: &MyState, trainer: &DQNAgentTrainerContinuous) -> Option<Action> {
    let best = trainer.best_action(state);
    let actions = state.actions();
    if actions.contains(&best) {
        Some(best)
    } else {
        match best {
            Action::Move(movement) => {
                // 前進の場合、前進-攻撃-後退の順に並び替え
                // 後退の場合、後退-攻撃-前進の順に並び替え
                // (クソ長い)
                fn ordering(
                    key_direction: Direction,
                    key_card: CardID,
                    action1: Action,
                    action2: Action,
                ) -> Ordering {
                    match action1 {
                        Action::Move(movement1) => {
                            let card1 = movement1.card();
                            let direction1 = movement1.direction();
                            match direction1 {
                                Direction::Forward => match action2 {
                                    Action::Move(movement2) => {
                                        let card2 = movement2.card();
                                        let direction2 = movement2.direction();
                                        match direction2 {
                                            Direction::Forward => {
                                                let key_card_i32 = i32::from(key_card.denote());
                                                let card1_i32 = i32::from(card1.denote());
                                                let card2_i32 = i32::from(card2.denote());
                                                (card1_i32 - key_card_i32)
                                                    .abs()
                                                    .cmp(&(card2_i32 - key_card_i32).abs())
                                            }
                                            Direction::Back => match key_direction {
                                                Direction::Forward => Ordering::Less,
                                                Direction::Back => Ordering::Greater,
                                            },
                                        }
                                    }
                                    Action::Attack(_) => match key_direction {
                                        Direction::Forward => Ordering::Less,
                                        Direction::Back => Ordering::Greater,
                                    },
                                },
                                Direction::Back => match action2 {
                                    Action::Move(movement2) => {
                                        let card2 = movement2.card();
                                        let direction2 = movement2.direction();
                                        match direction2 {
                                            Direction::Forward => match key_direction {
                                                Direction::Forward => Ordering::Greater,
                                                Direction::Back => Ordering::Less,
                                            },
                                            Direction::Back => {
                                                let key_card_i32 = i32::from(key_card.denote());
                                                let card1_i32 = i32::from(card1.denote());
                                                let card2_i32 = i32::from(card2.denote());
                                                (card1_i32 - key_card_i32)
                                                    .abs()
                                                    .cmp(&(card2_i32 - key_card_i32).abs())
                                            }
                                        }
                                    }
                                    Action::Attack(_) => match key_direction {
                                        Direction::Forward => Ordering::Greater,
                                        Direction::Back => Ordering::Less,
                                    },
                                },
                            }
                        }
                        Action::Attack(_) => match action2 {
                            Action::Move(movement2) => {
                                let direction2 = movement2.direction();
                                match direction2 {
                                    Direction::Forward => match key_direction {
                                        Direction::Forward => Ordering::Greater,
                                        Direction::Back => Ordering::Less,
                                    },
                                    Direction::Back => match key_direction {
                                        Direction::Forward => Ordering::Less,
                                        Direction::Back => Ordering::Greater,
                                    },
                                }
                            }
                            Action::Attack(_) => Ordering::Equal,
                        },
                    }
                }
                let card = movement.card();
                let direction = movement.direction();
                match direction {
                    Direction::Forward => {
                        let mut actions = actions;
                        actions.sort_by(|&action1, &action2| {
                            ordering(Direction::Forward, card, action1, action2)
                        });
                        actions.first().copied()
                    }
                    Direction::Back => {
                        let mut actions = actions;
                        actions.sort_by(|&action1, &action2| {
                            ordering(Direction::Back, card, action1, action2)
                        });
                        actions.first().copied()
                    }
                }
            }
            Action::Attack(_) => {
                let mut actions = actions;
                actions.sort_by_key(|action| match action {
                    Action::Move(movement) => movement.card(),
                    Action::Attack(attack) => attack.card(),
                });
                actions.first().copied()
            }
        }
    }
}

struct EpsilonGreedyDiscrete {
    past_exp: DQNAgentTrainerDiscreate,
    epsilon: u64,
}

impl EpsilonGreedyDiscrete {
    fn new(trainer: DQNAgentTrainerDiscreate, start_epsilon: u64) -> Self {
        EpsilonGreedyDiscrete {
            past_exp: trainer,
            epsilon: start_epsilon,
        }
    }
}

impl ExplorationStrategy<MyState> for EpsilonGreedyDiscrete {
    fn pick_action(&mut self, agent: &mut dyn Agent<MyState>) -> <MyState as State>::A {
        let mut rng = thread_rng();
        let random = rng.gen::<u64>();
        let expected_values = self.past_exp.expected_value(agent.current_state());
        println!("{expected_values:.2?}");
        #[cfg(feature = "print_priority")]
        print_actions_priority(expected_values);
        assert_ne!(expected_values[0], 320000f32, "NaN値になってます！");
        if random < self.epsilon {
            agent.pick_random_action()
        } else {
            let current_state = agent.current_state();

            // 行動していいアクション"のインデックス"のリストを取得
            let available_action_indicies = current_state
                .actions()
                .into_iter()
                .map(|action| action.to_index())
                .collect::<Vec<usize>>();
            // 評価値のリストを取得
            // let expected_values = self.past_exp.expected_value(current_state);
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
                .expect("必ず最大値がある")
                .0;

            // そのインデックスでアクションに変換
            let action = Action::from_index(action_index);

            // 行動
            agent.take_action(&action);
            action
        }
    }
}

struct BestExplorationDqnDiscrete(DQNAgentTrainerDiscreate);

impl BestExplorationDqnDiscrete {
    fn new(trainer: DQNAgentTrainerDiscreate) -> Self {
        BestExplorationDqnDiscrete(trainer)
    }
}

impl ExplorationStrategy<MyState> for BestExplorationDqnDiscrete {
    fn pick_action(&mut self, agent: &mut dyn Agent<MyState>) -> <MyState as State>::A {
        let current_state = agent.current_state();

        // 行動していいアクション"のインデックス"のリストを取得
        let available_action_indicies = current_state
            .actions()
            .into_iter()
            .map(|action| action.to_index())
            .collect::<Vec<usize>>();

        // 評価値のリストを取得
        let expected_values = self.0.expected_value(current_state);
        #[cfg(feature = "print_priority")]
        print_actions_priority(expected_values);
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
            .expect("必ず最大値がある")
            .0;

        // そのインデックスでアクションに変換
        let action = Action::from_index(action_index);

        // 行動
        agent.take_action(&action);
        action
    }
}

#[cfg(feature = "print_priority")]
fn print_actions_priority(expected_values: [f32; 35]) {
    let sorted_actions = expected_values
        .into_iter()
        .enumerate()
        .collect::<Vec<_>>()
        .also(|v| v.sort_unstable_by(|(_, x), (_, y)| y.total_cmp(x)))
        .into_iter()
        .map(|(i, _)| Action::from_index(i).to_string())
        .collect::<Vec<_>>();
    println!("{sorted_actions:?}");
}

struct EpsilonGreedyContinuous {
    past_exp: DQNAgentTrainerContinuous,
    epsilon: u64,
}

impl EpsilonGreedyContinuous {
    fn new(trainer: DQNAgentTrainerContinuous, start_epsilon: u64) -> Self {
        EpsilonGreedyContinuous {
            past_exp: trainer,
            epsilon: start_epsilon,
        }
    }
}

impl ExplorationStrategy<MyState> for EpsilonGreedyContinuous {
    fn pick_action(&mut self, agent: &mut dyn Agent<MyState>) -> <MyState as State>::A {
        let random = thread_rng().gen::<u64>();
        if random < self.epsilon {
            agent.pick_random_action()
        } else {
            match neary_best_action(agent.current_state(), &self.past_exp) {
                Some(action) => {
                    agent.take_action(&action);
                    action
                }
                None => agent.pick_random_action(),
            }
        }
    }
}

struct BestExplorationDqnContinuous(DQNAgentTrainerContinuous);

impl BestExplorationDqnContinuous {
    fn new(trainer: DQNAgentTrainerContinuous) -> Self {
        BestExplorationDqnContinuous(trainer)
    }
}

impl ExplorationStrategy<MyState> for BestExplorationDqnContinuous {
    fn pick_action(&mut self, agent: &mut dyn Agent<MyState>) -> <MyState as State>::A {
        match neary_best_action(agent.current_state(), &self.0) {
            Some(action) => {
                agent.take_action(&action);
                action
            }
            None => agent.pick_random_action(),
        }
    }
}

struct RandomExploration2(DQNAgentTrainerDiscreate);

impl ExplorationStrategy<MyState> for RandomExploration2 {
    fn pick_action(&mut self, agent: &mut dyn Agent<MyState>) -> <MyState as State>::A {
        let expected_values = self.0.expected_value(agent.current_state());
        #[cfg(feature = "print_priority")]
        print_actions_priority(expected_values);
        assert_ne!(expected_values[0], 320_000_f32, "NaN値になってます！");
        agent.pick_random_action()
    }
}

struct NNFileNames {
    weight_in: PathBuf,
    bias_in: PathBuf,
    inner: Vec<(PathBuf, PathBuf)>,
    norm: Vec<(PathBuf, PathBuf)>,
    weight_out: PathBuf,
    bias_out: PathBuf,
}

fn files_name(id: u8) -> NNFileNames {
    let folder = PathBuf::from_str("learned_dqn")
        .expect("エラーは起きない")
        .also(|path| path.push(id.to_string()));
    NNFileNames {
        weight_in: folder.clone().also(|folder| folder.push("weight_in.npy")),
        bias_in: folder.clone().also(|folder| folder.push("bias_in.npy")),
        inner: (1..=INNER_KAZU)
            .map(|x| {
                (
                    folder
                        .clone()
                        .also(|folder| folder.push(format!("weight{x}.npy"))),
                    folder
                        .clone()
                        .also(|folder| folder.push(format!("bias{x}.npy"))),
                )
            })
            .collect(),
        norm: (1..=NORM_KAZU)
            .map(|x| {
                (
                    folder
                        .clone()
                        .also(|folder| folder.push(format!("gamma{x}.npy"))),
                    folder
                        .clone()
                        .also(|folder| folder.push(format!("beta{x}.npy"))),
                )
            })
            .collect(),
        weight_out: folder.clone().also(|folder| folder.push("weight_out.npy")),
        bias_out: folder.clone().also(|folder| folder.push("bias_out.npy")),
    }
}

#[allow(clippy::too_many_lines)]
fn dqn_train(ip: SocketAddrV4) -> io::Result<()> {
    // let addr = SocketAddr::from(([127, 0, 0, 1], 12052));
    let addr = ip;
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
    let hand_vec = hand_info.to_vec().also(|hand_vec| hand_vec.sort());
    // AI用エージェント作成
    let mut agent = MyAgent::new(
        id,
        hand_vec,
        board_info_init.p0_position(),
        board_info_init.p1_position(),
        bufreader,
        bufwriter,
    );

    // let mut trainer = DQNAgentTrainerDiscreate::new(DISCOUNT_RATE, LEARNING_RATE);
    let mut trainer = DQNAgentTrainerDiscreate::new(DISCOUNT_RATE, LEARNING_RATE);
    let past_exp = {
        let cpu = Cpu::default();
        let mut weight_in: WeightInTensor = cpu.zeros();
        let mut bias_in: BiasInTensor = cpu.zeros();
        let mut inners: Vec<(WeightInnerTensor, BiasInnerTensor)> =
            vec![(cpu.zeros(), cpu.zeros()); INNER_KAZU];
        let mut normalizes: Vec<(_, _)> = vec![(cpu.zeros(), cpu.zeros()); NORM_KAZU];
        let mut weight_out: WeightOutTensorDiscreate = cpu.zeros();
        let mut bias_out: BiasOutTensorDiscreate = cpu.zeros();
        let files = files_name(id.denote());
        (|| {
            weight_in.load_from_npy(files.weight_in).ok()?;
            bias_in.load_from_npy(files.bias_in).ok()?;
            inners[0].0.load_from_npy(files.inner[0].0.clone()).ok()?;
            inners[0].1.load_from_npy(files.inner[0].1.clone()).ok()?;
            // inners[1].0.load_from_npy(files.inner[1].0.clone()).ok()?;
            // inners[1].1.load_from_npy(files.inner[1].1.clone()).ok()?;
            normalizes[0]
                .0
                .load_from_npy(files.norm[0].0.clone())
                .ok()?;
            normalizes[0]
                .1
                .load_from_npy(files.norm[0].1.clone())
                .ok()?;
            normalizes[1]
                .0
                .load_from_npy(files.norm[1].0.clone())
                .ok()?;
            normalizes[1]
                .1
                .load_from_npy(files.norm[1].1.clone())
                .ok()?;
            weight_out.load_from_npy(files.weight_out).ok()?;
            bias_out.load_from_npy(files.bias_out).ok()?;
            Some(())
        })()
        .map_or(trainer.export_learned_values(), |()| {
            (
                (
                    Linear {
                        weight: weight_in,
                        bias: bias_in,
                    },
                    LayerNorm1D {
                        gamma: normalizes[0].0.clone(),
                        beta: normalizes[0].1.clone(),
                        epsilon: 0.001,
                    },
                    LeakyReLU(0.01),
                ),
                (
                    Linear {
                        weight: inners[0].0.clone(),
                        bias: inners[0].1.clone(),
                    },
                    LayerNorm1D {
                        gamma: normalizes[1].0.clone(),
                        beta: normalizes[1].1.clone(),
                        epsilon: 0.001,
                    },
                    LeakyReLU(0.01),
                ),
                // (
                //     Linear {
                //         weight: inners[1].0.clone(),
                //         bias: inners[1].1.clone(),
                //     },
                //     ReLU,
                // ),
                Linear {
                    weight: weight_out,
                    bias: bias_out,
                },
            )
        })
    };
    trainer.import_model(past_exp.clone());
    let mut trainer2 = DQNAgentTrainer::new(DISCOUNT_RATE, LEARNING_RATE);
    trainer2.import_model(past_exp);
    // let epsilon = fs::read_to_string(format!("learned_dqn/{}/epsilon.txt", id.denote()))
    //     .map(|eps_str| eps_str.parse::<u64>().expect("εが適切なu64値でない"))
    //     .unwrap_or(u64::MAX);
    // let epsilon = (epsilon - (u64::MAX / 5000)).max(u64::MAX / 10);
    let mut epsilon_greedy_exploration = EpsilonGreedyDiscrete::new(trainer2, u64::MAX / 5);
    trainer.train(
        &mut agent,
        &mut SinkStates {},
        &mut epsilon_greedy_exploration,
    );
    {
        let learned_values = trainer.export_learned_values();
        let linear_in = learned_values.0 .0;
        let weight_in = linear_in.weight;
        let bias_in = linear_in.bias;
        let linear1 = learned_values.1 .0;
        let weight1 = linear1.weight;
        let bias1 = linear1.bias;
        // let linear2 = learned_values.2 .0;
        // let weight2 = linear2.weight;
        // let bias2 = linear2.bias;
        let norm1 = learned_values.0 .1;
        let gamma1 = norm1.gamma;
        let beta1 = norm1.beta;
        let norm2 = learned_values.1 .1;
        let gamma2 = norm2.gamma;
        let beta2 = norm2.beta;
        let linear_out = learned_values.2;
        let weight_out = linear_out.weight;
        let bias_out = linear_out.bias;
        let files = files_name(id.denote());
        let _ = create_dir_all(format!("learned_dqn/{}", id.denote()));
        weight_in.save_to_npy(files.weight_in)?;
        bias_in.save_to_npy(files.bias_in)?;
        weight1.save_to_npy(files.inner[0].0.clone())?;
        bias1.save_to_npy(files.inner[0].1.clone())?;
        // weight2.save_to_npy(files.inner[1].0.clone())?;
        // bias2.save_to_npy(files.inner[1].1.clone())?;
        gamma1.save_to_npy(files.norm[0].0.clone())?;
        beta1.save_to_npy(files.norm[0].1.clone())?;
        gamma2.save_to_npy(files.norm[1].0.clone())?;
        beta2.save_to_npy(files.norm[1].1.clone())?;
        weight_out.save_to_npy(files.weight_out)?;
        bias_out.save_to_npy(files.bias_out)?;
        // fs::write(
        //     format!("learned_dqn/{}/epsilon.txt", id.denote()),
        //     epsilon_greedy_exploration.epsilon.to_string(),
        // )?;
    }
    Ok(())
}

fn evaluation_continuous(
    agent: &mut MyAgent,
    termination_strategy: &mut dyn TerminationStrategy<MyState>,
    best_exploration_strategy: &mut BestExplorationDqnContinuous,
) {
    loop {
        best_exploration_strategy.pick_action(agent);
        if termination_strategy.should_stop(agent.current_state()) {
            break;
        }
    }
}

fn evaluation_discrete(
    agent: &mut MyAgent,
    termination_strategy: &mut dyn TerminationStrategy<MyState>,
    best_exploration_strategy: &mut BestExplorationDqnDiscrete,
) {
    loop {
        best_exploration_strategy.pick_action(agent);
        if termination_strategy.should_stop(agent.current_state()) {
            break;
        }
    }
}

#[allow(clippy::too_many_lines)]
fn dqn_eval(ip: SocketAddrV4) -> io::Result<()> {
    // let addr = SocketAddr::from(([127, 0, 0, 1], 12052));
    let addr = ip;
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
    let hand_vec = hand_info.to_vec().also(|hand_vec| hand_vec.sort());
    // AI用エージェント作成
    let mut agent = MyAgent::new(
        id,
        hand_vec,
        board_info_init.p0_position(),
        board_info_init.p1_position(),
        bufreader,
        bufwriter,
    );

    let mut trainer = DQNAgentTrainerDiscreate::new(DISCOUNT_RATE, LEARNING_RATE);
    let past_exp = {
        let cpu = Cpu::default();
        let mut weight_in: WeightInTensor = cpu.zeros();
        let mut bias_in: BiasInTensor = cpu.zeros();
        let mut inners: Vec<(WeightInnerTensor, BiasInnerTensor)> =
            vec![(cpu.zeros(), cpu.zeros()); INNER_KAZU];
        let mut normalizes: Vec<(_, _)> = vec![(cpu.zeros(), cpu.zeros()); NORM_KAZU];
        let mut weight_out: WeightOutTensorDiscreate = cpu.zeros();
        let mut bias_out: BiasOutTensorDiscreate = cpu.zeros();
        let files = files_name(id.denote());
        (|| {
            weight_in.load_from_npy(files.weight_in).ok()?;
            bias_in.load_from_npy(files.bias_in).ok()?;
            inners[0].0.load_from_npy(files.inner[0].0.clone()).ok()?;
            inners[0].1.load_from_npy(files.inner[0].1.clone()).ok()?;
            // inners[1].0.load_from_npy(files.inner[1].0.clone()).ok()?;
            // inners[1].1.load_from_npy(files.inner[1].1.clone()).ok()?;
            normalizes[0]
                .0
                .load_from_npy(files.norm[0].0.clone())
                .ok()?;
            normalizes[0]
                .1
                .load_from_npy(files.norm[0].1.clone())
                .ok()?;
            normalizes[1]
                .0
                .load_from_npy(files.norm[1].0.clone())
                .ok()?;
            normalizes[1]
                .1
                .load_from_npy(files.norm[1].1.clone())
                .ok()?;
            weight_out.load_from_npy(files.weight_out).ok()?;
            bias_out.load_from_npy(files.bias_out).ok()?;
            Some(())
        })()
        .map_or(trainer.export_learned_values(), |()| {
            (
                (
                    Linear {
                        weight: weight_in,
                        bias: bias_in,
                    },
                    LayerNorm1D {
                        gamma: normalizes[0].0.clone(),
                        beta: normalizes[0].1.clone(),
                        epsilon: 0.001,
                    },
                    LeakyReLU(0.01),
                ),
                (
                    Linear {
                        weight: inners[0].0.clone(),
                        bias: inners[0].1.clone(),
                    },
                    LayerNorm1D {
                        gamma: normalizes[1].0.clone(),
                        beta: normalizes[1].1.clone(),
                        epsilon: 0.001,
                    },
                    LeakyReLU(0.01),
                ),
                // (
                //     Linear {
                //         weight: inners[1].0.clone(),
                //         bias: inners[1].1.clone(),
                //     },
                //     ReLU,
                // ),
                Linear {
                    weight: weight_out,
                    bias: bias_out,
                },
            )
        })
    };
    trainer.import_model(past_exp.clone());
    evaluation_discrete(
        &mut agent,
        &mut SinkStates {},
        &mut BestExplorationDqnDiscrete::new(trainer),
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
    match args.mode {
        Mode::Train => dqn_train(ip),
        Mode::Eval => dqn_eval(ip),
    }
}
