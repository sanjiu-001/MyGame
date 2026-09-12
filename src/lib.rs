pub mod card;
pub mod deck;
pub mod game;
pub mod random;
pub mod simulation;

pub use card::{Card, CardKind, Pair, Rank, Suit};
pub use deck::Deck;
pub use game::{
    GameError, GameEvent, GamePhase, GameState, PLAYER_COUNT, Player, TurnPhase, find_pairs,
};
pub use random::{RandomSource, XorShift64};
pub use simulation::{SimulationError, SimulationReport, simulate_greedy};
