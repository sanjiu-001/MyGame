use std::collections::BTreeMap;
use std::fmt;

use crate::action_card::{
    ActionCard, ActionCardEvent, ActionCardKind, ActionDeck, PeekResult, PendingPeek,
};
use crate::card::{Card, Pair, Rank};
use crate::deck::Deck;
use crate::random::{RandomSource, XorShift64};

pub const PLAYER_COUNT: usize = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RoundMode {
    Normal,
    Playoff,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Player {
    pub id: usize,
    pub name: String,
    pub hand: Vec<Card>,
    pub action_cards: Vec<ActionCard>,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PendingDraw {
    pub player: usize,
    pub from: usize,
    pub first_card: Card,
    pub can_redraw: bool,
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
    PlayerFinished {
        player: usize,
        rank: usize,
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
    ActionCardNotInHand,
    ActiveActionAlreadyUsed,
    NoEligiblePeekTarget,
    PeekPositionOutOfRange,
    RedrawNotAvailable,
    PendingPeekMustBeResolved,
    PeekResultMustBeTaken,
    NoPendingPeek,
    NotPeekTarget,
    NoPeekResult,
    ShieldNotInHand,
    ShieldOnlyRespondsToPeek,
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
            Self::ActionCardNotInHand => "the selected action card is not in the player's hand",
            Self::ActiveActionAlreadyUsed => "an active action card was already used this turn",
            Self::NoEligiblePeekTarget => "there is no eligible hand to peek at",
            Self::PeekPositionOutOfRange => "the peek position is outside the target hand",
            Self::RedrawNotAvailable => "redraw is not available for this draw",
            Self::PendingPeekMustBeResolved => "the pending peek must be resolved first",
            Self::PeekResultMustBeTaken => "the private peek result must be viewed first",
            Self::NoPendingPeek => "there is no pending peek to resolve",
            Self::NotPeekTarget => "player is not the target of the pending peek",
            Self::NoPeekResult => "there is no private peek result to view",
            Self::ShieldNotInHand => "the target does not have a shield card",
            Self::ShieldOnlyRespondsToPeek => "a shield can only respond to a pending peek",
            Self::GameAlreadyFinished => "the game has already finished",
        };
        f.write_str(message)
    }
}

impl std::error::Error for GameError {}

#[derive(Clone, Debug)]
pub struct GameState {
    pub players: Vec<Player>,
    pub mode: RoundMode,
    pub discard_pile: Vec<Card>,
    pub action_discard_pile: Vec<ActionCard>,
    pub phase: GamePhase,
    pub turn_phase: TurnPhase,
    pub current_player: usize,
    pub setup_player: usize,
    pub pending_draw_player: Option<usize>,
    pub pending_draw: Option<PendingDraw>,
    pub pending_peek: Option<PendingPeek>,
    pub active_action_used: bool,
    pub finish_order: Vec<usize>,
    pub events: Vec<GameEvent>,
    private_action_events: Vec<ActionCardEvent>,
    private_peek_result: Option<PeekResult>,
}

impl GameState {
    pub fn new(names: [String; PLAYER_COUNT], seed: u64) -> Self {
        let mut random = XorShift64::seeded(seed);
        Self::new_with_random(names, &mut random)
    }

    pub fn new_with_random<R: RandomSource>(names: [String; PLAYER_COUNT], random: &mut R) -> Self {
        Self::new_with_player_names(names.into_iter().collect(), random, RoundMode::Normal)
    }

    pub fn new_with_players(players: Vec<String>, seed: u64, mode: RoundMode) -> Self {
        let mut random = XorShift64::seeded(seed);
        Self::new_with_player_names(players, &mut random, mode)
    }

    fn new_with_player_names<R: RandomSource>(
        names: Vec<String>,
        random: &mut R,
        mode: RoundMode,
    ) -> Self {
        assert!(
            (2..=PLAYER_COUNT).contains(&names.len()),
            "2 to 4 players are required"
        );
        let player_count = names.len();
        let hands = Deck::new_with_joker().deal(player_count, random);
        let action_hands = ActionDeck::new().deal(player_count, random);
        let players = names
            .into_iter()
            .enumerate()
            .map(|(id, name)| Player {
                id,
                name,
                hand: hands[id].clone(),
                action_cards: action_hands[id].clone(),
            })
            .collect();
        Self {
            players,
            mode,
            discard_pile: Vec::new(),
            action_discard_pile: Vec::new(),
            phase: GamePhase::Setup,
            turn_phase: TurnPhase::AwaitingAction,
            current_player: 0,
            setup_player: 0,
            pending_draw_player: None,
            pending_draw: None,
            pending_peek: None,
            active_action_used: false,
            finish_order: Vec::new(),
            events: vec![GameEvent::GameCreated],
            private_action_events: Vec::new(),
            private_peek_result: None,
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
        self.record_finished_if_empty(player);
        if self.setup_player + 1 < self.players.len() {
            self.setup_player += 1;
            return Ok(());
        }
        self.phase = GamePhase::Playing;
        self.current_player = self.first_active_player().unwrap_or(0);
        self.turn_phase = TurnPhase::AwaitingAction;
        self.finish_if_game_end();
        Ok(())
    }

    pub fn discard_pair(&mut self, player: usize, pair: Pair) -> Result<(), GameError> {
        self.ensure_playing_player(player)?;
        if self.turn_phase != TurnPhase::AwaitingAction {
            return Err(GameError::PendingDrawMustBeResolved);
        }
        self.remove_pair_from_hand(player, pair)?;
        self.events.push(GameEvent::TurnDiscard { player, pair });
        self.record_finished_if_empty(player);
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
        let can_redraw = self.players[from].hand.len() > 1
            && self.players[player]
                .action_cards
                .iter()
                .any(|card| card.kind == ActionCardKind::Redraw);
        let card_index = random.gen_range(self.players[from].hand.len());
        let card = self.players[from].hand.remove(card_index);
        self.players[player].hand.push(card);
        self.events.push(GameEvent::Draw { player, from, card });
        self.record_finished_if_empty(from);
        let has_pairs = !find_pairs(&self.players[player].hand).is_empty();
        if has_pairs || can_redraw {
            self.pending_draw = Some(PendingDraw {
                player,
                from,
                first_card: card,
                can_redraw,
            });
            self.pending_draw_player = Some(player);
            self.turn_phase = TurnPhase::ResolvingDraw;
        } else {
            self.finish_turn();
        }
        Ok(card)
    }

    pub fn redraw<R: RandomSource>(
        &mut self,
        player: usize,
        random: &mut R,
    ) -> Result<Card, GameError> {
        self.ensure_playing_player(player)?;
        if self.turn_phase != TurnPhase::ResolvingDraw || self.pending_draw_player != Some(player) {
            return Err(GameError::NoPendingDraw);
        }
        let pending = self.pending_draw.ok_or(GameError::NoPendingDraw)?;
        if !pending.can_redraw {
            return Err(GameError::RedrawNotAvailable);
        }
        if self.active_action_used {
            return Err(GameError::ActiveActionAlreadyUsed);
        }
        let redraw_card = self.players[player]
            .action_cards
            .iter()
            .copied()
            .find(|card| card.kind == ActionCardKind::Redraw)
            .ok_or(GameError::ActionCardNotInHand)?;
        let first_index = self.players[player]
            .hand
            .iter()
            .position(|card| *card == pending.first_card)
            .ok_or(GameError::NoPendingDraw)?;
        self.consume_action_card_internal(player, redraw_card)?;
        let returned = self.players[player].hand.remove(first_index);
        self.players[pending.from].hand.push(returned);
        let card_index = random.gen_range(self.players[pending.from].hand.len());
        let card = self.players[pending.from].hand.remove(card_index);
        self.players[player].hand.push(card);
        self.private_action_events.push(ActionCardEvent::Redrawn {
            player,
            from: pending.from,
            returned,
            card,
        });
        self.active_action_used = true;

        if find_pairs(&self.players[player].hand).is_empty() {
            self.finish_turn();
        } else {
            self.pending_draw = Some(PendingDraw {
                can_redraw: false,
                ..pending
            });
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
        self.record_finished_if_empty(player);
        self.pending_draw_player = None;
        self.pending_draw = None;
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
        self.pending_draw = None;
        self.finish_turn();
        Ok(())
    }

    pub fn action_cards(&self, player: usize) -> Result<&[ActionCard], GameError> {
        self.players
            .get(player)
            .map(|player| player.action_cards.as_slice())
            .ok_or(GameError::InvalidPlayer)
    }

    pub fn action_card_events(&self) -> &[ActionCardEvent] {
        &self.private_action_events
    }

    pub fn total_action_cards(&self) -> usize {
        self.players
            .iter()
            .map(|player| player.action_cards.len())
            .sum::<usize>()
            + self.action_discard_pile.len()
    }

    pub fn consume_action_card(
        &mut self,
        player: usize,
        card: ActionCard,
    ) -> Result<(), GameError> {
        self.ensure_playing_player(player)?;
        if self.turn_phase != TurnPhase::AwaitingAction {
            return Err(GameError::PendingDrawMustBeResolved);
        }
        if self.active_action_used {
            return Err(GameError::ActiveActionAlreadyUsed);
        }
        if card.kind == ActionCardKind::Shield {
            return Err(GameError::ShieldOnlyRespondsToPeek);
        }
        self.consume_action_card_internal(player, card)
    }

    fn consume_action_card_internal(
        &mut self,
        player: usize,
        card: ActionCard,
    ) -> Result<(), GameError> {
        if self.is_finished() {
            return Err(GameError::GameAlreadyFinished);
        }
        if self.phase != GamePhase::Playing {
            return Err(GameError::WrongPhase);
        }
        let Some(player_state) = self.players.get_mut(player) else {
            return Err(GameError::InvalidPlayer);
        };
        let Some(card_index) = player_state
            .action_cards
            .iter()
            .position(|held| *held == card)
        else {
            return Err(GameError::ActionCardNotInHand);
        };
        let consumed = player_state.action_cards.remove(card_index);
        self.action_discard_pile.push(consumed);
        self.private_action_events.push(ActionCardEvent::Consumed {
            player,
            card: consumed,
        });
        Ok(())
    }

    pub fn peek(&mut self, player: usize, position: usize) -> Result<(), GameError> {
        self.ensure_playing_player(player)?;
        if self.turn_phase != TurnPhase::AwaitingAction {
            return Err(GameError::PendingDrawMustBeResolved);
        }
        if self.active_action_used {
            return Err(GameError::ActiveActionAlreadyUsed);
        }
        let target = self
            .next_active_player(player)
            .ok_or(GameError::NoEligiblePeekTarget)?;
        if position >= self.players[target].hand.len() {
            return Err(GameError::PeekPositionOutOfRange);
        }
        let peek_card = self.players[player]
            .action_cards
            .iter()
            .copied()
            .find(|card| card.kind == ActionCardKind::Peek)
            .ok_or(GameError::ActionCardNotInHand)?;
        self.consume_action_card_internal(player, peek_card)?;
        self.active_action_used = true;
        self.pending_peek = Some(PendingPeek {
            player,
            target,
            position,
        });
        self.private_action_events
            .push(ActionCardEvent::PeekRequested {
                player,
                target,
                position,
            });
        Ok(())
    }

    pub fn respond_to_peek(&mut self, target: usize, use_shield: bool) -> Result<(), GameError> {
        let pending = self.pending_peek.ok_or(GameError::NoPendingPeek)?;
        if target != pending.target {
            return Err(GameError::NotPeekTarget);
        }
        let result = if use_shield {
            let shield = self.players[target]
                .action_cards
                .iter()
                .copied()
                .find(|card| card.kind == ActionCardKind::Shield)
                .ok_or(GameError::ShieldNotInHand)?;
            self.consume_action_card_internal(target, shield)?;
            PeekResult::Blocked {
                player: pending.player,
                target: pending.target,
                position: pending.position,
            }
        } else {
            let card = self.players[target]
                .hand
                .get(pending.position)
                .copied()
                .ok_or(GameError::PeekPositionOutOfRange)?;
            PeekResult::Revealed {
                player: pending.player,
                target: pending.target,
                position: pending.position,
                card,
            }
        };
        self.private_action_events
            .push(ActionCardEvent::PeekResolved {
                player: pending.player,
                target: pending.target,
                position: pending.position,
                blocked: use_shield,
            });
        self.pending_peek = None;
        self.private_peek_result = Some(result);
        Ok(())
    }

    pub fn take_peek_result(&mut self, player: usize) -> Result<PeekResult, GameError> {
        let result = self.private_peek_result.ok_or(GameError::NoPeekResult)?;
        let result_player = match result {
            PeekResult::Blocked { player, .. } | PeekResult::Revealed { player, .. } => player,
        };
        if player != result_player {
            return Err(GameError::NotCurrentPlayer);
        }
        self.private_peek_result
            .take()
            .ok_or(GameError::NoPeekResult)
    }

    pub fn has_peek_result(&self) -> bool {
        self.private_peek_result.is_some()
    }

    pub fn peek_target(&self, player: usize) -> Option<usize> {
        (player < self.players.len())
            .then(|| self.next_active_player(player))
            .flatten()
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

    pub fn finish_order(&self) -> &[usize] {
        &self.finish_order
    }

    pub fn final_ranking(&self) -> Option<Vec<usize>> {
        if !self.is_finished() || self.finish_order.len() != self.players.len() {
            return None;
        }
        let mut seen = vec![false; self.players.len()];
        for &player in &self.finish_order {
            if player >= self.players.len() || seen[player] {
                return None;
            }
            seen[player] = true;
        }
        Some(self.finish_order.clone())
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
        if self.pending_peek.is_some() {
            return Err(GameError::PendingPeekMustBeResolved);
        }
        if self.private_peek_result.is_some() {
            return Err(GameError::PeekResultMustBeTaken);
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
        self.finish_if_game_end();
        if self.is_finished() {
            return;
        }
        self.current_player = self
            .next_active_player(self.current_player)
            .unwrap_or(self.current_player);
        self.turn_phase = TurnPhase::AwaitingAction;
        self.active_action_used = false;
        self.pending_draw_player = None;
        self.pending_draw = None;
        self.pending_peek = None;
        self.private_peek_result = None;
        self.events.push(GameEvent::TurnAdvanced {
            player: self.current_player,
        });
    }

    fn finish_if_game_end(&mut self) {
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
            self.record_finished_player(loser);
            self.phase = GamePhase::Finished { loser };
            self.events.push(GameEvent::GameFinished { loser });
            return;
        }
        if self.mode == RoundMode::Playoff {
            let active: Vec<usize> = self
                .players
                .iter()
                .enumerate()
                .filter_map(|(player, state)| (!state.hand.is_empty()).then_some(player))
                .collect();
            if active.len() == 1 {
                let loser = active[0];
                self.record_finished_player(loser);
                self.phase = GamePhase::Finished { loser };
                self.events.push(GameEvent::GameFinished { loser });
            }
        }
    }

    fn record_finished_if_empty(&mut self, player: usize) {
        if self.players[player].hand.is_empty() {
            self.record_finished_player(player);
        }
    }

    fn record_finished_player(&mut self, player: usize) {
        if self.finish_order.contains(&player) {
            return;
        }
        let rank = self.finish_order.len() + 1;
        self.finish_order.push(player);
        self.events.push(GameEvent::PlayerFinished { player, rank });
    }

    fn first_active_player(&self) -> Option<usize> {
        self.players
            .iter()
            .position(|player| !player.hand.is_empty())
    }

    fn next_active_player(&self, player: usize) -> Option<usize> {
        (1..=self.players.len())
            .map(|offset| (player + offset) % self.players.len())
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
    use std::collections::HashSet;

    use super::*;
    use crate::action_card::ActionCardKind;
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
        assert_eq!(game.final_ranking(), Some(vec![0, 1, 3, 2]));
    }

    #[test]
    fn empty_hand_during_setup_is_recorded_in_setup_order() {
        let mut game = GameState::new(names(), 8);
        for player in &mut game.players {
            player.hand.clear();
        }
        game.players[3].hand.push(Card::joker(52));

        game.finish_setup(0).unwrap();
        game.finish_setup(1).unwrap();
        game.finish_setup(2).unwrap();
        assert_eq!(game.finish_order(), &[0, 1, 2]);
    }

    #[test]
    fn discarding_last_pair_records_the_first_formal_finisher() {
        let mut game = GameState::new(names(), 9);
        for player in 0..PLAYER_COUNT {
            game.finish_setup(player).unwrap();
        }
        game.phase = GamePhase::Playing;
        game.current_player = 0;
        game.players[0].hand = vec![
            Card::standard(0, Suit::Clubs, Rank::Ace),
            Card::standard(13, Suit::Diamonds, Rank::Ace),
        ];
        game.players[1].hand = vec![Card::standard(1, Suit::Clubs, Rank::Two)];
        game.players[2].hand = vec![Card::standard(2, Suit::Clubs, Rank::Three)];
        game.players[3].hand = vec![Card::joker(52)];

        game.discard_pair(
            0,
            Pair::new(
                Card::standard(0, Suit::Clubs, Rank::Ace),
                Card::standard(13, Suit::Diamonds, Rank::Ace),
            )
            .unwrap(),
        )
        .unwrap();

        assert_eq!(game.finish_order(), &[0]);
    }

    #[test]
    fn drawing_the_last_card_records_the_source_as_finished() {
        let mut game = GameState::new(names(), 10);
        for player in 0..PLAYER_COUNT {
            game.finish_setup(player).unwrap();
        }
        game.phase = GamePhase::Playing;
        game.current_player = 0;
        game.players[0].hand = vec![Card::standard(0, Suit::Clubs, Rank::Ace)];
        game.players[1].hand = vec![Card::standard(1, Suit::Clubs, Rank::Two)];
        game.players[2].hand = vec![Card::standard(2, Suit::Clubs, Rank::Three)];
        game.players[3].hand = vec![Card::joker(52)];

        game.draw_from_next(0, &mut XorShift64::seeded(1)).unwrap();

        assert_eq!(game.finish_order(), &[1]);
    }

    #[test]
    fn each_game_receives_six_separate_action_cards() {
        let game = GameState::new(names(), 11);
        let action_cards: Vec<_> = game
            .players
            .iter()
            .flat_map(|player| player.action_cards.iter().copied())
            .collect();
        let ids: HashSet<_> = action_cards.iter().map(|card| card.id).collect();

        assert_eq!(action_cards.len(), 6);
        assert_eq!(ids.len(), 6);
        assert_eq!(game.total_action_cards(), 6);
        for kind in ActionCardKind::ALL {
            assert_eq!(
                action_cards.iter().filter(|card| card.kind == kind).count(),
                2
            );
        }
    }

    #[test]
    fn consuming_an_action_card_keeps_it_out_of_the_public_log() {
        let mut game = GameState::new(names(), 12);
        for player in 0..PLAYER_COUNT {
            game.finish_setup(player).unwrap();
        }
        let holder = game
            .players
            .iter()
            .position(|player| !player.action_cards.is_empty())
            .unwrap();
        let card = game.players[holder]
            .action_cards
            .iter()
            .copied()
            .find(|card| card.kind != ActionCardKind::Shield)
            .unwrap();

        game.current_player = holder;
        game.consume_action_card(holder, card).unwrap();

        assert_eq!(game.total_action_cards(), 6);
        assert_eq!(game.action_discard_pile, vec![card]);
        assert_eq!(game.action_card_events().len(), 1);
        assert_eq!(game.players[holder].action_cards.contains(&card), false);
        assert!(
            game.events
                .iter()
                .all(|event| !matches!(event, GameEvent::PlayerFinished { .. }))
        );
    }

    fn prepared_peek_game() -> GameState {
        let mut game = GameState::new(names(), 13);
        for player in 0..PLAYER_COUNT {
            game.finish_setup(player).unwrap();
        }
        game.phase = GamePhase::Playing;
        game.current_player = 0;
        game.turn_phase = TurnPhase::AwaitingAction;
        game.players[0].hand = vec![Card::standard(0, Suit::Clubs, Rank::Ace)];
        game.players[1].hand = vec![
            Card::standard(1, Suit::Clubs, Rank::Two),
            Card::standard(14, Suit::Diamonds, Rank::Two),
        ];
        game.players[2].hand = vec![Card::standard(2, Suit::Clubs, Rank::Three)];
        game.players[3].hand = vec![Card::joker(52)];
        game.players[0].action_cards = vec![
            ActionCard::new(0, ActionCardKind::Peek),
            ActionCard::new(1, ActionCardKind::Redraw),
        ];
        game.players[1].action_cards = vec![
            ActionCard::new(2, ActionCardKind::Shield),
            ActionCard::new(3, ActionCardKind::Shield),
        ];
        game.players[2].action_cards = vec![ActionCard::new(4, ActionCardKind::Redraw)];
        game.players[3].action_cards = vec![ActionCard::new(5, ActionCardKind::Peek)];
        game
    }

    #[test]
    fn peek_only_targets_the_next_active_player_and_preserves_the_turn() {
        let mut game = prepared_peek_game();
        let public_events = game.events.clone();
        let hands_before = game
            .players
            .iter()
            .map(|player| player.hand.clone())
            .collect::<Vec<_>>();

        game.peek(0, 1).unwrap();

        assert_eq!(
            game.pending_peek,
            Some(PendingPeek {
                player: 0,
                target: 1,
                position: 1,
            })
        );
        assert_eq!(game.current_player, 0);
        assert_eq!(game.turn_phase, TurnPhase::AwaitingAction);
        assert_eq!(
            game.players
                .iter()
                .map(|player| player.hand.clone())
                .collect::<Vec<_>>(),
            hands_before
        );
        assert_eq!(game.events, public_events);
        assert_eq!(game.peek(0, 0), Err(GameError::PendingPeekMustBeResolved));
        assert_eq!(
            game.respond_to_peek(0, false),
            Err(GameError::NotPeekTarget)
        );
        assert_eq!(
            game.respond_to_peek(3, false),
            Err(GameError::NotPeekTarget)
        );

        game.respond_to_peek(1, false).unwrap();
        assert_eq!(
            game.take_peek_result(0),
            Ok(PeekResult::Revealed {
                player: 0,
                target: 1,
                position: 1,
                card: Card::standard(14, Suit::Diamonds, Rank::Two),
            })
        );
        assert_eq!(game.events, public_events);
        assert_eq!(game.peek(0, 0), Err(GameError::ActiveActionAlreadyUsed));

        game.draw_from_next(0, &mut XorShift64::seeded(2)).unwrap();
        assert_eq!(game.current_player, 0);
        assert_eq!(game.turn_phase, TurnPhase::ResolvingDraw);
    }

    #[test]
    fn shield_blocks_peek_and_is_consumed_without_using_an_active_action() {
        let mut game = prepared_peek_game();

        game.peek(0, 0).unwrap();
        game.respond_to_peek(1, true).unwrap();
        assert_eq!(
            game.take_peek_result(0),
            Ok(PeekResult::Blocked {
                player: 0,
                target: 1,
                position: 0,
            })
        );
        assert_eq!(
            game.players[1]
                .action_cards
                .iter()
                .filter(|card| card.kind == ActionCardKind::Shield)
                .count(),
            1
        );
        assert_eq!(game.action_discard_pile.len(), 2);
        assert!(game.active_action_used);
        assert_eq!(game.respond_to_peek(1, true), Err(GameError::NoPendingPeek));
    }

    #[test]
    fn shield_cannot_be_consumed_without_a_pending_peek() {
        let mut game = prepared_peek_game();
        let shield = game.players[1]
            .action_cards
            .iter()
            .copied()
            .find(|card| card.kind == ActionCardKind::Shield)
            .unwrap();
        game.current_player = 1;

        assert_eq!(
            game.consume_action_card(1, shield),
            Err(GameError::ShieldOnlyRespondsToPeek)
        );
        assert!(game.players[1].action_cards.contains(&shield));
    }

    #[test]
    fn playoff_game_with_two_players_finishes_when_one_active_player_remains() {
        let mut game = GameState::new_with_players(
            vec!["甲".to_owned(), "乙".to_owned()],
            120,
            RoundMode::Playoff,
        );
        for player in &mut game.players {
            player.hand.clear();
        }
        game.players[0]
            .hand
            .push(Card::standard(0, Suit::Spades, Rank::Ace));
        game.players[1]
            .hand
            .push(Card::standard(1, Suit::Hearts, Rank::King));
        game.phase = GamePhase::Playing;
        game.current_player = 0;

        let mut random = XorShift64::seeded(121);
        game.draw_from_next(0, &mut random).unwrap();

        assert!(game.is_finished());
        assert_eq!(game.final_ranking(), Some(vec![1, 0]));
    }

    #[test]
    fn peek_position_must_be_inside_the_next_hand() {
        let mut game = prepared_peek_game();

        assert_eq!(game.peek(0, 2), Err(GameError::PeekPositionOutOfRange));
        assert!(
            game.players[0]
                .action_cards
                .iter()
                .any(|card| card.kind == ActionCardKind::Peek)
        );
    }

    #[test]
    fn peek_is_rejected_after_the_formal_draw_has_started() {
        let mut game = prepared_peek_game();
        game.players[0].hand = vec![Card::standard(1, Suit::Clubs, Rank::Two)];
        game.players[1].hand = vec![
            Card::standard(14, Suit::Diamonds, Rank::Two),
            Card::standard(2, Suit::Clubs, Rank::Three),
        ];

        game.draw_from_next(0, &mut XorShift64::seeded(2)).unwrap();

        assert_eq!(game.turn_phase, TurnPhase::ResolvingDraw);
        assert_eq!(game.peek(0, 0), Err(GameError::PendingDrawMustBeResolved));
    }

    #[test]
    fn redraw_returns_the_first_card_to_the_same_player_and_replaces_it() {
        let mut game = prepared_peek_game();
        let public_events_before_redraw;
        let source_before = game.players[1].hand.clone();
        let before_total = game.total_cards_in_hands();

        let first_card = game.draw_from_next(0, &mut XorShift64::seeded(3)).unwrap();
        let pending = game.pending_draw.unwrap();
        assert_eq!(pending.from, 1);
        assert_eq!(pending.first_card, first_card);
        assert!(pending.can_redraw);
        public_events_before_redraw = game.events.clone();

        let replacement = game.redraw(0, &mut XorShift64::seeded(4)).unwrap();

        assert_eq!(game.total_cards_in_hands(), before_total);
        assert_eq!(game.current_player, 1);
        assert_eq!(game.pending_draw, None);
        assert!(source_before.contains(&first_card));
        assert!(source_before.contains(&replacement));
        assert!(
            !game.players[0]
                .action_cards
                .iter()
                .any(|card| card.kind == ActionCardKind::Redraw)
        );
        assert_eq!(
            &game.events[..public_events_before_redraw.len()],
            &public_events_before_redraw
        );
        assert!(game.action_card_events().iter().any(|event| matches!(
            event,
            ActionCardEvent::Redrawn {
                player: 0,
                from: 1,
                ..
            }
        )));
    }

    #[test]
    fn redraw_requires_a_pending_draw_and_a_source_with_more_than_one_card() {
        let mut game = prepared_peek_game();
        assert_eq!(
            game.redraw(0, &mut XorShift64::seeded(5)),
            Err(GameError::NoPendingDraw)
        );

        game.players[0].hand = vec![Card::standard(1, Suit::Clubs, Rank::Two)];
        game.players[1].hand = vec![Card::standard(14, Suit::Diamonds, Rank::Two)];
        game.draw_from_next(0, &mut XorShift64::seeded(6)).unwrap();
        assert_eq!(game.pending_draw.unwrap().can_redraw, false);
        assert_eq!(
            game.redraw(0, &mut XorShift64::seeded(7)),
            Err(GameError::RedrawNotAvailable)
        );
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
