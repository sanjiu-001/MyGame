use std::fmt;

use crate::game::{GameState, PLAYER_COUNT, RoundMode};
use crate::random::{RandomSource, XorShift64};

pub const MATCH_TARGET_SCORE: i32 = 10;
pub const ROUND_POINTS: [i32; PLAYER_COUNT] = [3, 2, 1, -1];

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MatchPhase {
    Playing,
    PlayoffPending { players: Vec<usize> },
    Playoff { players: Vec<usize> },
    Finished { winner: usize },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoundResult {
    pub round: u32,
    pub finish_order: Vec<usize>,
    pub points: [i32; PLAYER_COUNT],
    pub joker_holder: usize,
    /// Compatibility alias for callers that used the V2.0.1 name.
    pub loser: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MatchError {
    RoundNotFinished,
    MatchAlreadyFinished,
    PlayoffPending,
    PlayoffInProgress,
    PlayoffNotPending,
    IncompletePlayoffRanking,
    IncompleteRanking,
}

impl fmt::Display for MatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::RoundNotFinished => "the current round has not finished",
            Self::MatchAlreadyFinished => "the match has already finished",
            Self::PlayoffPending => "the match is waiting for a playoff",
            Self::PlayoffInProgress => "the playoff is still in progress",
            Self::PlayoffNotPending => "there is no playoff waiting to start",
            Self::IncompletePlayoffRanking => "the playoff has an incomplete ranking",
            Self::IncompleteRanking => "the finished round has an incomplete ranking",
        };
        f.write_str(message)
    }
}

impl std::error::Error for MatchError {}

#[derive(Clone, Debug)]
pub struct MatchState {
    names: [String; PLAYER_COUNT],
    scores: [i32; PLAYER_COUNT],
    round: u32,
    game: GameState,
    phase: MatchPhase,
    history: Vec<RoundResult>,
    playoff_ranking: Option<Vec<usize>>,
    random: XorShift64,
}

impl MatchState {
    pub fn new(names: [String; PLAYER_COUNT], seed: u64) -> Self {
        let mut random = XorShift64::seeded(seed);
        let game = GameState::new_with_random(names.clone(), &mut random);
        Self {
            names,
            scores: [0; PLAYER_COUNT],
            round: 1,
            game,
            phase: MatchPhase::Playing,
            history: Vec::new(),
            playoff_ranking: None,
            random,
        }
    }

    pub fn names(&self) -> &[String; PLAYER_COUNT] {
        &self.names
    }

    pub fn scores(&self) -> [i32; PLAYER_COUNT] {
        self.scores
    }

    pub fn round(&self) -> u32 {
        self.round
    }

    pub fn game(&self) -> &GameState {
        &self.game
    }

    pub fn game_mut(&mut self) -> &mut GameState {
        &mut self.game
    }

    pub fn phase(&self) -> &MatchPhase {
        &self.phase
    }

    pub fn history(&self) -> &[RoundResult] {
        &self.history
    }

    pub fn playoff_players(&self) -> Option<&[usize]> {
        match &self.phase {
            MatchPhase::PlayoffPending { players } => Some(players),
            _ => None,
        }
    }

    pub fn winner(&self) -> Option<usize> {
        match self.phase {
            MatchPhase::Finished { winner } => Some(winner),
            _ => None,
        }
    }

    pub fn playoff_ranking(&self) -> Option<&[usize]> {
        self.playoff_ranking.as_deref()
    }

    pub fn start_playoff(&mut self) -> Result<(), MatchError> {
        let players = match &self.phase {
            MatchPhase::PlayoffPending { players } => players.clone(),
            MatchPhase::Playing => return Err(MatchError::PlayoffNotPending),
            MatchPhase::Playoff { .. } => return Err(MatchError::PlayoffInProgress),
            MatchPhase::Finished { .. } => return Err(MatchError::MatchAlreadyFinished),
        };
        let names = players
            .iter()
            .map(|player| self.names[*player].clone())
            .collect();
        let seed = self.random.next_u64();
        self.game = GameState::new_with_players(names, seed, RoundMode::Playoff);
        self.phase = MatchPhase::Playoff { players };
        Ok(())
    }

    pub fn finish_playoff(&mut self) -> Result<&[usize], MatchError> {
        let players = match &self.phase {
            MatchPhase::Playoff { players } => players.clone(),
            MatchPhase::PlayoffPending { .. } => return Err(MatchError::PlayoffPending),
            MatchPhase::Playing => return Err(MatchError::PlayoffNotPending),
            MatchPhase::Finished { .. } => return Err(MatchError::MatchAlreadyFinished),
        };
        if !self.game.is_finished() {
            return Err(MatchError::PlayoffInProgress);
        }
        let ranking = self
            .game
            .final_ranking()
            .ok_or(MatchError::IncompletePlayoffRanking)?;
        let global_ranking: Vec<usize> = ranking.iter().map(|player| players[*player]).collect();
        let winner = global_ranking
            .first()
            .copied()
            .ok_or(MatchError::IncompletePlayoffRanking)?;
        self.playoff_ranking = Some(global_ranking);
        self.phase = MatchPhase::Finished { winner };
        Ok(self
            .playoff_ranking
            .as_deref()
            .expect("ranking was just stored"))
    }

    pub fn settle_round(&mut self) -> Result<&RoundResult, MatchError> {
        match self.phase {
            MatchPhase::Playing => {}
            MatchPhase::PlayoffPending { .. } => return Err(MatchError::PlayoffPending),
            MatchPhase::Playoff { .. } => return Err(MatchError::PlayoffInProgress),
            MatchPhase::Finished { .. } => return Err(MatchError::MatchAlreadyFinished),
        }
        if !self.game.is_finished() {
            return Err(MatchError::RoundNotFinished);
        }

        let finish_order = self
            .game
            .final_ranking()
            .ok_or(MatchError::IncompleteRanking)?;
        let loser = *finish_order.last().ok_or(MatchError::IncompleteRanking)?;
        let mut points = [0; PLAYER_COUNT];
        for (rank, player) in finish_order.iter().copied().enumerate() {
            points[player] = ROUND_POINTS
                .get(rank)
                .copied()
                .ok_or(MatchError::IncompleteRanking)?;
        }
        for player in 0..PLAYER_COUNT {
            self.scores[player] += points[player];
        }

        let result = RoundResult {
            round: self.round,
            finish_order,
            points,
            joker_holder: loser,
            loser,
        };
        self.history.push(result);

        let reached_target: Vec<usize> = self
            .scores
            .iter()
            .enumerate()
            .filter_map(|(player, score)| (*score >= MATCH_TARGET_SCORE).then_some(player))
            .collect();
        match reached_target.as_slice() {
            [winner] => self.phase = MatchPhase::Finished { winner: *winner },
            [] => {
                self.round += 1;
                self.game = GameState::new_with_random(self.names.clone(), &mut self.random);
            }
            players => {
                self.phase = MatchPhase::PlayoffPending {
                    players: players.to_vec(),
                };
            }
        }

        Ok(self.history.last().expect("round result was just recorded"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::Card;
    use crate::game::{GamePhase, RoundMode};

    fn names() -> [String; PLAYER_COUNT] {
        ["甲", "乙", "丙", "丁"].map(str::to_owned)
    }

    fn finished_game(order: [usize; PLAYER_COUNT]) -> GameState {
        let mut game = GameState::new(names(), 1);
        for player in &mut game.players {
            player.hand.clear();
        }
        game.players[*order.last().unwrap()]
            .hand
            .push(Card::joker(52));
        game.finish_order = order.to_vec();
        game.phase = GamePhase::Finished {
            loser: *order.last().unwrap(),
        };
        game
    }

    fn finished_playoff_game(player_count: usize, order: &[usize]) -> GameState {
        let names = (0..player_count)
            .map(|player| format!("加赛{player}"))
            .collect();
        let mut game = GameState::new_with_players(names, 99, RoundMode::Playoff);
        for player in &mut game.players {
            player.hand.clear();
        }
        game.players[*order.last().unwrap()]
            .hand
            .push(Card::joker(52));
        game.finish_order = order.to_vec();
        game.phase = GamePhase::Finished {
            loser: *order.last().unwrap(),
        };
        game
    }

    #[test]
    fn round_points_are_three_two_one_and_minus_one() {
        let mut state = MatchState::new(names(), 1);
        state.game = finished_game([2, 0, 3, 1]);

        let result = state.settle_round().unwrap().clone();

        assert_eq!(result.points, [2, -1, 3, 1]);
        assert_eq!(state.scores(), [2, -1, 3, 1]);
        assert_eq!(result.loser, 1);
    }

    #[test]
    fn negative_scores_are_preserved_and_next_round_is_reset() {
        let mut state = MatchState::new(names(), 22);
        state.game = finished_game([0, 1, 2, 3]);
        state.settle_round().unwrap();

        assert_eq!(state.scores(), [3, 2, 1, -1]);
        assert_eq!(state.round(), 2);
        assert_eq!(state.game().phase, GamePhase::Setup);
        assert_eq!(state.game().discard_pile.len(), 0);
        assert_eq!(state.game().setup_player, 0);
        assert_eq!(state.game().pending_draw_player, None);
        assert_eq!(state.game().finish_order(), &[]);
        assert_eq!(state.game().total_action_cards(), 6);
        assert!(state.game().action_discard_pile.is_empty());
    }

    #[test]
    fn same_seed_reproduces_each_new_round() {
        let mut first = MatchState::new(names(), 23);
        let mut second = MatchState::new(names(), 23);
        assert_eq!(first.game().players, second.game().players);

        first.game = finished_game([0, 1, 2, 3]);
        second.game = finished_game([0, 1, 2, 3]);
        first.settle_round().unwrap();
        second.settle_round().unwrap();

        assert_eq!(first.game().players, second.game().players);
    }

    #[test]
    fn scores_accumulate_across_rounds() {
        let mut state = MatchState::new(names(), 27);
        state.game = finished_game([0, 1, 2, 3]);
        state.settle_round().unwrap();
        state.game = finished_game([0, 1, 2, 3]);

        state.settle_round().unwrap();

        assert_eq!(state.scores(), [6, 4, 2, -2]);
        assert_eq!(state.history().len(), 2);
        assert_eq!(state.round(), 3);
    }

    #[test]
    fn one_player_reaching_target_finishes_match() {
        let mut state = MatchState::new(names(), 24);
        state.scores = [7, 0, 0, 0];
        state.game = finished_game([0, 1, 2, 3]);

        state.settle_round().unwrap();

        assert_eq!(state.phase(), &MatchPhase::Finished { winner: 0 });
        assert_eq!(state.winner(), Some(0));
        assert_eq!(state.settle_round(), Err(MatchError::MatchAlreadyFinished));
    }

    #[test]
    fn multiple_players_reaching_target_wait_for_playoff() {
        let mut state = MatchState::new(names(), 25);
        state.scores = [7, 8, 0, 0];
        state.game = finished_game([0, 1, 2, 3]);

        state.settle_round().unwrap();

        assert_eq!(
            state.phase(),
            &MatchPhase::PlayoffPending {
                players: vec![0, 1]
            }
        );
        assert_eq!(state.playoff_players(), Some(&[0, 1][..]));
        assert_eq!(state.round(), 1);
    }

    #[test]
    fn cannot_settle_an_unfinished_round_or_twice() {
        let mut state = MatchState::new(names(), 26);
        assert_eq!(state.settle_round(), Err(MatchError::RoundNotFinished));

        state.game = finished_game([0, 1, 2, 3]);
        state.settle_round().unwrap();
        assert_eq!(state.settle_round(), Err(MatchError::RoundNotFinished));

        state.scores = [7, 8, 0, 0];
        state.game = finished_game([0, 1, 2, 3]);
        state.settle_round().unwrap();
        assert_eq!(state.settle_round(), Err(MatchError::PlayoffPending));
    }

    #[test]
    fn playoff_uses_only_qualifying_players_and_does_not_change_scores() {
        let mut state = MatchState::new(names(), 31);
        state.scores = [10, 12, 4, 2];
        state.game = finished_game([0, 1, 2, 3]);
        state.phase = MatchPhase::PlayoffPending {
            players: vec![0, 1],
        };
        let before = state.scores();

        state.start_playoff().unwrap();

        assert_eq!(
            state.phase(),
            &MatchPhase::Playoff {
                players: vec![0, 1]
            }
        );
        assert_eq!(state.game().players.len(), 2);
        assert_eq!(state.game().players[0].name, "甲");
        assert_eq!(state.game().players[1].name, "乙");
        assert_eq!(state.game().mode, RoundMode::Playoff);
        assert_eq!(state.scores(), before);
    }

    #[test]
    fn finishing_playoff_records_global_ranking_without_scoring() {
        let mut state = MatchState::new(names(), 32);
        state.scores = [10, 11, 0, 0];
        state.phase = MatchPhase::Playoff {
            players: vec![0, 1],
        };
        state.game = finished_playoff_game(2, &[1, 0]);

        let ranking = state.finish_playoff().unwrap().to_vec();

        assert_eq!(ranking, vec![1, 0]);
        assert_eq!(state.winner(), Some(1));
        assert_eq!(state.playoff_ranking(), Some(&[1, 0][..]));
        assert_eq!(state.scores(), [10, 11, 0, 0]);
    }

    #[test]
    fn playoff_phase_rejects_normal_round_settlement() {
        let mut state = MatchState::new(names(), 33);
        state.phase = MatchPhase::Playoff {
            players: vec![0, 1, 2],
        };
        state.game = finished_playoff_game(3, &[0, 1, 2]);

        assert_eq!(state.settle_round(), Err(MatchError::PlayoffInProgress));
        assert_eq!(state.start_playoff(), Err(MatchError::PlayoffInProgress));
    }
}
