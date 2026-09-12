use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

impl Suit {
    pub const ALL: [Self; 4] = [Self::Clubs, Self::Diamonds, Self::Hearts, Self::Spades];

    pub const fn symbol(self) -> char {
        match self {
            Self::Clubs => '♣',
            Self::Diamonds => '♦',
            Self::Hearts => '♥',
            Self::Spades => '♠',
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum Rank {
    Ace = 1,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
}

impl Rank {
    pub const ALL: [Self; 13] = [
        Self::Ace,
        Self::Two,
        Self::Three,
        Self::Four,
        Self::Five,
        Self::Six,
        Self::Seven,
        Self::Eight,
        Self::Nine,
        Self::Ten,
        Self::Jack,
        Self::Queen,
        Self::King,
    ];

    pub const fn value(self) -> u8 {
        self as u8
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Ace => "A",
            Self::Two => "2",
            Self::Three => "3",
            Self::Four => "4",
            Self::Five => "5",
            Self::Six => "6",
            Self::Seven => "7",
            Self::Eight => "8",
            Self::Nine => "9",
            Self::Ten => "10",
            Self::Jack => "J",
            Self::Queen => "Q",
            Self::King => "K",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CardKind {
    Standard { suit: Suit, rank: Rank },
    Joker,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Card {
    pub id: u8,
    pub kind: CardKind,
}

impl Card {
    pub const fn standard(id: u8, suit: Suit, rank: Rank) -> Self {
        Self {
            id,
            kind: CardKind::Standard { suit, rank },
        }
    }

    pub const fn joker(id: u8) -> Self {
        Self {
            id,
            kind: CardKind::Joker,
        }
    }

    pub const fn is_joker(self) -> bool {
        matches!(self.kind, CardKind::Joker)
    }

    pub const fn rank(self) -> Option<Rank> {
        match self.kind {
            CardKind::Standard { rank, .. } => Some(rank),
            CardKind::Joker => None,
        }
    }

    pub fn label(self) -> String {
        match self.kind {
            CardKind::Standard { suit, rank } => format!("{}{}", suit.symbol(), rank.label()),
            CardKind::Joker => "小王".to_owned(),
        }
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Pair {
    pub first: Card,
    pub second: Card,
}

impl Pair {
    pub fn new(first: Card, second: Card) -> Option<Self> {
        if first.is_joker() || second.is_joker() || first.rank() != second.rank() || first == second
        {
            None
        } else {
            Some(Self { first, second })
        }
    }

    pub fn cards(self) -> [Card; 2] {
        [self.first, self.second]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_ranks_and_suits_are_present() {
        assert_eq!(Suit::ALL.len(), 4);
        assert_eq!(Rank::ALL.len(), 13);
        assert_eq!(Rank::King.value(), 13);
    }

    #[test]
    fn card_labels_are_human_readable() {
        assert_eq!(Card::standard(0, Suit::Hearts, Rank::Ace).label(), "♥A");
        assert_eq!(Card::joker(52).to_string(), "小王");
    }

    #[test]
    fn joker_cannot_make_a_pair() {
        let ace = Card::standard(0, Suit::Clubs, Rank::Ace);
        assert!(Pair::new(ace, Card::joker(52)).is_none());
        assert!(Pair::new(ace, ace).is_none());
    }
}
