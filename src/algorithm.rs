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

fn probability(more: u8, rest: u8, unknown: u8) -> f64 {
    let more: u64 = more.into();
    let rest: u64 = rest.into();
    let unknown: u64 = unknown.into();
    let mut n = rest;
    let mut prob: f64 = 0.0;
    while n >= more {
        prob += (combination(5, n) * permutation(rest, n) * permutation(unknown - rest, 5 - n))
            as f64
            / (permutation(unknown, 5)) as f64;
        n -= 1;
    }
    prob
}
