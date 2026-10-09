pub mod action_card;
pub mod card;
pub mod deck;
pub mod game;
pub mod match_state;
pub mod random;
pub mod simulation;

pub use action_card::{
    ACTION_CARD_COPIES_PER_KIND, ACTION_CARD_COUNT, ActionCard, ActionCardEvent, ActionCardKind,
    ActionDeck, PeekResult, PendingPeek,
};
pub use card::{Card, CardKind, Pair, Rank, Suit};
pub use deck::Deck;
pub use game::{
    GameError, GameEvent, GamePhase, GameState, PLAYER_COUNT, PendingDraw, Player, RoundMode,
    TurnPhase, find_pairs,
};
pub use match_state::{
    MATCH_TARGET_SCORE, MatchError, MatchPhase, MatchState, ROUND_POINTS, RoundResult,
};
pub use random::{RandomSource, XorShift64};
pub use simulation::{
    MatchSimulationReport, SimulationError, SimulationReport, simulate_greedy,
    simulate_match_greedy, simulate_playoff_greedy,
};
