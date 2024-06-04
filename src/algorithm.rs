//! 基礎アルゴリズム集

use std::iter;

use num_rational::Ratio;
use num_traits::identities::{One, Zero};

use crate::{
    Action, Attack, CardID, Direction, Maisuu, RestCards, HANDS_DEFAULT_U64, HANDS_DEFAULT_U8,
};

/// 相手の手札にカード番号`i`が`j`枚ある確率
#[derive(Debug)]
pub struct ProbabilityTable {
    card1: [Ratio<u64>; Maisuu::MAX.denote_usize() + 1],
    card2: [Ratio<u64>; Maisuu::MAX.denote_usize() + 1],
    card3: [Ratio<u64>; Maisuu::MAX.denote_usize() + 1],
    card4: [Ratio<u64>; Maisuu::MAX.denote_usize() + 1],
    card5: [Ratio<u64>; Maisuu::MAX.denote_usize() + 1],
}

impl ProbabilityTable {
    /// 山札の数と`RestCards`から生成します
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

    // fn card(&self, i: u8) -> Option<[Ratio<u64>; Maisuu::MAX.denote_usize() + 1]> {
    //     match i {
    //         1 => Some(self.card1),
    //         2 => Some(self.card2),
    //         3 => Some(self.card3),
    //         4 => Some(self.card4),
    //         5 => Some(self.card5),
    //         _ => None,
    //     }
    // }

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

/// 手札からカード番号-枚数表にします。
/// `hands`の長さが5より大きい場合、`None`となります。
pub fn card_map_from_hands(hands: &[CardID]) -> Option<[Maisuu; 5]> {
    use CardID::{Five, Four, One, Three, Two};
    let map = [One, Two, Three, Four, Five]
        .into_iter()
        .map(|x| -> Option<Maisuu> {
            let have = hands.iter().filter(|&&y| x == y).count();
            Maisuu::from_usize(have)
        })
        .collect::<Option<Vec<_>>>()?
        .try_into()
        .ok()?;
    Some(map)
}

/// カード番号-枚数表から手札にします。
pub fn hands_from_card_map(card_map: &[Maisuu]) -> Option<[CardID; 5]> {
    (0..5)
        .filter_map(|i| {
            Some(
                iter::repeat(CardID::from_u8(i)?)
                    .take(card_map.get(usize::from(i)).map(Maisuu::denote_usize)?),
            )
        })
        .flatten()
        .collect::<Vec<CardID>>()
        .try_into()
        .ok()
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
        .expect("必ず長さが6")
}

trait HandsUtil {
    fn count_cards(&self, card_id: CardID) -> Maisuu;
}

impl<T: AsRef<[CardID]>> HandsUtil for T {
    fn count_cards(&self, card_id: CardID) -> Maisuu {
        Maisuu::from_usize(self.as_ref().iter().filter(|&&i| i == card_id).count())
            .expect("必ずMaisuuの境界内")
    }
}

/// その行動を行った時に安全である確率を求める。`distance`は相手との距離、`unvisible`は墓地にあるカード枚数、`hands`は自分の手札、`table`は相手が指定されたカードを何枚もっているか保持している構造体、`action`は何かしらのアクションを指定する。
/// 返り値はそのアクションを行ったときの安全な確率。
/// `None`の場合、`hands`に異常があります。
pub fn safe_possibility(
    distance: u8,
    // カード番号がiのやつが墓地に何枚あるかを示す
    rest_cards: RestCards,
    // 手札(ソート済み)
    hands: &[CardID],
    table: &ProbabilityTable,
    action: Action,
) -> Option<Ratio<u64>> {
    match action {
        Action::Attack(attack) => {
            let i: usize = attack.card().denote().into();
            if rest_cards[i] <= hands.count_cards(attack.card()) {
                Some(Ratio::<u64>::one())
            } else {
                Some(calc_possibility_attack(
                    &card_map_from_hands(hands)?,
                    table,
                    attack.card(),
                ))
            }
        }
        Action::Move(movement) if matches!(movement.direction(), Direction::Forward) => {
            let card = movement.card();
            let i = card.denote_usize();
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
                Some(Ratio::<u64>::one())
            } else if let Some(card_id) = CardID::from_u8(distance - card.denote()) {
                Some(calc_possibility_move(
                    &card_map_from_hands(hands)?,
                    table,
                    card_id,
                    dup,
                ))
            } else {
                Some(Ratio::<u64>::zero())
            }
        }
        Action::Move(movement) => {
            let card = movement.card();
            let i: usize = card.denote().into();
            if rest_cards[i] <= hands.count_cards(card) {
                Some(Ratio::<u64>::one())
            } else if let Some(card_id) = CardID::from_u8(distance + card.denote()) {
                Some(calc_possibility_move(
                    &card_map_from_hands(hands)?,
                    table,
                    card_id,
                    false,
                ))
            } else {
                Some(Ratio::<u64>::zero())
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
            if hands[usize::from(card_num.denote())] >= enemy_quant {
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
            if hands[usize::from(card_num.denote())].saturating_sub(if dup {
                Maisuu::ONE
            } else {
                Maisuu::ZERO
            }) >= i
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

/// ゲームが始まって最初の動きを決定する。基本的に相手と交戦しない限り最も大きいカードを使う。返り値は使うべきカード番号(`card_id`)
/// # Panics
/// 起きてから考える
/// すみませんこのコード何してんのか分かりません！！！！！
pub fn initial_move(distance: u64, hands: &[u64]) -> Option<u64> {
    // 11よりも距離が大きい場合はsafe_possibilityまたはaiによる処理に任せる
    if distance < 11 {
        None
    } else {
        let mut max = 0;
        for i in hands {
            if hands[usize::try_from(*i).expect("usizeの境界内")] != 0 {
                max = *i;
            }
        }
        Some(max + 1)
    }
}

/// 攻撃したときに勝てる確率
///`None`の場合、`hands`に異常があります。
pub fn win_poss_attack(
    // カード番号がiのやつが墓地に何枚あるかを示す
    rest_cards: RestCards,
    // 手札(ソート済み)
    hands: &[CardID],
    table: &ProbabilityTable,
    action: Action,
) -> Option<Ratio<u64>> {
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
                if hands[usize::from(card_num.denote())] > enemy_quant {
                    table.access(card_num, enemy_quant)
                } else {
                    Ratio::<u64>::zero()
                }
            })
            .sum()
    }
    match action {
        Action::Attack(attack) => {
            let i: usize = attack.card().denote().into();
            if rest_cards[i] < hands.count_cards(attack.card()) {
                return Some(Ratio::<u64>::one());
            }
            let win_possibility =
                calc_win_possibility(&card_map_from_hands(hands)?, table, attack.card());
            Some(win_possibility)
        }
        Action::Move(_) => Some(Ratio::<u64>::zero()),
    }
}
/// 最後の動きを決定する。(自分が最後動いて距離を決定できる場合)返り値は使うべきカード番号(`card_id`)
/// # TODO
/// なぜ`position`が`(i64, i64)`で受け取られるのですか? これはどちらが何を意味しているのですか?
/// # Panics
/// しないと思う
pub fn last_move(
    restcards: RestCards,
    hands: &[Maisuu],
    position: (i64, i64),
    parried_quant: u8,
    table: &ProbabilityTable,
) -> Option<u64> {
    //次に自分が行う行動が最後か否か判定。trueなら最後falseなら最後ではない
    fn check_last(parried_quant: u8, restcards: RestCards) -> bool {
        restcards.iter().map(Maisuu::denote).sum::<u8>() <= 1 + parried_quant
    }
    //自分が行動することで届く距離を求める。
    // fn reachable(distance: u64, hands: &[u8]) -> Vec<u8> {
    //     let mut reachable_vec = Vec::new();
    //     for i in hands {
    //         reachable_vec.push(distance as u8 + *i);
    //         if distance as i64 - i64::from(*i) >= 0 {
    //             reachable_vec.push(distance as u8 - *i);
    //         }
    //     }
    //     reachable_vec
    // }
    let distance = position.0 - position.1;
    let mut return_value = None;
    let last = check_last(parried_quant, restcards);
    if last {
        let can_attack =
            hands[usize::try_from(distance).expect("usizeの境界内") - 1] != Maisuu::ZERO;
        let attack_action = Action::Attack(Attack::new(
            CardID::from_u8(u8::try_from(distance).expect("u8の境界内")).expect("CardIDの境界内"),
            hands[usize::try_from(distance).expect("usizeの境界内")],
        ));
        if can_attack {
            let possibility = win_poss_attack(
                restcards,
                &hands_from_card_map(hands)?,
                table,
                attack_action,
            )?;
            if possibility == Ratio::one() {
                return_value = Some(u64::try_from(distance).expect("u64の境界内"));
            }
        }
        return_value
    } else {
        None
    }
}
