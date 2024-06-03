//! 基礎アルゴリズム集

use std::iter;

use num_rational::Ratio;
use num_traits::identities::{One, Zero};

use crate::{
    states::{Action, Attack, Direction, RestCards},
    CardID, Maisuu, HANDS_DEFAULT_U64, HANDS_DEFAULT_U8,
};

#[derive(Debug)]
pub struct ProbabilityTable {
    card1: [Ratio<u64>; Maisuu::MAX.denote_usize() + 1],
    card2: [Ratio<u64>; Maisuu::MAX.denote_usize() + 1],
    card3: [Ratio<u64>; Maisuu::MAX.denote_usize() + 1],
    card4: [Ratio<u64>; Maisuu::MAX.denote_usize() + 1],
    card5: [Ratio<u64>; Maisuu::MAX.denote_usize() + 1],
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

    fn card(&self, i: u8) -> Option<[Ratio<u64>; Maisuu::MAX.denote_usize() + 1]> {
        match i {
            1 => Some(self.card1),
            2 => Some(self.card2),
            3 => Some(self.card3),
            4 => Some(self.card4),
            5 => Some(self.card5),
            _ => None,
        }
    }

    fn access(&self, card: CardID, quantity: Maisuu) -> Ratio<u64> {
        use CardID::{Five, Four, One, Three, Two};
        let idx: usize = quantity.denote().into();
        match card {
            One => self.card1[idx],
            Two => self.card2[idx],
            Three => self.card3[idx],
            Four => self.card4[idx],
            Five => self.card5[idx],
        }
    }
}

/// 手札からカード番号-枚数表にする
pub fn card_map_from_hands(hands: &[CardID]) -> [Maisuu; 5] {
    use CardID::{Five, Four, One, Three, Two};
    [One, Two, Three, Four, Five]
        .into_iter()
        .map(|x| Maisuu::new(hands.iter().filter(|&&y| x == y).count() as u8).unwrap())
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}

/// カード番号-枚数表から手札にする
pub fn hands_from_card_map(card_map: &[Maisuu]) -> Vec<CardID> {
    (0..5)
        .flat_map(|i| {
            iter::repeat(CardID::from_u8(i).unwrap()).take(card_map[i as usize].denote().into())
        })
        .collect::<Vec<CardID>>()
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

/// `total_unvisible_cards`枚(山札+相手の手札)の中に`target_unvisible_cards`枚残っているカードが相手の手札(5枚)の中に`i`枚ある確率のリスト(添え字`i`)
fn probability(target_unvisible_cards: Maisuu, total_unvisible_cards: u8) -> [Ratio<u64>; 6] {
    let target_unvisible_cards: u64 = target_unvisible_cards.denote().into();
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
    fn count_cards(&self, card_id: CardID) -> Maisuu;
}

impl HandsUtil for &[CardID] {
    fn count_cards(&self, card_id: CardID) -> Maisuu {
        Maisuu::new(
            self.iter()
                .filter(|&&i| i == card_id)
                .count()
                .try_into()
                .unwrap(),
        )
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
    hands: &[CardID],
    table: &ProbabilityTable,
    action: Action,
) -> Ratio<u64> {
    match action {
        Action::Attack(attack) => {
            let i: usize = attack.card().denote().into();
            if rest_cards[i] <= hands.count_cards(attack.card()) {
                Ratio::<u64>::one()
            } else {
                calc_possibility_attack(&card_map_from_hands(hands), table, attack.card())
            }
        }
        Action::Move(movement) if matches!(movement.direction(), Direction::Forward) => {
            let card = movement.card();
            let i: usize = card.denote().into();
            // 例:手持ちdistのカードがn枚、相手と自分の距離がdist*2のとき、1枚使ったときにn-1枚でアタックされたらパリーできる。
            // そのような、相手がn-1枚以下を持っているような確率の総和
            let dup = check_twice(distance, card.denote());
            if rest_cards[i]
                <= (hands.count_cards(card).saturating_sub(if dup {
                    Maisuu::ONE
                } else {
                    Maisuu::ZERO
                }))
            {
                Ratio::<u64>::one()
            } else {
                calc_possibility_move(
                    &card_map_from_hands(hands),
                    table,
                    CardID::from_u8(distance - card.denote()).unwrap(),
                    dup,
                )
            }
        }
        Action::Move(movement) => {
            let card = movement.card();
            let i: usize = card.denote().into();
            if rest_cards[i] <= hands.count_cards(card) {
                Ratio::<u64>::one()
            } else {
                calc_possibility_move(
                    &card_map_from_hands(hands),
                    table,
                    CardID::from_u8(distance + card.denote()).unwrap(),
                    false,
                )
            }
        }
    }
}

//勝負したい距離につめるためにその距離の手札を使わなければいけないかどうか
fn check_twice(distance: u8, i: u8) -> bool {
    distance - (i * 2) == 0
}

//アタックするとき、相手にパリーされても安全な確率。兼相手が自分の枚数以下を持っている確率
fn calc_possibility_attack(
    hands: &[Maisuu],
    table: &ProbabilityTable,
    card_num: CardID,
) -> Ratio<u64> {
    // なぜ3なのかというと、3枚の攻撃の時点で勝負が決まるから
    //enemy_quantは相手のカード枚数
    [Maisuu::ZERO, Maisuu::ONE, Maisuu::TWO, Maisuu::SOKUSHI]
        .iter()
        .map(|&enemy_quant| {
            if hands[card_num as usize] >= enemy_quant {
                table.access(card_num, enemy_quant)
            } else {
                Ratio::<u64>::zero()
            }
        })
        .sum()
}

fn calc_possibility_move(
    hands: &[Maisuu],
    table: &ProbabilityTable,
    card_num: CardID,
    dup: bool,
) -> Ratio<u64> {
    [Maisuu::ONE, Maisuu::TWO, Maisuu::SOKUSHI]
        .iter()
        .map(|&i| {
            if hands[card_num as usize].saturating_sub(if dup { Maisuu::ONE } else { Maisuu::ZERO })
                >= i
            {
                table.access(card_num, i)
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
    hands: &[CardID],
    table: &ProbabilityTable,
    action: Action,
) -> Ratio<u64> {
    match action {
        Action::Attack(attack) => {
            let i: usize = attack.card().denote().into();
            if rest_cards[i] < hands.count_cards(attack.card()) {
                return Ratio::<u64>::one();
            } else {
                return calc_win_possibility(&card_map_from_hands(hands), table, attack.card());
            }
        }
        _ => return Ratio::<u64>::zero(),
    }
    fn calc_win_possibility(
        hands: &[Maisuu],
        table: &ProbabilityTable,
        card_num: CardID,
    ) -> Ratio<u64> {
        // なぜ3なのかというと、3枚の攻撃の時点で勝負が決まるから
        //enemy_quantは相手のカード枚数
        [Maisuu::ZERO, Maisuu::ONE, Maisuu::TWO, Maisuu::SOKUSHI]
            .iter()
            .map(|&enemy_quant| {
                if hands[card_num as usize] > enemy_quant {
                    table.access(card_num, enemy_quant)
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
    hands: &[Maisuu],
    position: (i64, i64),
    parried_quant: u8,
    table: &ProbabilityTable,
) -> Option<u64> {
    let distance = position.0 - position.1;
    let mut last: bool = false;
    //次に自分が行う行動が最後か否か判定。trueなら最後falseなら最後ではない
    fn check_last(parried_quant: u8, restcards: &RestCards) -> bool {
        restcards.iter().map(Maisuu::denote).sum::<u8>() <= 1 + parried_quant
    }
    //自分が行動することで届く距離を求める。
    fn reachable(distance: u64, hands: &[u8]) -> Vec<u8> {
        let mut reachable_vec = Vec::new();
        for i in hands {
            reachable_vec.push(distance as u8 + *i);
            if distance as i64 - i64::from(*i) >= 0 {
                reachable_vec.push(distance as u8 - *i);
            }
        }
        reachable_vec
    }
    let mut return_value = None;
    last = check_last(parried_quant, &restcards);
    match last {
        true => {
            let can_attack = hands[distance as usize - 1] != Maisuu::ZERO;
            let attack_action = Action::Attack(Attack::new(
                CardID::from_u8(distance as u8).unwrap(),
                hands[distance as usize],
            ));
            if can_attack {
                let possibility = win_poss_attack(
                    &restcards,
                    &hands_from_card_map(hands),
                    table,
                    attack_action,
                );
                if possibility == Ratio::one() {
                    return_value = Some(distance as u64);
                }
            }
            return_value
        }
        false => None,
    }
}
