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

// pub fn safe_possibility(not_bochi: &[u64], hands: &[u64]) -> [u64; 5] {
//     let arr: [u64; 5] = (0..5)
//         .map(|i| {
//             if 5 - not_bochi[i] - hands[i] <= hands[i] {
//                 100
//             } else {
//                 let possiblity = ProbilityTable::new();
//                 let winrate = 0;
//                 let i: usize = 1;
//                 let j: usize = 1;
//                 while i < hands[i] as usize {
//                     while j < 4 {
//                         possiblity.access(i, j);
//                         j += 1;
//                     }
//                 }
//             }
//         })
//         .collect::<Vec<u64>>()
//         .try_into()
//         .unwrap();
// }
