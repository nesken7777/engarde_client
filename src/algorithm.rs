use core::str;
use std::vec;

use crate::protocol::Played;

pub fn permutation(n: u64, r: u64) -> u64 {
    (n - r + 1..=n).product()
}

pub fn combination(n: u64, mut r: u64) -> u64 {
    let perm = permutation(n, r);
    perm / (1..=r).product::<u64>()
}

pub fn used_card(cards: &mut Vec<u64>, message: Played) {
    message.play_card.parse::<usize>().map(|i| cards[i] -= 1);
}
