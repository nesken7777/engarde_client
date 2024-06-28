//! 行動アルゴリズム集

use std::ops::{Index, IndexMut};

use num_rational::Ratio;

use crate::{
    algorithm::{card_map_from_hands, safe_possibility, win_poss_attack, ProbabilityTable},
    Action, Attack, CardID, Direction, Maisuu, Movement, RestCards,
};

/// 指定された`card_id`のカードを使用可能かを決める構造体
#[derive(Debug)]
pub struct AcceptableNumbers {
    can_use: [bool; 5],
}

impl AcceptableNumbers {
    /// 特定の番号が使用可能かどうかを返す
    // fn can_use(&self, card_id: u8) -> Result<bool, &'static str> {
    //     match card_id {
    //         1..=5 => Ok(self.can_use[card_id as usize]),
    //         _ => Err("カードidがおかしいよ"),
    //     }
    // }
    /// acceptablenumbers構造体に値を登録する
    // fn register(&mut self, card_id: u8, value: bool) -> Result<(), &'static str> {
    //     match card_id {
    //         1..=5 => {
    //             self.can_use[card_id as usize] = value;
    //             Ok(())
    //         }
    //         _ => Err("カードidがおかしいよ"),
    //     }
    // }

    //4と5は合計二枚以上あるなら使用可能
    fn can_use4and5(hands: [Maisuu; 5], distance: u8) -> bool {
        if distance >= 12 {
            count_4and5(hands) >= 2
        } else {
            true
        }
    }
    fn can_use3(hands: [Maisuu; 5]) -> bool {
        hands[2] > Maisuu::ZERO
    }
    //二枚以上2があるなら使ってもよい
    fn can_use2(hands: [Maisuu; 5]) -> bool {
        hands[1] > Maisuu::ONE
    }

    //1が3枚以上あるなら使ってもよい
    fn can_use1(hands: [Maisuu; 5], rest: RestCards) -> bool {
        let usedcard_1 = Maisuu::FIVE.saturating_sub(rest[0]);
        hands[0] > Maisuu::THREE.saturating_sub(usedcard_1)
    }
    /// 初期化
    pub fn new(hands: [Maisuu; 5], rest: RestCards, distance: u8) -> AcceptableNumbers {
        let can_use = [
            Self::can_use1(hands, rest),
            Self::can_use2(hands),
            Self::can_use3(hands),
            Self::can_use4and5(hands, distance),
            Self::can_use4and5(hands, distance),
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
/// 手札に存在する4と5の数を数えます
/// `Maisuu`はあくまでも「ある番号の上で」であるため、この関数は`Maisuu`ではなく`u8`を返します。
pub fn count_4and5(hands: [Maisuu; 5]) -> u8 {
    [Maisuu::THREE, Maisuu::FOUR, Maisuu::FIVE]
        .iter()
        .map(|&i| hands[i.denote_usize() - 1].denote())
        .sum()
}
/// 三枚以上持っているカードをtrueにして返す
pub fn more_than_three(hands: &[u8; 5]) -> Vec<bool> {
    (0..5).map(|i| hands[i] > 2).collect::<Vec<bool>>()
}
/// カード番号の大きさの平均
pub fn calc_ave(hands: &[Maisuu; 5]) -> Ratio<u8> {
    Ratio::from_integer((0..5).map(|i| hands[i].denote()).sum()) / Ratio::from_integer(5)
}
/// 最初の動きを定義する。距離が12以下の時は別のメゾットに任せる。返り値は使うべきカード
/// # Errors
/// `distance`が12より大きい場合、エラーです。
pub fn initial_move(
    hands: &[Maisuu; 5],
    distance: u8,
    acceptable: &AcceptableNumbers,
) -> Result<Action, &'static str> {
    //距離が12以下なら他のプログラムに任せる
    if distance <= 12 {
        return Err("距離が12以下だからこの関数は使えないよ");
    }
    //4と5が使用可能か問い合わせる

    for i in (0..5).rev() {
        if acceptable[usize::from(i)] && hands[usize::from(i)].denote() > 0 {
            return Ok(Action::Move(Movement::new(
                CardID::from_u8(i + 1).ok_or("意味わからんけど")?,
                Direction::Forward,
            )));
        }
    }
    //todo:平均にする

    let average = calc_ave(hands);
    //clippyに従うとエラーになった
    if average < Ratio::from_integer(3) && hands[Maisuu::TWO.denote_usize() - 1].denote() > 0 {
        Ok(Action::Move(Movement::new(
            CardID::from_u8(2).ok_or("意味わからんけど")?,
            Direction::Forward,
        )))
    } else {
        Ok(Action::Move(Movement::new(
            CardID::from_u8(5).ok_or("意味わからんけど")?,
            Direction::Forward,
        )))
    }
}
/// 自分の手札で到達し得る相手との距離のvecを返す。
/// # Panics
/// `todo!()`があります

pub fn reachable(hands: &[u8; 5], distance: u8) -> Vec<i8> {
    let vec1 = hands
        .iter()
        .map(|i| {
            if i8::try_from(distance).expect("i8の境界内") - i8::try_from(*i).expect("i8の境界内")
                > 0
            {
                i8::try_from(distance).expect("i8の境界内") - i8::try_from(*i).expect("i8の境界内")
            } else {
                todo!()
            }
        })
        .collect::<Vec<_>>();
    let vec2 = hands
        .iter()
        .map(|i| {
            i8::try_from(distance).expect("i8の境界内") + i8::try_from(*i).expect("i8の境界内")
        })
        .collect::<Vec<_>>();
    [vec1, vec2].concat()
}
/// `n`が指定する距離に行くために行うActionを返す
pub fn action_togo(n: u8, distance: u8) -> Option<Action> {
    use std::cmp::Ordering::{Equal, Greater, Less};
    match n.cmp(&distance) {
        Greater => Some(Action::Move(Movement::new(
            CardID::from_u8(n - distance)?,
            Direction::Back,
        ))),
        Less => Some(Action::Move(Movement::new(
            CardID::from_u8(distance - n)?,
            Direction::Forward,
        ))),
        Equal => None,
    }
}

/// 主に7と2の距離になるように調整するプログラム。優先度3
pub fn should_go_2_7(
    hands: [Maisuu; 5],
    distance: u8,
    rest: RestCards,
    _table: &ProbabilityTable,
) -> Option<Action> {
    let acceptable = AcceptableNumbers::new(hands, rest, distance);

    let togo7 = action_togo(7, distance);
    let togo2 = action_togo(2, distance);
    let movement_togo7 = togo7.and_then(Action::get_movement);
    let movement_togo2 = togo2.and_then(Action::get_movement);

    //7の距離に行くべき状態か判断する

    if let Some(movement) = movement_togo7 {
        if hands[movement.card().denote_usize() - 1] != Maisuu::ZERO
            && movement.direction() == Direction::Forward
            && acceptable[movement.card().denote_usize() - 1]
        {
            return togo7;
        }
    };
    //2の距離に行くべきかを判定する
    if let Some(movement) = movement_togo2 {
        if hands[movement.card().denote_usize() - 1] != Maisuu::ZERO
            && movement.direction() == Direction::Forward
            && acceptable[movement.card().denote_usize() - 1]
        {
            return togo2;
        }
    };

    None
}

/// 通常行動
/// # Panics
/// 使ってる`safe_possibility`による！
pub fn middle_move(
    hands: &[CardID],
    distance: u8,
    rest: RestCards,
    table: &ProbabilityTable,
) -> Option<Action> {
    let att_action = (distance <= 5)
        .then(|| -> Option<Action> {
            Some(Action::Attack(Attack::new(
                CardID::from_u8(distance).expect("CardIDの境界内"),
                card_map_from_hands(hands)?[usize::from(distance - 1)],
            )))
        })
        .flatten();
    //優先度高い
    let att_action = att_action.and_then(|att_action| {
        (win_poss_attack(rest, hands, table, att_action)? >= Ratio::from_integer(3) / 4)
            .then_some(att_action)
    });

    let mov_action = should_go_2_7(card_map_from_hands(hands)?, distance, rest, table)?;
    let mov_action = (safe_possibility(distance, rest, hands, table, mov_action)?
        >= Ratio::from_integer(3) / 4)
        .then_some(mov_action);

    att_action.or(mov_action)
}
