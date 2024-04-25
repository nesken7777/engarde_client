use std::collections::HashSet;

use rurel::mdp::State;

use crate::protocol::{
    self,
    Action::{self, Move},
    Direction::{Back, Forward},
    Movement, PlayerID,
};

#[derive(PartialEq, Eq, Hash, Clone)]
struct MyState {
    my_id: PlayerID,
    hands: [u8; 5],
    my_position: u8,
    enemy_position: u8,
}

impl State for MyState {
    type A = Action;
    fn reward(&self) -> f64 {
        // Negative Euclidean distance
        1.0
    }
    fn actions(&self) -> Vec<Action> {
        fn attack_cards(hands: [u8; 5], card: u8) -> Vec<Action> {
            let have = hands.into_iter().filter(|&x| x == card).count();
            (1..=have)
                .map(|x| {
                    Action::Attack(protocol::Attack {
                        card,
                        quantity: x as u8,
                    })
                })
                .collect()
        }
        fn decide_moves(for_back: bool, for_forward: bool, card: u8) -> Vec<Action> {
            match (for_back, for_forward) {
                (true, true) => vec![
                    Action::Move(Movement {
                        card,
                        direction: Back,
                    }),
                    Action::Move(Movement {
                        card,
                        direction: Forward,
                    }),
                ],
                (true, false) => vec![Action::Move(Movement {
                    card,
                    direction: Back,
                })],
                (false, true) => vec![Action::Move(Movement {
                    card,
                    direction: Forward,
                })],
                (false, false) => {
                    vec![]
                }
            }
        }
        let set = HashSet::from(self.hands);
        match self.my_id {
            PlayerID::Zero => {
                let moves = set
                    .into_iter()
                    .map(|card| {
                        decide_moves(
                            self.my_position - card > 0,
                            self.my_position + card < self.enemy_position,
                            card,
                        )
                    })
                    .collect::<Vec<Vec<Action>>>()
                    .concat();
                [
                    moves,
                    attack_cards(self.hands, self.enemy_position - self.my_position),
                ]
                .concat()
            }
            PlayerID::One => {
                let moves = set
                    .into_iter()
                    .map(|card| {
                        decide_moves(
                            self.my_position + card < 23,
                            self.my_position - card > self.enemy_position,
                            card,
                        )
                    })
                    .collect::<Vec<Vec<Action>>>()
                    .concat();
                [
                    moves,
                    attack_cards(self.hands, self.my_position - self.enemy_position),
                ]
                .concat()
            }
        }
    }
}
