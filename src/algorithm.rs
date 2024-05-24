use num_rational::Ratio;
use num_traits::identities::{One, Zero};
use serde::{Deserialize, Serialize};

use std::ops::{Deref, Index, IndexMut};

use crate::protocol::{Action, Attack, Direction, Movement, Played};

const HANDS_DEFAULT_U8: u8 = 5;
const HANDS_DEFAULT_U64: u64 = HANDS_DEFAULT_U8 as u64;
const MAX_MAISUU_OF_ID_U8: u8 = 5;
const MAX_MAISUU_OF_ID_USIZE: usize = MAX_MAISUU_OF_ID_U8 as usize;
const MAX_ID: usize = 5;
const SOKUSHI_U8: u8 = HANDS_DEFAULT_U8 / 2 + 1;

//残りのカード枚数(種類ごと)
#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct RestCards {
    cards: [u8; MAX_ID],
}

impl RestCards {
    pub fn new() -> Self {
        Self {
            cards: [MAX_MAISUU_OF_ID_U8; MAX_ID],
        }
    }
    pub fn from_slice(slice: &[u8]) -> RestCards {
        RestCards {
            cards: slice.try_into().unwrap(),
        }
    }
}

impl Index<usize> for RestCards {
    type Output = u8;
    fn index(&self, index: usize) -> &Self::Output {
        self.cards.get(index).expect("out of bound")
    }
}

impl IndexMut<usize> for RestCards {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.cards.get_mut(index).expect("out of bound")
    }
}

impl Deref for RestCards {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.cards
    }
}

#[derive(Debug)]
pub struct ProbabilityTable {
    card1: [Ratio<u64>; MAX_MAISUU_OF_ID_USIZE + 1],
    card2: [Ratio<u64>; MAX_MAISUU_OF_ID_USIZE + 1],
    card3: [Ratio<u64>; MAX_MAISUU_OF_ID_USIZE + 1],
    card4: [Ratio<u64>; MAX_MAISUU_OF_ID_USIZE + 1],
    card5: [Ratio<u64>; MAX_MAISUU_OF_ID_USIZE + 1],
}

impl ProbabilityTable {
    pub fn new(num_of_deck: u8, cards: &RestCards) -> Self {
        let total_unvisible_cards = num_of_deck + HANDS_DEFAULT_U8;
        ProbabilityTable {
            card1: probability(cards[0], total_unvisible_cards),
            card2: probability(cards[1], total_unvisible_cards),
            card3: probability(cards[2], total_unvisible_cards),
            card4: probability(cards[3], total_unvisible_cards),
            card5: probability(cards[4], total_unvisible_cards),
        }
    }

    fn card(&self, i: u8) -> Option<[Ratio<u64>; MAX_MAISUU_OF_ID_USIZE + 1]> {
        match i {
            1 => Some(self.card1),
            2 => Some(self.card2),
            3 => Some(self.card3),
            4 => Some(self.card4),
            5 => Some(self.card5),
            _ => None,
        }
    }

    fn access(&self, card: u8, quantity: usize) -> Option<Ratio<u64>> {
        match card {
            1 => self.card1.get(quantity).copied(),
            2 => self.card2.get(quantity).copied(),
            3 => self.card3.get(quantity).copied(),
            4 => self.card4.get(quantity).copied(),
            5 => self.card5.get(quantity).copied(),
            _ => None,
        }
    }
}

fn permutation(n: u64, r: u64) -> u64 {
    if n < r {
        0
    } else {
        (n - r + 1..=n).product()
    }
}

fn combination(n: u64, r: u64) -> u64 {
    let perm = permutation(n, r);
    perm / (1..=r).product::<u64>()
}

pub fn used_card(cards: &mut RestCards, message: Played) {
    match message {
        Played::MoveMent(movement) => {
            let i: usize = movement.play_card.into();
            cards[i - 1] -= 1;
        }
        Played::Attack(attack) => {
            let i: usize = attack.play_card.into();
            cards[i - 1] = cards[i - 1].saturating_sub(attack.num_of_card * 2);
        }
    }
}

/// total_unvisible_cards枚(山札+相手の手札)の中にtarget_unvisible_cards枚残っているカードが相手の手札(5枚)の中にi枚ある確率のリスト(添え字i)
fn probability(target_unvisible_cards: u8, total_unvisible_cards: u8) -> [Ratio<u64>; 6] {
    let target_unvisible_cards: u64 = target_unvisible_cards.into();
    let total_unvisible_cards: u64 = total_unvisible_cards.into();
    (0..=HANDS_DEFAULT_U64)
        .map(|r| {
            Ratio::from_integer(
                combination(HANDS_DEFAULT_U64, r)
                    * permutation(target_unvisible_cards, r)
                    * permutation(
                        total_unvisible_cards - target_unvisible_cards,
                        HANDS_DEFAULT_U64 - r,
                    ),
            ) / permutation(total_unvisible_cards, HANDS_DEFAULT_U64)
        })
        .collect::<Vec<Ratio<u64>>>()
        .try_into()
        .unwrap()
}

trait HandsUtil {
    fn count_cards(&self, card_id: u8) -> u8;
}

impl HandsUtil for &[u8] {
    fn count_cards(&self, card_id: u8) -> u8 {
        self.iter()
            .filter(|&&i| i == card_id)
            .count()
            .try_into()
            .unwrap()
    }
}

//その行動を行った時に安全である確率を求める。distanceは相手との距離、unvisibleは墓地にあるカード枚数、handsは自分の手札、tableは相手が指定されたカードを何枚もっているか保持している構造体、actionは何かしらのアクションを指定する。
//返り値はそのアクションを行ったときの安全な確率。
pub fn safe_possibility(
    distance: u8,
    // カード番号がiのやつが墓地に何枚あるかを示す
    rest_cards: &RestCards,
    // 手札(ソート済み)
    hands: &[u8],
    table: &ProbabilityTable,
    action: Action,
) -> Ratio<u64> {
    match action {
        Action::Attack(attack) => {
            let i: usize = attack.card.into();
            if rest_cards[i] <= hands.count_cards(attack.card) {
                Ratio::<u64>::one()
            } else {
                calc_possibility_attack(hands, table, i as u64)
            }
        }
        Action::Move(Movement {
            card,
            direction: Direction::Forward,
        }) => {
            let i: usize = card.into();
            // 例:手持ちdistのカードがn枚、相手と自分の距離がdist*2のとき、1枚使ったときにn-1枚でアタックされたらパリーできる。
            // そのような、相手がn-1枚以下を持っているような確率の総和
            let dup = check_twice(distance, card);
            if rest_cards[i] <= (hands.count_cards(card) - if dup { 1 } else { 0 }) {
                Ratio::<u64>::one()
            } else {
                calc_possibility_move(hands, table, distance - card, dup)
            }
        }
        Action::Move(Movement {
            card,
            direction: Direction::Back,
        }) => {
            let i: usize = card.into();
            if rest_cards[i] <= hands.count_cards(card) {
                Ratio::<u64>::one()
            } else {
                calc_possibility_move(hands, table, distance + card, false)
            }
        }
    }
}

//勝負したい距離につめるためにその距離の手札を使わなければいけないかどうか
fn check_twice(distance: u8, i: u8) -> bool {
    distance - (i * 2) == 0
}

//アタックするとき、相手にパリーされても安全な確率。兼相手が自分の枚数以下を持っている確率
fn calc_possibility_attack(hands: &[u8], table: &ProbabilityTable, card_num: u64) -> Ratio<u64> {
    // なぜ3なのかというと、3枚の攻撃の時点で勝負が決まるから
    //enemy_quantは相手のカード枚数
    (0..SOKUSHI_U8)
        .map(|enemy_quant| {
            if hands[card_num as usize] >= enemy_quant {
                table.access(card_num as u8, enemy_quant as usize).unwrap()
            } else {
                Ratio::<u64>::zero()
            }
        })
        .sum()
}

fn calc_possibility_move(
    hands: &[u8],
    table: &ProbabilityTable,
    card_num: u8,
    dup: bool,
) -> Ratio<u64> {
    (0..SOKUSHI_U8)
        .map(|i| {
            if hands[card_num as usize] - if dup { 1 } else { 0 } >= i {
                table.access(card_num, i as usize).unwrap()
            } else {
                Ratio::<u64>::zero()
            }
        })
        .sum()
}

// pub struct Consequence {
//     status: Status,
//     cards: u64,
// }

//ゲームが始まって最初の動きを決定する。基本的に相手と交戦しない限り最も大きいカードを使う。返り値は使うべきカード番号(card_id)
pub fn initial_move(distance: u64, hands: &[u64]) -> Option<u64> {
    // 11よりも距離が大きい場合はsafe_possibilityまたはaiによる処理に任せる
    if distance < 11 {
        None
    } else {
        let mut max = 0;
        for i in hands {
            if hands[*i as usize] != 0 {
                max = *i;
            }
        }
        Some(max + 1)
    }
}
pub fn win_poss_attack(

    // カード番号がiのやつが墓地に何枚あるかを示す
    rest_cards: &RestCards,
    // 手札(ソート済み)
    hands: &[u8],
    table: &ProbabilityTable,
    action: Action,
) -> Ratio<u64> {
    match action {
        Action::Attack(attack) => {
            let i: usize = attack.card.into();
            if rest_cards[i] < hands.count_cards(attack.card) {
                return Ratio::<u64>::one();
            } else {
                return calc_win_possibility(hands, table, i as u64);
            }
        }
        _ => return Ratio::<u64>::zero(),
    }
    fn calc_win_possibility(hands: &[u8], table: &ProbabilityTable, card_num: u64) -> Ratio<u64> {
        // なぜ3なのかというと、3枚の攻撃の時点で勝負が決まるから
        //enemy_quantは相手のカード枚数
        (0..SOKUSHI_U8)
            .map(|enemy_quant| {
                if hands[card_num as usize] > enemy_quant {
                    table.access(card_num as u8, enemy_quant as usize).unwrap()
                } else {
                    Ratio::<u64>::zero()
                }
            })
            .sum()
    }
}
//最後の動きを決定する。(自分が最後動いて距離を決定できる場合)返り値は使うべきカード番号(card_id)
pub fn last_move(
    restcards: RestCards,
    hands: &[u8],
    position: (i64, i64),
    parried_quant: u8,
    table: &ProbabilityTable,
) -> Option<u64> {
    let distance = position.0 - position.1;
    let mut last: bool = false;
    //次に自分が行う行動が最後か否か判定。trueなら最後falseなら最後ではない
    fn check_last(parried_quant: u8, restcards: &RestCards) -> bool {
        restcards.iter().sum::<u8>() <= 1 + parried_quant
    }
    //自分が行動することで届く距離を求める。
    fn reachable(distance: u64, hands: &[u8]) -> Vec<u8> {
        let mut reachable_vec = Vec::new();
        for i in hands {
            reachable_vec.push(distance as u8 + *i);
            if distance as i64 - *i as i64 >= 0 {
                reachable_vec.push(distance as u8 - *i);
            }
        }
        reachable_vec
    }
    let mut return_value=None;
    last = check_last(parried_quant, &restcards);
    match last {
        
        true => {
            
            let can_attack = hands[distance as usize - 1] != 0;
            let attack_action = Action::Attack(Attack {
                card: distance as u8,
                quantity: hands[distance as usize],
            });
            if can_attack {
                let possibility =
                    win_poss_attack( &restcards, hands, table, attack_action);
                if possibility == Ratio::one() {
                    return_value=Some(distance as u64);
                }
            
        }
        return_value
    }
        false => None,
    }
}
