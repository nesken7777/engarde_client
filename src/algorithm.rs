use std::ops::{Index, IndexMut};

use num_rational::Ratio;

use crate::protocol::{BoardInfo, Played};

const HANDS_DEFAULT_U8: u8 = 5;
const HANDS_DEFAULT_U64: u64 = HANDS_DEFAULT_U8 as u64;

//残りのカード枚数(種類ごと)
pub struct RestCards {
    hands: [u8; 5],
}

impl RestCards {
    pub fn new() -> Self {
        Self { hands: [5; 5] }
    }
}

impl Index<usize> for RestCards {
    type Output = u8;
    fn index(&self, index: usize) -> &Self::Output {
        self.hands.get(index).expect("out of bound")
    }
}

impl IndexMut<usize> for RestCards {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.hands.get_mut(index).expect("out of bound")
    }
}

#[derive(Debug)]
pub struct ProbabilityTable {
    card1: [Ratio<u64>; 6],
    card2: [Ratio<u64>; 6],
    card3: [Ratio<u64>; 6],
    card4: [Ratio<u64>; 6],
    card5: [Ratio<u64>; 6],
}

impl ProbabilityTable {

    pub fn new(num_of_deck: u8,cards: &RestCards) -> Self {

        let total_unvisible_cards = num_of_deck + HANDS_DEFAULT_U8;
        ProbabilityTable {
            card1: probability(cards[0], total_unvisible_cards),
            card2: probability(cards[1], total_unvisible_cards),
            card3: probability(cards[2], total_unvisible_cards),
            card4: probability(cards[3], total_unvisible_cards),
            card5: probability(cards[4], total_unvisible_cards),
        }
    }

    fn card(&self, i: u8) -> Option<[Ratio<u64>; 6]> {
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
            cards[i - 1] -= attack.num_of_card * 2;
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
pub enum Status {
    Attack,
    Move,
}
//その行動を行った時に安全である確率を求める。distanceは相手との距離、unvisibleは墓地にあるカード枚数、handsは自分の手札、tableは相手が指定されたカードを何枚もっているか保持している構造体、statusは攻撃か動きかを指定する。
//返り値はそのカードでアタックまたは行動を行ったときの安全な確率。
pub fn safe_possibility(
    distance: u64,
    unvisible: &[u64],
    hands: &[u64],
    table: &ProbabilityTable,
    status: Status,
) -> [Ratio<u64>; 5] {
    match status {
        Status::Attack => (0..5)
            .map(|i| {
                if 5 - unvisible[i] - hands[i] <= hands[i] {
                    Ratio::<u64>::from_integer(100)
                } else {
                    calc_possibility(hands, table, i as u64, false)
                }
            })
            .collect::<Vec<Ratio<u64>>>()
            .try_into()
            .unwrap(),
        Status::Move => {
            let duplicate = check_dup(distance);

            (0..5)
                .map(|i| match duplicate[i] {
                    true => {
                        if 5 - unvisible[i] - hands[i] <= (hands[i] - 1) {
                            Ratio::<u64>::from_integer(100)
                        } else {
                            calc_possibility(hands, table, i as u64, true)
                        }
                    }
                    false => {
                        if 5 - unvisible[i] - hands[i] <= hands[i] {
                            Ratio::<u64>::from_integer(100)
                        } else {
                            calc_possibility(hands, table, i as u64, false)
                        }
                    }
                })
                .collect::<Vec<Ratio<u64>>>()
                .try_into()
                .unwrap()
        }
    }
}
//勝負したい距離につめるためにその距離の手札を使わなければいけないかどうか
fn check_dup(distance: u64) -> [bool; 5] {
    let mut arr = [false; 5];
    let mut i = 0;
    while i < 5 {
        if distance - (i * 2) == 0{
            arr[i as usize] = true;
        }
        i+=1;
    }
    arr
}
//自分の手札に相手にどの距離で勝負可能かを示す
fn check_reacheable(hands: &[u64], distance: u64) -> [bool; 5] {
    let mut arr = [false; 5];
    let mut i: usize = 0;
    while i < 5 {
        match distance - i as u64 {
            1 => {
                if hands[i] != 0 {
                    arr[0] = true
                }
            },
            2 => {
                if hands[i] != 0 {
                    arr[1] = true
                }
            },
            3 => {
                if hands[i] != 0 {
                    arr[2] = true
                }
            },
            4 => {
                if hands[i] != 0 {
                    arr[3] = true
                }
            },
            5 => {
                if hands[i] != 0 {
                    arr[4] = true
                }
            },
            _=>()
        }
        i+=1;
    }
    while i < 5 {
        match distance + i as u64 {
            1 => {
                if hands[i] != 0 {
                    arr[0] = true
                }
            },
            2 => {
                if hands[i] != 0 {
                    arr[1] = true
                }
            },
            3 => {
                if hands[i] != 0 {
                    arr[2] = true
                }
            },
            4 => {
                if hands[i] != 0 {
                    arr[3] = true
                }
            },
            5 => {
                if hands[i] != 0 {
                    arr[4] = true
                }
            },
            _=>()
        }
        i+=1;
    }
    arr
}
//safe_possibilityで使う計算過程
fn calc_possibility(
    hands: &[u64],
    table: &ProbabilityTable,
    card_num: u64,
    dup: bool,
) -> Ratio<u64> {
    let mut possibility = Ratio::<u64>::from_integer(0);
    let mut j: usize = 0;
    let mut i = 0;
    match dup {
        true => {
            while i < 3 {
                if (hands[card_num as usize] - 1) >= i {
                    possibility += table.access(card_num as u8, i as usize).unwrap();
                }
                i+=1;
            }
        }
        false => {
            while i < 3 {
                if hands[card_num as usize] >= i {
                    possibility += table.access(card_num as u8, i as usize).unwrap();
                }
                i+=1;
            }
        }
    }
    possibility
}
