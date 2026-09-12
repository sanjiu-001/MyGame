use crate::card::{Card, CardKind, Rank, Suit};
use crate::random::RandomSource;

#[derive(Clone, Debug)]
pub struct Deck {
    pub cards: Vec<Card>,
}

impl Deck {
    pub fn new_with_joker() -> Self {
        let mut cards = Vec::with_capacity(53);
        let mut id = 0;
        for suit in Suit::ALL {
            for rank in Rank::ALL {
                cards.push(Card::standard(id, suit, rank));
                id += 1;
            }
        }
        cards.push(Card::joker(52));
        Self { cards }
    }

    pub fn shuffle<R: RandomSource>(&mut self, random: &mut R) {
        for index in (1..self.cards.len()).rev() {
            let swap_index = random.gen_range(index + 1);
            self.cards.swap(index, swap_index);
        }
    }

    pub fn deal<R: RandomSource>(mut self, player_count: usize, random: &mut R) -> Vec<Vec<Card>> {
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

    pub fn standard_count(&self) -> usize {
        self.cards
            .iter()
            .filter(|card| matches!(card.kind, CardKind::Standard { .. }))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::random::XorShift64;

    #[test]
    fn fresh_deck_has_53_unique_cards() {
        let deck = Deck::new_with_joker();
        let ids: HashSet<_> = deck.cards.iter().map(|card| card.id).collect();
        assert_eq!(deck.cards.len(), 53);
        assert_eq!(ids.len(), 53);
        assert_eq!(deck.standard_count(), 52);
    }

    #[test]
    fn dealing_four_players_is_13_13_13_14_in_some_order() {
        let mut random = XorShift64::seeded(7);
        let hands = Deck::new_with_joker().deal(4, &mut random);
        let mut sizes: Vec<_> = hands.iter().map(Vec::len).collect();
        sizes.sort_unstable();
        assert_eq!(sizes, vec![13, 13, 13, 14]);
    }

    #[test]
    fn same_seed_produces_same_deal() {
        let mut first_random = XorShift64::seeded(99);
        let mut second_random = XorShift64::seeded(99);
        let first = Deck::new_with_joker().deal(4, &mut first_random);
        let second = Deck::new_with_joker().deal(4, &mut second_random);
        assert_eq!(first, second);
    }
}
