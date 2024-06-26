//! ランダムに動きます 
use std::{
    io::{self, BufReader, BufWriter},
    net::{SocketAddr, TcpStream},
};

use engarde_client::{
    get_id,
    protocol::{Messages, PlayerName},
    read_stream, send_info,
    states::MyAgent,
};
use rurel::{
    strategy::{explore::RandomExploration, learn::QLearning, terminate::SinkStates},
    AgentTrainer,
};

fn random_main() -> io::Result<()> {
    let mut trainer = AgentTrainer::new();
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
    let hand_vec = hand_info.to_vec();
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
        &mut RandomExploration,
    );
    Ok(())
}

fn main() -> io::Result<()> {
    random_main()
}
