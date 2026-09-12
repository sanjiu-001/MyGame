use std::collections::BTreeMap;
use std::fmt;

use crate::card::{Card, Pair, Rank};
use crate::deck::Deck;
use crate::random::{RandomSource, XorShift64};

pub const PLAYER_COUNT: usize = 4;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Player {
    pub id: usize,
    pub name: String,
    pub hand: Vec<Card>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GamePhase {
    Setup,
    Playing,
    Finished { loser: usize },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TurnPhase {
    AwaitingAction,
    ResolvingDraw,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GameEvent {
    GameCreated,
    SetupDiscard {
        player: usize,
        pair: Pair,
    },
    TurnDiscard {
        player: usize,
        pair: Pair,
    },
    Draw {
        player: usize,
        from: usize,
        card: Card,
    },
    KeepDrawnPair {
        player: usize,
    },
    TurnAdvanced {
        player: usize,
    },
    GameFinished {
        loser: usize,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GameError {
    InvalidPlayer,
    NotCurrentPlayer,
    WrongPhase,
    PairNotInHand,
    CannotDrawFromEmptyTable,
    NoPendingDraw,
    PendingDrawMustBeResolved,
    GameAlreadyFinished,
}

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidPlayer => "invalid player",
            Self::NotCurrentPlayer => "player is not the current player",
            Self::WrongPhase => "action is not valid in the current phase",
            Self::PairNotInHand => "the selected pair is not in the player's hand",
            Self::CannotDrawFromEmptyTable => "there is no eligible hand to draw from",
            Self::NoPendingDraw => "there is no pending draw to resolve",
            Self::PendingDrawMustBeResolved => "the pending draw must be resolved first",
            Self::GameAlreadyFinished => "the game has already finished",
        };
        f.write_str(message)
    }
}

impl std::error::Error for GameError {}

#[derive(Clone, Debug)]
pub struct GameState {
    pub players: Vec<Player>,
    pub discard_pile: Vec<Card>,
    pub phase: GamePhase,
    pub turn_phase: TurnPhase,
    pub current_player: usize,
    pub setup_player: usize,
    pub pending_draw_player: Option<usize>,
    pub events: Vec<GameEvent>,
}

impl GameState {
    pub fn new(names: [String; PLAYER_COUNT], seed: u64) -> Self {
        let mut random = XorShift64::seeded(seed);
        Self::new_with_random(names, &mut random)
    }

    pub fn new_with_random<R: RandomSource>(names: [String; PLAYER_COUNT], random: &mut R) -> Self {
        let hands = Deck::new_with_joker().deal(PLAYER_COUNT, random);
        let players = names
            .into_iter()
            .enumerate()
            .map(|(id, name)| Player {
                id,
                name,
                hand: hands[id].clone(),
            })
            .collect();
        Self {
            players,
            discard_pile: Vec::new(),
            phase: GamePhase::Setup,
            turn_phase: TurnPhase::AwaitingAction,
            current_player: 0,
            setup_player: 0,
            pending_draw_player: None,
            events: vec![GameEvent::GameCreated],
        }
    }

    pub fn available_pairs(&self, player: usize) -> Result<Vec<Pair>, GameError> {
        Ok(find_pairs(self.hand(player)?))
    }

    pub fn setup_discard(&mut self, player: usize, pair: Pair) -> Result<(), GameError> {
        if self.phase != GamePhase::Setup {
            return Err(GameError::WrongPhase);
        }
        if player != self.setup_player {
            return Err(GameError::NotCurrentPlayer);
        }
        self.remove_pair_from_hand(player, pair)?;
        self.events.push(GameEvent::SetupDiscard { player, pair });
        Ok(())
    }

    pub fn finish_setup(&mut self, player: usize) -> Result<(), GameError> {
        if self.phase != GamePhase::Setup {
            return Err(GameError::WrongPhase);
        }
        if player != self.setup_player {
            return Err(GameError::NotCurrentPlayer);
        }
        if self.setup_player + 1 < PLAYER_COUNT {
            self.setup_player += 1;
            return Ok(());
        }
        self.phase = GamePhase::Playing;
        self.current_player = self.first_active_player().unwrap_or(0);
        self.turn_phase = TurnPhase::AwaitingAction;
        self.finish_if_only_joker_remains();
        Ok(())
    }

    pub fn discard_pair(&mut self, player: usize, pair: Pair) -> Result<(), GameError> {
        self.ensure_playing_player(player)?;
        if self.turn_phase != TurnPhase::AwaitingAction {
            return Err(GameError::PendingDrawMustBeResolved);
        }
        self.remove_pair_from_hand(player, pair)?;
        self.events.push(GameEvent::TurnDiscard { player, pair });
        self.finish_turn();
        Ok(())
    }

    pub fn draw_from_next<R: RandomSource>(
        &mut self,
        player: usize,
        random: &mut R,
    ) -> Result<Card, GameError> {
        self.ensure_playing_player(player)?;
        if self.turn_phase != TurnPhase::AwaitingAction {
            return Err(GameError::PendingDrawMustBeResolved);
        }
        let from = self
            .next_active_player(player)
            .ok_or(GameError::CannotDrawFromEmptyTable)?;
        let card_index = random.gen_range(self.players[from].hand.len());
        let card = self.players[from].hand.remove(card_index);
        self.players[player].hand.push(card);
        self.events.push(GameEvent::Draw { player, from, card });
        if find_pairs(&self.players[player].hand).is_empty() {
            self.finish_turn();
        } else {
            self.turn_phase = TurnPhase::ResolvingDraw;
            self.pending_draw_player = Some(player);
        }
        Ok(card)
    }

    pub fn discard_after_draw(&mut self, player: usize, pair: Pair) -> Result<(), GameError> {
        self.ensure_playing_player(player)?;
        if self.turn_phase != TurnPhase::ResolvingDraw || self.pending_draw_player != Some(player) {
            return Err(GameError::NoPendingDraw);
        }
        self.remove_pair_from_hand(player, pair)?;
        self.events.push(GameEvent::TurnDiscard { player, pair });
        self.pending_draw_player = None;
        self.finish_turn();
        Ok(())
    }

    pub fn keep_drawn_pairs(&mut self, player: usize) -> Result<(), GameError> {
        self.ensure_playing_player(player)?;
        if self.turn_phase != TurnPhase::ResolvingDraw || self.pending_draw_player != Some(player) {
            return Err(GameError::NoPendingDraw);
        }
        self.events.push(GameEvent::KeepDrawnPair { player });
        self.pending_draw_player = None;
        self.finish_turn();
        Ok(())
    }

    pub fn hand(&self, player: usize) -> Result<&[Card], GameError> {
        self.players
            .get(player)
            .map(|player| player.hand.as_slice())
            .ok_or(GameError::InvalidPlayer)
    }

    pub fn is_finished(&self) -> bool {
        matches!(self.phase, GamePhase::Finished { .. })
    }

    pub fn loser(&self) -> Option<usize> {
        match self.phase {
            GamePhase::Finished { loser } => Some(loser),
            _ => None,
        }
    }

    pub fn total_cards_in_hands(&self) -> usize {
        self.players.iter().map(|player| player.hand.len()).sum()
    }

    fn ensure_playing_player(&self, player: usize) -> Result<(), GameError> {
        if self.is_finished() {
            return Err(GameError::GameAlreadyFinished);
        }
        if self.phase != GamePhase::Playing {
            return Err(GameError::WrongPhase);
        }
        if player != self.current_player {
            return Err(GameError::NotCurrentPlayer);
        }
        Ok(())
    }

    fn remove_pair_from_hand(&mut self, player: usize, pair: Pair) -> Result<(), GameError> {
        let hand = &mut self.players[player].hand;
        let first_index = hand.iter().position(|card| *card == pair.first);
        let second_index = hand.iter().position(|card| *card == pair.second);
        let (Some(first_index), Some(second_index)) = (first_index, second_index) else {
            return Err(GameError::PairNotInHand);
        };
        if first_index == second_index
            || pair.first.rank().is_none()
            || pair.first.rank() != pair.second.rank()
        {
            return Err(GameError::PairNotInHand);
        }
        let (higher, lower) = if first_index > second_index {
            (first_index, second_index)
        } else {
            (second_index, first_index)
        };
        self.discard_pile.push(hand.remove(higher));
        self.discard_pile.push(hand.remove(lower));
        Ok(())
    }

    fn finish_turn(&mut self) {
        self.finish_if_only_joker_remains();
        if self.is_finished() {
            return;
        }
        self.current_player = self
            .next_active_player(self.current_player)
            .unwrap_or(self.current_player);
        self.turn_phase = TurnPhase::AwaitingAction;
        self.events.push(GameEvent::TurnAdvanced {
            player: self.current_player,
        });
    }

    fn finish_if_only_joker_remains(&mut self) {
        let remaining: Vec<Card> = self
            .players
            .iter()
            .flat_map(|player| player.hand.iter().copied())
            .collect();
        if remaining.len() == 1 && remaining[0].is_joker() {
            let loser = self
                .players
                .iter()
                .position(|player| player.hand.contains(&remaining[0]))
                .unwrap_or(0);
            self.phase = GamePhase::Finished { loser };
            self.events.push(GameEvent::GameFinished { loser });
        }
    }

    fn first_active_player(&self) -> Option<usize> {
        self.players
            .iter()
            .position(|player| !player.hand.is_empty())
    }

    fn next_active_player(&self, player: usize) -> Option<usize> {
        (1..=PLAYER_COUNT)
            .map(|offset| (player + offset) % PLAYER_COUNT)
            .find(|index| *index != player && !self.players[*index].hand.is_empty())
    }
}

pub fn find_pairs(hand: &[Card]) -> Vec<Pair> {
    let mut by_rank: BTreeMap<Rank, Vec<Card>> = BTreeMap::new();
    for card in hand.iter().copied() {
        if let Some(rank) = card.rank() {
            by_rank.entry(rank).or_default().push(card);
        }
    }
    by_rank
        .into_values()
        .flat_map(|cards| {
            cards
                .chunks_exact(2)
                .filter_map(|chunk| Pair::new(chunk[0], chunk[1]))
                .collect::<Vec<_>>()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardKind, Suit};

    fn names() -> [String; PLAYER_COUNT] {
        ["甲", "乙", "丙", "丁"].map(str::to_owned)
    }

    #[test]
    fn pair_detection_ignores_suit_but_not_joker() {
        let hand = [
            Card::standard(0, Suit::Clubs, Rank::Ace),
            Card::standard(13, Suit::Diamonds, Rank::Ace),
            Card::standard(1, Suit::Clubs, Rank::Two),
            Card::joker(52),
        ];
        let pairs = find_pairs(&hand);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].first.rank(), Some(Rank::Ace));
    }

    #[test]
    fn setup_can_discard_or_keep_pairs() {
        let mut game = GameState::new(names(), 1);
        let pairs = game.available_pairs(0).unwrap();
        if let Some(pair) = pairs.first().copied() {
            game.setup_discard(0, pair).unwrap();
        }
        let before = game.hand(0).unwrap().len();
        game.finish_setup(0).unwrap();
        assert!(game.hand(0).unwrap().len() <= before + 2);
    }

    #[test]
    fn setup_requires_all_players_before_playing() {
        let mut game = GameState::new(names(), 2);
        for player in 0..PLAYER_COUNT {
            game.finish_setup(player).unwrap();
        }
        assert_eq!(game.phase, GamePhase::Playing);
    }

    #[test]
    fn non_current_player_cannot_act() {
        let mut game = GameState::new(names(), 3);
        for player in 0..PLAYER_COUNT {
            game.finish_setup(player).unwrap();
        }
        let error = game
            .draw_from_next(1, &mut XorShift64::seeded(1))
            .unwrap_err();
        assert_eq!(error, GameError::NotCurrentPlayer);
    }

    #[test]
    fn drawing_moves_exactly_one_card_between_hands() {
        let mut game = GameState::new(names(), 4);
        for player in 0..PLAYER_COUNT {
            game.finish_setup(player).unwrap();
        }
        let current = game.current_player;
        let from = game.next_active_player(current).unwrap();
        let before_current = game.hand(current).unwrap().len();
        let before_from = game.hand(from).unwrap().len();
        game.draw_from_next(current, &mut XorShift64::seeded(5))
            .unwrap();
        assert_eq!(game.total_cards_in_hands(), 53 - game.discard_pile.len());
        assert_eq!(game.hand(current).unwrap().len(), before_current + 1);
        assert_eq!(game.hand(from).unwrap().len(), before_from - 1);
    }

    #[test]
    fn empty_players_are_skipped() {
        let mut game = GameState::new(names(), 6);
        game.players[1].hand.clear();
        game.players[2].hand.clear();
        game.players[3].hand.clear();
        for player in 0..PLAYER_COUNT {
            game.finish_setup(player).unwrap();
        }
        assert_eq!(game.current_player, 0);
        let error = game
            .draw_from_next(0, &mut XorShift64::seeded(1))
            .unwrap_err();
        assert_eq!(error, GameError::CannotDrawFromEmptyTable);
    }

    #[test]
    fn only_joker_finishes_and_identifies_loser() {
        let mut game = GameState::new(names(), 7);
        for player in &mut game.players {
            player.hand.clear();
        }
        game.players[2].hand.push(Card::joker(52));
        for player in 0..PLAYER_COUNT {
            game.finish_setup(player).unwrap();
        }
        assert_eq!(game.phase, GamePhase::Finished { loser: 2 });
        assert_eq!(game.loser(), Some(2));
    }

    #[test]
    fn invalid_pair_cannot_be_discarded() {
        let not_a_pair = Pair::new(
            Card::standard(0, Suit::Clubs, Rank::Ace),
            Card::standard(1, Suit::Clubs, Rank::Two),
        );
        assert!(not_a_pair.is_none());
        assert!(matches!(CardKind::Joker, CardKind::Joker));
    }
}
