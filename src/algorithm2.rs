use num_rational::Ratio;
use num_traits::identities::{One, Zero};
use serde::de::value::Error;
use serde::{Deserialize, Serialize};

use crate::algorithm::{used_card, ProbabilityTable, RestCards};
use crate::protocol::{Action, Attack, Direction, Movement, Played};
use std::ops::{Deref, Index, IndexMut};

const HANDS_DEFAULT_U8: u8 = 5;
const HANDS_DEFAULT_U64: u64 = HANDS_DEFAULT_U8 as u64;
const MAX_MAISUU_OF_ID_U8: u8 = 5;
const MAX_MAISUU_OF_ID_USIZE: usize = MAX_MAISUU_OF_ID_U8 as usize;
const MAX_ID: usize = 5;
const SOKUSHI_U8: u8 = HANDS_DEFAULT_U8 / 2 + 1;

//指定されたcard_idのカードを使用可能かを決める構造体
pub struct AcceptableNumbers {
    can_use: [bool; 5],
}
impl AcceptableNumbers {
    //特定の番号が使用可能かどうかを返す
    fn can_use(&self, card_id: u8) -> Result<bool, &'static str> {
        match card_id {
            1..=5 => Ok(self.can_use[card_id as usize]),
            _ => Err("カードidがおかしいよ"),
        }
    }
    //acceptablenumbers構造体に値を登録する
    fn register(&mut self, card_id: u8, value: bool) -> Result<(), &'static str> {
        match card_id {
            1..=5 => {
                self.can_use[card_id as usize] = value;
                Ok(())
            }
            _ => Err("カードidがおかしいよ"),
        }
    }

    //4と5は合計二枚以上あるなら使用可能
    fn can_use4and5(hands: &[u8; 5]) -> bool {
        count_4and5(hands) >= 2
    }
    fn can_use3(hands: &[u8; 5]) -> bool {
        hands[2] > 0
    }
    //三枚以上2があるなら使ってもよい
    fn can_use2(hands: &[u8; 5]) -> bool {
        hands[1] > 2
    }

    //1が3枚以上あるなら使ってもよい
    fn can_use1(hands: &[u8; 5], rest: RestCards) -> bool {
        let usedcard_1 = 5 - rest[0];
        hands[0] > 3 - usedcard_1
    }
    //初期化
    fn new(hands: &[u8; 5], rest: RestCards) -> AcceptableNumbers {
        let can_use = [
            Self::can_use1(hands, rest),
            Self::can_use2(hands),
            Self::can_use3(hands),
            Self::can_use4and5(hands),
            Self::can_use4and5(hands),
        ];
        AcceptableNumbers { can_use }
    }
}
impl Index<usize> for AcceptableNumbers {
    type Output = bool;
    fn index(&self, index: usize) -> &Self::Output {
        self.can_use.get(index).expect("out of bound")
    }
}

impl IndexMut<usize> for AcceptableNumbers {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.can_use.get_mut(index).expect("out of bound")
    }
}
//手札に存在する4と5の数を数える
pub fn count_4and5(hands: &[u8; 5]) -> u8 {
    (4..6).map(|i| hands[i]).sum()
}
//三枚以上持っているカードをtrueにして返す
pub fn more_than_three(hands: &[u8; 5]) -> Vec<bool> {
    (0..5).map(|i| hands[i] > 2).collect::<Vec<bool>>()
}
pub fn calc_ave(hands: &[u8; 5]) -> Ratio<u8> {
    Ratio::from_integer((0..5).map(|i| hands[i]).sum()) / Ratio::from_integer(5)
}
pub fn initial_move(
    hands: &[u8; 5],
    distance: u8,
    acceptable: AcceptableNumbers,
) -> Result<Action, &'static str> {
    //距離が12以下なら他のプログラムに任せる
    if distance <= 12 {
        return Err("距離が12以下だからこの関数は使えないよ");
    }
    //4と5が使用可能か問い合わせる

    for i in (0..5).rev() {
        if acceptable[i as usize] {
            return Ok(Action::Move(Movement {
                card: i,
                direction: Direction::Forward,
            }));
        }
    }
    //todo:平均にする

    let average = calc_ave(hands);
    //clippyに従うとエラーになった
    if average < Ratio::from_integer(3) {
        return Ok(Action::Move(Movement {
            card: 2,
            direction: Direction::Forward,
        }));
    } else {
        return Ok(Action::Move(Movement {
            card: 5,
            direction: Direction::Forward,
        }));
    }

}

pub fn normal_move() {

}
