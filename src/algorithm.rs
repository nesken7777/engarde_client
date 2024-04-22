use crate::protocol::Played;

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

/// total_unvisible_cards枚(山札+相手の手札)の中にtarget_unvisible_cards枚残っているカードが相手の手札(5枚)の中にmin_cards_in_enemy_hand枚以上ある確率
fn probability(min_cards_in_enemy_hand: u8, target_unvisible_cards: u8, total_unvisible_cards: u8) -> f64 {
    let min_cards_in_enemy_hand: u64 = min_cards_in_enemy_hand.into();
    let target_unvisible_cards: u64 = target_unvisible_cards.into();
    let total_unvisible_cards: u64 = total_unvisible_cards.into();
    let mut n = target_unvisible_cards;
    let mut probability: f64 = 0.0;
    while n >= min_cards_in_enemy_hand {
        probability += (combination(5,n) * permutation(target_unvisible_cards,n) * permutation(total_unvisible_cards-target_unvisible_cards,5-n)) as f64 / (permutation(total_unvisible_cards,5)) as f64;
        n -= 1;
    }
    probability
}
