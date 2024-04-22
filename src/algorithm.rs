use num_rational::Ratio;

use crate::protocol::Played;

const HANDS_DEFAULT_U8: u8 = 5;
const HANDS_DEFAULT_U64: u64 = 5;

pub struct ProbabilityTable {
    hand1: [Ratio<u64>; 6],
    hand2: [Ratio<u64>; 6],
    hand3: [Ratio<u64>; 6],
    hand4: [Ratio<u64>; 6],
    hand5: [Ratio<u64>; 6],
}

impl ProbabilityTable {
    fn new() -> Self {
        let default = Ratio::<u64>::new(1, 30);
        ProbabilityTable {
            hand1: [default; 6],
            hand2: [default; 6],
            hand3: [default; 6],
            hand4: [default; 6],
            hand5: [default; 6],
        }
    }

    fn hand(&self, i: u8) -> Option<[Ratio<u64>; 6]> {
        match i {
            1 => Some(self.hand1),
            2 => Some(self.hand2),
            3 => Some(self.hand3),
            4 => Some(self.hand4),
            5 => Some(self.hand5),
            _ => None,
        }
    }

    fn access(&self, hand: u8, quantity: usize) -> Option<Ratio<u64>> {
        match hand {
            1 => self.hand1.get(quantity).map(|&x| x),
            2 => self.hand2.get(quantity).map(|&x| x),
            3 => self.hand3.get(quantity).map(|&x| x),
            4 => self.hand4.get(quantity).map(|&x| x),
            5 => self.hand5.get(quantity).map(|&x| x),
            _ => None,
        }
    }
}

pub fn permutation(n: u64, r: u64) -> u64 {
    (n - r + 1..=n).product()
}

pub fn combination(n: u64, r: u64) -> u64 {
    let perm = permutation(n, r);
    perm / (1..=r).product::<u64>()
}

pub fn used_card(cards: &mut [u8], message: Played) {
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
fn probability(target_unvisible_cards: u8, total_unvisible_cards: u8) -> Vec<Ratio<u64>> {
    let target_unvisible_cards: u64 = target_unvisible_cards.into();
    let total_unvisible_cards: u64 = total_unvisible_cards.into();
    (0..=target_unvisible_cards)
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
        .collect()
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
