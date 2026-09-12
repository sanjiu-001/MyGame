use std::collections::HashSet;
use std::fmt;

use crate::game::{GameError, GamePhase, GameState, PLAYER_COUNT, TurnPhase};
use crate::random::XorShift64;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimulationReport {
    pub seed: u64,
    pub turns: usize,
    pub loser: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimulationError {
    Rule { turn: usize, error: GameError },
    Invariant { turn: usize, message: String },
    MaxTurnsExceeded { seed: u64, max_turns: usize },
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
    if matches!(game.phase, GamePhase::Finished { .. }) && game.total_cards_in_hands() != 1 {
        return Err(SimulationError::Invariant {
            turn,
            message: "finished game must have exactly one card in players' hands".to_owned(),
        });
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
}
