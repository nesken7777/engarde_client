//! 繰り返し学習させるアプリ

use core::str;
use std::{
    fmt::{Display, Formatter, Result},
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
    str::FromStr,
    thread,
    time::Duration,
};

use clap::{Parser, ValueEnum};
use engarde_client::print;
use plotters::{
    chart::ChartBuilder,
    prelude::{BitMapBackend, IntoDrawingArea, PathElement},
    series::LineSeries,
    style::{Color, IntoFont, BLACK, BLUE, RED, WHITE},
};
use regex::Regex;
use tap::Tap;

const FINAL_LOOP_COUNT: usize = 20;
const LOOP_COUNT: usize = 20;
const MAX_ROUND: u32 = 100;

#[derive(ValueEnum, Clone, Debug, Copy)]
enum Client {
    Dqn,
    Random,
    RandomForward,
    Algorithm,
    Aggressive,
    ToCenter,
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
            Self::RandomForward => Command::new(".\\random_forward.exe")
                .spawn()
                .expect("random_forward.exe起動失敗"),
            Self::Algorithm => Command::new(".\\using_algorithm.exe")
                .spawn()
                .expect("using_algorithm.exe起動失敗"),
            Self::Aggressive => Command::new(".\\aggressive.exe")
                .spawn()
                .expect("aggressive.exe起動失敗"),
            Self::ToCenter => Command::new(".\\to_center.exe")
                .spawn()
                .expect("to_center.exe起動失敗"),
        }
    }
}

impl Display for Client {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let s = match self {
            Self::Dqn => "dqn",
            Self::Random => "random",
            Self::RandomForward => "random_forward",
            Self::Algorithm => "algorithm",
            Self::Aggressive => "aggressive",
            Self::ToCenter => "to_center",
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
    let mut scores = vec![];
    let resut_path = PathBuf::from_str("result").expect("");
    let result_text_path = resut_path.clone().tap_mut(|path| path.push("result.txt"));
    let result_image_path = resut_path.clone().tap_mut(|path| path.push("result.png"));
    {
        fs::create_dir_all(&resut_path).expect("ディレクトリ作成失敗");
        OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&result_text_path)
            .expect("");
    }
    for i in 0..loop_count {
        let server = Command::new(".\\engarde_server.exe")
            .arg(max_round.to_string())
            .stdout(Stdio::piped())
            .spawn()
            .expect("engarde_server.exe起動失敗");
        let mut client0 = client0.execute();
        thread::sleep(Duration::from_millis(50));
        let mut client1 = client1.execute();
        let server_stdout = server.wait_with_output().expect("engarde_serverクラッシュ");
        let server_string = str::from_utf8(&server_stdout.stdout).expect("読み取れない");
        let re = Regex::new(r"p0: (\d+)点, p1: (\d+)点").expect("正規表現がおかしい");
        let caps = re.captures(server_string).expect("キャプチャできなかった");
        let p0_score = caps[1].parse::<u32>().expect("整数値じゃない");
        let p1_score = caps[2].parse::<u32>().expect("整数値じゃない");
        let mut result_text = OpenOptions::new()
            .append(true)
            .truncate(false)
            .create(true)
            .open(&result_text_path)
            .expect("ファイル作成/読み込み失敗");
        result_text
            .write_all(format!("{i} {p0_score} {p1_score}\n").as_bytes())
            .expect("書き込み失敗");
        scores.push((p0_score, p1_score));
        client0.wait().expect("p0クラッシュ");
        client1.wait().expect("p1クラッシュ");
        print(i.to_string()).expect("");
    }

    // 折れ線グラフの描画
    let root_area = BitMapBackend::new(&result_image_path, (1024, 768)).into_drawing_area();
    root_area.fill(&WHITE).expect("");

    let mut chart = ChartBuilder::on(&root_area)
        .caption("Scores Over Time", ("sans-serif", 50).into_font())
        .margin(10)
        .x_label_area_size(30)
        .y_label_area_size(30)
        .build_cartesian_2d(0..loop_count, 0usize..100)
        .expect("");

    chart.configure_mesh().draw().expect("");

    chart
        .draw_series(LineSeries::new(
            scores
                .iter()
                .enumerate()
                .map(|(x, &(p0, _))| (x, p0 as usize)),
            &RED,
        ))
        .expect("")
        .label("Player 0")
        .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], RED));

    chart
        .draw_series(LineSeries::new(
            scores
                .iter()
                .enumerate()
                .map(|(x, &(_, p1))| (x, p1 as usize)),
            &BLUE,
        ))
        .expect("")
        .label("Player 1")
        .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], BLUE));

    chart
        .configure_series_labels()
        .background_style(WHITE.mix(0.8))
        .border_style(BLACK)
        .draw()
        .expect("");
}

fn main() {
    let args = Args::parse();
    client_loop(args.player0, args.player1, args.loop_count, args.max_round);
}
