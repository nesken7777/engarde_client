use std::{fmt::Display, process::Command};

use clap::{Parser, ValueEnum};
use engarde_client::print;

const FINAL_LOOP_COUNT: usize = 20;
const LOOP_COUNT: usize = 20;
const MAX_SCORE: u32 = 100;

#[derive(ValueEnum, Clone, Debug)]
enum LearningMode {
    QLearning,
    Dqn,
}

impl Display for LearningMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            LearningMode::QLearning => "q-learning",
            LearningMode::Dqn => "dqn",
        };
        s.fmt(f)
    }
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, short, default_value_t = LearningMode::Dqn)]
    learning_mode: LearningMode,
    #[arg(long, short, default_value_t = FINAL_LOOP_COUNT)]
    final_loop: usize,
    #[arg(long, short = 'c', default_value_t = LOOP_COUNT)]
    loop_count: usize,
    #[arg(long, short, default_value_t = MAX_SCORE)]
    max_score: u32,
}

fn q_learning_loop(final_loop: usize, loop_count: usize, max_score: u32) {
    for _ in 0..final_loop {
        let mut client0 = Command::new(".\\q-learning.exe")
            .arg("-m")
            .arg("train")
            .arg("-i")
            .arg(0.to_string())
            .arg("-l")
            .arg(loop_count.to_string())
            .spawn()
            .unwrap();
        let mut client1 = Command::new(".\\q-learning.exe")
            .arg("-m")
            .arg("train")
            .arg("-i")
            .arg(1.to_string())
            .arg("-l")
            .arg(loop_count.to_string())
            .spawn()
            .unwrap();
        for _ in 0..loop_count {
            let mut server = Command::new(".\\engarde_server.exe")
                .arg(max_score.to_string())
                .spawn()
                .unwrap();
            server.wait().unwrap();
        }
        client0.wait().unwrap();
        client1.wait().unwrap();
    }
}

fn dqn_loop(final_loop: usize, loop_count: usize, max_score: u32) {
    for i in 0..final_loop * loop_count {
        let mut server = Command::new(".\\engarde_server.exe")
            .arg(max_score.to_string())
            .spawn()
            .unwrap();
        let mut client0 = Command::new(".\\dqn.exe")
            .arg("-m")
            .arg("train")
            .spawn()
            .unwrap();
        let mut client1 = Command::new(".\\dqn.exe")
            .arg("-m")
            .arg("train")
            .spawn()
            .unwrap();

        server.wait().unwrap();
        client0.wait().unwrap();
        client1.wait().unwrap();
        print(format!("{i}").as_str()).unwrap();
    }
}

fn main() {
    let args = Args::parse();
    match args.learning_mode {
        LearningMode::QLearning => {
            q_learning_loop(args.final_loop, args.loop_count, args.max_score)
        }
        LearningMode::Dqn => dqn_loop(args.final_loop, args.loop_count, args.max_score),
    }
}
