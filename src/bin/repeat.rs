//! 繰り返し学習させるアプリ

use std::{
    fmt::{Display, Formatter, Result},
    process::{Child, Command},
};

use clap::{Parser, ValueEnum};

const FINAL_LOOP_COUNT: usize = 20;
const LOOP_COUNT: usize = 20;
const MAX_ROUND: u32 = 100;

#[derive(ValueEnum, Clone, Debug, Copy)]
enum Client {
    Dqn,
    Random,
    Algorithm,
    Aggressive,
}

impl Client {
    fn execute(&self) -> Child {
        match self {
            Self::Dqn => Command::new(".\\dqn.exe")
                .arg("-m")
                .arg("train")
                .spawn()
                .expect("dqn.exe起動失敗"),
            Self::Random => Command::new(".\\random.exe")
                .spawn()
                .expect("random.exe起動失敗"),
            Self::Algorithm => Command::new(".\\using_algorithm.exe")
                .spawn()
                .expect("using_algorithm.exe起動失敗"),
            Self::Aggressive => Command::new(".\\aggressive.exe")
                .spawn()
                .expect("aggressive.exe起動失敗"),
        }
    }
}

impl Display for Client {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let s = match self {
            Self::Dqn => "dqn",
            Self::Random => "random",
            Self::Algorithm => "algorithm",
            Self::Aggressive => "aggressive",
        };
        s.fmt(f)
    }
}

#[derive(ValueEnum, Clone, Debug)]
enum LearningMode {
    QLearning,
    Dqn,
}

impl Display for LearningMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let s = match self {
            LearningMode::QLearning => "q-learning",
            LearningMode::Dqn => "dqn",
        };
        s.fmt(f)
    }
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(long,short,default_value_t = Client::Random)]
    player0: Client,
    #[arg(long,short,default_value_t = Client::Random)]
    player1: Client,
    #[arg(long, short = 'c', default_value_t = LOOP_COUNT)]
    loop_count: usize,
    #[arg(long, short, default_value_t = MAX_ROUND)]
    max_round: u32,
}

fn client_loop(client0: Client, client1: Client, loop_count: usize, max_round: u32) {
    for _ in 0..loop_count {
        let mut server = Command::new(".\\engarde_server.exe")
            .arg(max_round.to_string())
            .spawn()
            .expect("engarde_server.exe起動失敗");
        let mut client0 = client0.execute();
        let mut client1 = client1.execute();
        server.wait().expect("engarde_serverクラッシュ");
        client0.wait().expect("p0クラッシュ");
        client1.wait().expect("p1クラッシュ");
    }
}

fn main() {
    let args = Args::parse();
    client_loop(args.player0, args.player1, args.loop_count, args.max_round);
}
