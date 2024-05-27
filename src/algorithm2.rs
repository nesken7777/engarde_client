use num_rational::Ratio;
use num_traits::identities::{One, Zero};
use serde::de::value::Error;
use serde::{Deserialize, Serialize};

use crate::algorithm::{used_card, ProbabilityTable, RestCards};
use crate::protocol::{Action, Attack, Direction, Movement, Played};
use core::panic;
use std::ops::{Deref, Index, IndexMut};
use std::vec;

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
//最初の動きを定義する。距離が12以下の時は別のメゾットに任せる。返り値は使うべきカード
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
//自分の手札で到達し得る相手との距離のvecを返す。
pub fn reachable(hands: &[u8; 5], distance: u8) -> Vec<i8> {
    let mut vec1 = hands
        .into_iter()
        .map(|i| {
            if distance as i8 - *i as i8 > 0 {
                distance as i8 - *i as i8
            } else {
                todo!()
            }
        })
        .collect::<Vec<_>>();
    let mut vec2 = hands
        .into_iter()
        .map(|i| distance as i8 + *i as i8)
        .collect::<Vec<_>>();
    let vec = [vec1, vec2].concat();
    vec
}
//nが指定する距離に行くために行うActionを返す
pub fn action_togo(n: u8, distance: u8) -> Option<Action> {
    //値を比較する関数
    fn compare_numbers(num1: i32, num2: i32) -> i8 {
        match num1.cmp(&num2) {
            std::cmp::Ordering::Greater => 1,
            std::cmp::Ordering::Equal => 0,
            std::cmp::Ordering::Less => -1,
        }
    }
    //行くべき方向を判定する関数
    fn check_direction(n: i32, distance: i32) -> Option<Direction> {
        match compare_numbers(n as i32, distance as i32) {
            1 => Some(Direction::Back),
            -1 => Some(Direction::Forward),
            0 => None,
            _ => unreachable!(),
        }
    }
    let direct = check_direction(n as i32, distance as i32)?;
    match compare_numbers(n as i32, distance as i32) {
        1 => Some(Action::Move(Movement {
            card: n - distance,
            direction: direct,
        })),
        -1 => Some(Action::Move(Movement {
            card: distance - n,
            direction: direct,
        })),
        _ => None,
    }
}

pub fn normal_move(hands: &[u8; 5], distance: u8,rest:RestCards,table:ProbabilityTable) -> Option<Action> {
    let acceptable=AcceptableNumbers::new(hands, rest);
    let togo7 = action_togo(7, distance);
    let togo2 = action_togo(2, distance);
    let movement_togo7 = togo7.and_then(|act|act.get_movement());
    let movement_togo2 = togo2.and_then(|act|act.get_movement());
    if let Some(movement) = movement_togo7 {
        if hands[movement.card as usize] != 0 && movement.direction == Direction::Forward&&acceptable[movement.card as usize] {
            return togo7;
        }
    };
    if let Some(movement) = movement_togo2 {
        if hands[movement.card as usize] != 0 && movement.direction == Direction::Forward {
            return togo2;
        }
    };
    
    None 
}
