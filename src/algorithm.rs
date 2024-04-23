use num_rational::Ratio;

use crate::protocol::{BoardInfo, Played};

const HANDS_DEFAULT_U8: u8 = 5;
const HANDS_DEFAULT_U64: u64 = 5;

#[derive(Debug)]
pub struct ProbabilityTable {
    card1: [Ratio<u64>; 6],
    card2: [Ratio<u64>; 6],
    card3: [Ratio<u64>; 6],
    card4: [Ratio<u64>; 6],
    card5: [Ratio<u64>; 6],
}

impl ProbabilityTable {
    pub fn new() -> Self {
        const DEFAULT: Ratio<u64> = Ratio::<u64>::new_raw(1, 30);
        ProbabilityTable {
            card1: [DEFAULT; 6],
            card2: [DEFAULT; 6],
            card3: [DEFAULT; 6],
            card4: [DEFAULT; 6],
            card5: [DEFAULT; 6],
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

    pub fn update(&mut self, board: &BoardInfo, cards: &[u8]) {
        let total_unvisible_cards = board.num_of_deck + HANDS_DEFAULT_U8;
        self.card1 = probability(cards[0], total_unvisible_cards);
        self.card2 = probability(cards[1], total_unvisible_cards);
        self.card3 = probability(cards[2], total_unvisible_cards);
        self.card4 = probability(cards[3], total_unvisible_cards);
        self.card5 = probability(cards[4], total_unvisible_cards);
    }

    fn update_copy(board: &BoardInfo, cards: &[u8]) -> Self {
        let total_unvisible_cards = board.num_of_deck + HANDS_DEFAULT_U8;
        Self {
            card1: probability(cards[0], total_unvisible_cards),
            card2: probability(cards[1], total_unvisible_cards),
            card3: probability(cards[2], total_unvisible_cards),
            card4: probability(cards[3], total_unvisible_cards),
            card5: probability(cards[4], total_unvisible_cards),
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

pub fn safe_possibility(
    not_bochi: &[u64],
    hands: &[u64],
    table: &ProbabilityTable,
) -> [Ratio<u64>; 5] {
    (0..5)
        .map(|i| {
            if 5 - not_bochi[i] - hands[i] <= hands[i] {
                Ratio::<u64>::from_integer(100)
            } else {
                calc_possibility(hands, table)
            }
        })
        .collect::<Vec<Ratio<u64>>>()
        .try_into()
        .unwrap()
}
//safe_possibilityで使う計算過程
fn calc_possibility(hands: &[u64], table: &ProbabilityTable) -> Ratio<u64> {
    let mut possibility = Ratio::<u64>::from_integer(0);
    let mut j: usize = 0;
    let mut i = 0;
    while i < 5 {
        while j < 4 {
            if hands[i] >= j as u64 {
                possibility += table.access(i as u8, j).unwrap();
            }
            j += 1;
        }
        i += 1;
    }
    possibility
}
