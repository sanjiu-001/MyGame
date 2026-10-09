use std::fmt;

use crate::card::Card;
use crate::random::RandomSource;

pub const ACTION_CARD_COUNT: usize = 6;
pub const ACTION_CARD_COPIES_PER_KIND: usize = 2;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ActionCardKind {
    Peek,
    Redraw,
    Shield,
}

impl ActionCardKind {
    pub const ALL: [Self; 3] = [Self::Peek, Self::Redraw, Self::Shield];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Peek => "窥视牌",
            Self::Redraw => "重抽牌",
            Self::Shield => "护盾牌",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActionCard {
    pub id: u8,
    pub kind: ActionCardKind,
}

impl ActionCard {
    pub const fn new(id: u8, kind: ActionCardKind) -> Self {
        Self { id, kind }
    }
}

impl fmt::Display for ActionCard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{}", self.kind.label(), self.id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionDeck {
    pub cards: Vec<ActionCard>,
}

impl ActionDeck {
    pub fn new() -> Self {
        let mut cards = Vec::with_capacity(ACTION_CARD_COUNT);
        let mut id = 0;
        for kind in ActionCardKind::ALL {
            for _ in 0..ACTION_CARD_COPIES_PER_KIND {
                cards.push(ActionCard::new(id, kind));
                id += 1;
            }
        }
        Self { cards }
    }

    pub fn shuffle<R: RandomSource>(&mut self, random: &mut R) {
        for index in (1..self.cards.len()).rev() {
            let swap_index = random.gen_range(index + 1);
            self.cards.swap(index, swap_index);
        }
    }

    pub fn deal<R: RandomSource>(
        mut self,
        player_count: usize,
        random: &mut R,
    ) -> Vec<Vec<ActionCard>> {
        assert!(player_count > 0, "at least one player is required");
        self.shuffle(random);
        let mut hands = vec![Vec::new(); player_count];
        let base_count = self.cards.len() / player_count;
        let extra_count = self.cards.len() % player_count;
        let extra_player = random.gen_range(player_count);
        let mut card_index = 0;

        for player_index in 0..player_count {
            let count = base_count
                + if player_index == extra_player {
                    extra_count
                } else {
                    0
                };
            hands[player_index].extend(self.cards[card_index..card_index + count].iter().copied());
            card_index += count;
        }
        hands
    }
}

impl Default for ActionDeck {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActionCardEvent {
    Consumed {
        player: usize,
        card: ActionCard,
    },
    Redrawn {
        player: usize,
        from: usize,
        returned: Card,
        card: Card,
    },
    PeekRequested {
        player: usize,
        target: usize,
        position: usize,
    },
    PeekResolved {
        player: usize,
        target: usize,
        position: usize,
        blocked: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PendingPeek {
    pub player: usize,
    pub target: usize,
    pub position: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PeekResult {
    Blocked {
        player: usize,
        target: usize,
        position: usize,
    },
    Revealed {
        player: usize,
        target: usize,
        position: usize,
        card: Card,
    },
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::random::XorShift64;

    #[test]
    fn action_deck_has_two_of_each_kind_and_unique_ids() {
        let deck = ActionDeck::new();
        let ids: HashSet<_> = deck.cards.iter().map(|card| card.id).collect();

        assert_eq!(deck.cards.len(), ACTION_CARD_COUNT);
        assert_eq!(ids.len(), ACTION_CARD_COUNT);
        for kind in ActionCardKind::ALL {
            assert_eq!(
                deck.cards.iter().filter(|card| card.kind == kind).count(),
                2
            );
        }
    }

    #[test]
    fn fixed_seed_reproduces_action_card_deal() {
        let mut first_random = XorShift64::seeded(41);
        let mut second_random = XorShift64::seeded(41);

        let first = ActionDeck::new().deal(4, &mut first_random);
        let second = ActionDeck::new().deal(4, &mut second_random);

        assert_eq!(first, second);
        assert_eq!(first.iter().map(Vec::len).sum::<usize>(), ACTION_CARD_COUNT);
    }

    #[test]
    fn different_seeds_can_change_action_card_allocation() {
        let allocations: HashSet<_> = (0..16)
            .map(|seed| {
                let mut random = XorShift64::seeded(seed);
                ActionDeck::new().deal(4, &mut random)
            })
            .collect();

        assert!(allocations.len() > 1);
    }
}
