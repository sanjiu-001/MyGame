use std::collections::HashSet;
use std::fmt;

use crate::action_card::ActionCardKind;
use crate::action_card::{ACTION_CARD_COPIES_PER_KIND, ACTION_CARD_COUNT};
use crate::game::{GameError, GamePhase, GameState, PLAYER_COUNT, RoundMode, TurnPhase};
use crate::match_state::{MatchPhase, MatchState};
use crate::random::XorShift64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimulationReport {
    pub seed: u64,
    pub turns: usize,
    pub loser: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchSimulationReport {
    pub seed: u64,
    pub rounds: u32,
    pub turns: usize,
    pub winner: usize,
    pub playoff_players: Vec<usize>,
    pub active_actions: usize,
    pub peeks: usize,
    pub shields: usize,
    pub redraws: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimulationError {
    Rule { turn: usize, error: GameError },
    Invariant { turn: usize, message: String },
    MaxTurnsExceeded { seed: u64, max_turns: usize },
    MaxRoundsExceeded { seed: u64, max_rounds: u32 },
}

impl fmt::Display for SimulationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rule { turn, error } => write!(f, "rule error on turn {turn}: {error}"),
            Self::Invariant { turn, message } => {
                write!(f, "invariant error on turn {turn}: {message}")
            }
            Self::MaxTurnsExceeded { seed, max_turns } => {
                write!(f, "seed {seed} exceeded the {max_turns}-turn limit")
            }
            Self::MaxRoundsExceeded { seed, max_rounds } => {
                write!(f, "seed {seed} exceeded the {max_rounds}-round limit")
            }
        }
    }
}

impl std::error::Error for SimulationError {}

pub fn simulate_greedy(seed: u64, max_turns: usize) -> Result<SimulationReport, SimulationError> {
    let names = ["甲", "乙", "丙", "丁"].map(str::to_owned);
    let mut random = XorShift64::seeded(seed);
    let mut game = GameState::new_with_random(names, &mut random);

    for player in 0..PLAYER_COUNT {
        while let Some(pair) = game
            .available_pairs(player)
            .map_err(|error| SimulationError::Rule { turn: 0, error })?
            .first()
            .copied()
        {
            game.setup_discard(player, pair)
                .map_err(|error| SimulationError::Rule { turn: 0, error })?;
        }
        game.finish_setup(player)
            .map_err(|error| SimulationError::Rule { turn: 0, error })?;
    }

    validate_state(&game, 0)?;

    for turn in 1..=max_turns {
        if game.is_finished() {
            return Ok(SimulationReport {
                seed,
                turns: turn - 1,
                loser: game.loser().expect("finished game has a loser"),
            });
        }

        let player = game.current_player;
        match game.turn_phase {
            TurnPhase::AwaitingAction => {
                let pair = game
                    .available_pairs(player)
                    .map_err(|error| SimulationError::Rule { turn, error })?
                    .first()
                    .copied();
                if let Some(pair) = pair {
                    game.discard_pair(player, pair)
                        .map_err(|error| SimulationError::Rule { turn, error })?;
                } else {
                    game.draw_from_next(player, &mut random)
                        .map_err(|error| SimulationError::Rule { turn, error })?;
                }
            }
            TurnPhase::ResolvingDraw => {
                let pair = game
                    .available_pairs(player)
                    .map_err(|error| SimulationError::Rule { turn, error })?
                    .first()
                    .copied();
                if let Some(pair) = pair {
                    game.discard_after_draw(player, pair)
                        .map_err(|error| SimulationError::Rule { turn, error })?;
                } else {
                    game.keep_drawn_pairs(player)
                        .map_err(|error| SimulationError::Rule { turn, error })?;
                }
            }
        }
        validate_state(&game, turn)?;
    }

    Err(SimulationError::MaxTurnsExceeded { seed, max_turns })
}

pub fn simulate_match_greedy(
    seed: u64,
    max_turns: usize,
    max_rounds: u32,
) -> Result<MatchSimulationReport, SimulationError> {
    let names = ["甲", "乙", "丙", "丁"].map(str::to_owned);
    let mut state = MatchState::new(names, seed);
    let mut random = XorShift64::seeded(seed ^ 0xD1A6_5EED);
    let mut turns = 0;
    let mut active_actions = 0;
    let mut peeks = 0;
    let mut shields = 0;
    let mut redraws = 0;

    loop {
        if matches!(state.phase(), MatchPhase::Finished { .. }) {
            let winner = state.winner().expect("finished match has a winner");
            return Ok(MatchSimulationReport {
                seed,
                rounds: state.round(),
                turns,
                winner,
                playoff_players: state.playoff_ranking().unwrap_or(&[]).to_vec(),
                active_actions,
                peeks,
                shields,
                redraws,
            });
        }
        if state.round() > max_rounds {
            return Err(SimulationError::MaxRoundsExceeded { seed, max_rounds });
        }

        let game = state.game_mut();
        if game.phase == GamePhase::Setup {
            finish_setup_greedily(game)?;
            validate_state(game, turns)?;
            continue;
        }
        if game.is_finished() {
            collect_action_metrics(
                game,
                &mut active_actions,
                &mut peeks,
                &mut shields,
                &mut redraws,
            );
            match state.phase() {
                MatchPhase::Playing => {
                    state
                        .settle_round()
                        .map_err(|error| SimulationError::Invariant {
                            turn: turns,
                            message: error.to_string(),
                        })?;
                }
                MatchPhase::Playoff { .. } => {
                    state
                        .finish_playoff()
                        .map_err(|error| SimulationError::Invariant {
                            turn: turns,
                            message: error.to_string(),
                        })?;
                }
                MatchPhase::PlayoffPending { .. } => {
                    state
                        .start_playoff()
                        .map_err(|error| SimulationError::Invariant {
                            turn: turns,
                            message: error.to_string(),
                        })?;
                }
                MatchPhase::Finished { .. } => {}
            }
            continue;
        }

        if turns >= max_turns {
            return Err(SimulationError::MaxTurnsExceeded { seed, max_turns });
        }
        play_simulation_turn(game, &mut random, turns)?;
        turns += 1;
        validate_state(game, turns)?;
    }
}

fn collect_action_metrics(
    game: &GameState,
    active_actions: &mut usize,
    peeks: &mut usize,
    shields: &mut usize,
    redraws: &mut usize,
) {
    for event in game.action_card_events() {
        match event {
            crate::action_card::ActionCardEvent::Consumed { card, .. } => {
                *active_actions += usize::from(card.kind != ActionCardKind::Shield);
                *shields += usize::from(card.kind == ActionCardKind::Shield);
            }
            crate::action_card::ActionCardEvent::PeekRequested { .. } => *peeks += 1,
            crate::action_card::ActionCardEvent::Redrawn { .. } => *redraws += 1,
            crate::action_card::ActionCardEvent::PeekResolved { .. } => {}
        }
    }
}

pub fn simulate_playoff_greedy(
    player_count: usize,
    seed: u64,
    max_turns: usize,
) -> Result<Vec<usize>, SimulationError> {
    let names = (0..player_count)
        .map(|player| format!("加赛{player}"))
        .collect();
    let mut game = GameState::new_with_players(names, seed, RoundMode::Playoff);
    let mut random = XorShift64::seeded(seed ^ 0xABCD_1234);
    finish_setup_greedily(&mut game)?;
    for turn in 0..max_turns {
        if game.is_finished() {
            return game
                .final_ranking()
                .ok_or_else(|| SimulationError::Invariant {
                    turn,
                    message: "playoff ended without a complete ranking".to_owned(),
                });
        }
        play_simulation_turn(&mut game, &mut random, turn)?;
    }
    Err(SimulationError::MaxTurnsExceeded { seed, max_turns })
}

fn finish_setup_greedily(game: &mut GameState) -> Result<(), SimulationError> {
    for player in 0..game.players.len() {
        while let Some(pair) = game
            .available_pairs(player)
            .map_err(|error| SimulationError::Rule { turn: 0, error })?
            .first()
            .copied()
        {
            game.setup_discard(player, pair)
                .map_err(|error| SimulationError::Rule { turn: 0, error })?;
        }
        game.finish_setup(player)
            .map_err(|error| SimulationError::Rule { turn: 0, error })?;
    }
    Ok(())
}

fn play_simulation_turn(
    game: &mut GameState,
    random: &mut XorShift64,
    turn: usize,
) -> Result<(), SimulationError> {
    if game.pending_peek.is_some() {
        let target = game.pending_peek.expect("pending peek exists").target;
        let use_shield = turn % 3 == 0
            && game.players[target]
                .action_cards
                .iter()
                .any(|card| card.kind == ActionCardKind::Shield);
        game.respond_to_peek(target, use_shield)
            .map_err(|error| SimulationError::Rule { turn, error })?;
        game.take_peek_result(game.current_player)
            .map_err(|error| SimulationError::Rule { turn, error })?;
        return Ok(());
    }
    if game.has_peek_result() {
        game.take_peek_result(game.current_player)
            .map_err(|error| SimulationError::Rule { turn, error })?;
        return Ok(());
    }
    let player = game.current_player;
    match game.turn_phase {
        TurnPhase::AwaitingAction => {
            let has_peek = !game.active_action_used
                && game.players[player]
                    .action_cards
                    .iter()
                    .any(|card| card.kind == ActionCardKind::Peek);
            if has_peek && turn % 5 == 0 {
                if let Some(target) = game.peek_target(player) {
                    if !game.players[target].hand.is_empty() {
                        game.peek(player, 0)
                            .map_err(|error| SimulationError::Rule { turn, error })?;
                        return Ok(());
                    }
                }
            }
            if let Some(pair) = game
                .available_pairs(player)
                .map_err(|error| SimulationError::Rule { turn, error })?
                .first()
                .copied()
            {
                game.discard_pair(player, pair)
                    .map_err(|error| SimulationError::Rule { turn, error })?;
            } else {
                game.draw_from_next(player, random)
                    .map_err(|error| SimulationError::Rule { turn, error })?;
            }
        }
        TurnPhase::ResolvingDraw => {
            if game
                .pending_draw
                .map(|draw| draw.can_redraw)
                .unwrap_or(false)
                && !game.active_action_used
                && turn % 2 == 0
            {
                game.redraw(player, random)
                    .map_err(|error| SimulationError::Rule { turn, error })?;
            } else if let Some(pair) = game
                .available_pairs(player)
                .map_err(|error| SimulationError::Rule { turn, error })?
                .first()
                .copied()
            {
                game.discard_after_draw(player, pair)
                    .map_err(|error| SimulationError::Rule { turn, error })?;
            } else {
                game.keep_drawn_pairs(player)
                    .map_err(|error| SimulationError::Rule { turn, error })?;
            }
        }
    }
    Ok(())
}

fn validate_state(game: &GameState, turn: usize) -> Result<(), SimulationError> {
    let mut ids = HashSet::new();
    let mut total = 0;
    let mut joker_count = 0;

    for card in game
        .players
        .iter()
        .flat_map(|player| player.hand.iter())
        .chain(game.discard_pile.iter())
    {
        if !ids.insert(card.id) {
            return Err(SimulationError::Invariant {
                turn,
                message: format!("duplicate card id {}", card.id),
            });
        }
        total += 1;
        if card.is_joker() {
            joker_count += 1;
            if game.discard_pile.contains(card) {
                return Err(SimulationError::Invariant {
                    turn,
                    message: "joker entered the discard pile".to_owned(),
                });
            }
        }
    }

    if total != 53 || joker_count != 1 {
        return Err(SimulationError::Invariant {
            turn,
            message: format!("expected 53 total cards and 1 joker, got {total} and {joker_count}"),
        });
    }
    if game.mode == RoundMode::Normal
        && matches!(game.phase, GamePhase::Finished { .. })
        && game.total_cards_in_hands() != 1
    {
        return Err(SimulationError::Invariant {
            turn,
            message: "finished game must have exactly one card in players' hands".to_owned(),
        });
    }
    if game.is_finished() && game.final_ranking().is_none() {
        return Err(SimulationError::Invariant {
            turn,
            message: "finished game must have a complete player ranking".to_owned(),
        });
    }
    let mut action_ids = HashSet::new();
    let action_cards = game
        .players
        .iter()
        .flat_map(|player| player.action_cards.iter())
        .chain(game.action_discard_pile.iter())
        .copied()
        .collect::<Vec<_>>();
    if action_cards.len() != ACTION_CARD_COUNT {
        return Err(SimulationError::Invariant {
            turn,
            message: format!(
                "expected {ACTION_CARD_COUNT} action cards, got {}",
                action_cards.len()
            ),
        });
    }
    for card in &action_cards {
        if !action_ids.insert(card.id) {
            return Err(SimulationError::Invariant {
                turn,
                message: format!("duplicate action card id {}", card.id),
            });
        }
    }
    for kind in ActionCardKind::ALL {
        let count = action_cards.iter().filter(|card| card.kind == kind).count();
        if count != ACTION_CARD_COPIES_PER_KIND {
            return Err(SimulationError::Invariant {
                turn,
                message: format!(
                    "expected {ACTION_CARD_COPIES_PER_KIND} action cards of {kind:?}, got {count}"
                ),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn many_fixed_seeds_finish_without_invariant_failures() {
        for seed in 0..64 {
            let report = simulate_greedy(seed, 10_000).unwrap();
            assert!(report.turns < 10_000);
            assert!(report.loser < PLAYER_COUNT);
        }
    }

    #[test]
    fn same_seed_has_same_simulation_result() {
        assert_eq!(simulate_greedy(1234, 10_000), simulate_greedy(1234, 10_000));
    }

    #[test]
    fn action_card_match_simulation_is_deterministic_and_bounded() {
        let first = simulate_match_greedy(1234, 100_000, 100).unwrap();
        let second = simulate_match_greedy(1234, 100_000, 100).unwrap();

        assert_eq!(first, second);
        assert!(first.winner < PLAYER_COUNT);
        assert!(first.turns < 100_000);
    }

    #[test]
    fn one_thousand_match_seeds_preserve_match_invariants() {
        for seed in 0..1_000 {
            let report = simulate_match_greedy(seed, 100_000, 100).unwrap();
            assert!(report.winner < PLAYER_COUNT);
        }
    }

    #[test]
    fn two_to_four_player_playoffs_finish_without_deadlock() {
        for player_count in 2..=4 {
            for seed in 0..100 {
                let ranking = simulate_playoff_greedy(player_count, seed, 100_000).unwrap();
                assert_eq!(ranking.len(), player_count);
                let unique: HashSet<_> = ranking.iter().copied().collect();
                assert_eq!(unique.len(), player_count);
            }
        }
    }
}
