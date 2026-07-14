//! Pure player-facing battle lifecycle and semantic playback state.

#![forbid(unsafe_code)]

mod coordinator;
mod reducer;
mod session;

pub use battle_application::{
    Action, BattleError, BattleObservation, MoveCategory, MoveSlot, ObservedBattleOutcome,
    Participant, Pokemon, PokemonId, PokemonType, TEAM_SIZE, TeamSlot, TypeEffectiveness, UsedMove,
};
pub use coordinator::{BattleCoordinator, OpponentPolicy};
pub use reducer::{
    BattleCue, BattleScene, CombatantCondition, CombatantScene, PlaybackStep, ReplayError,
    reduce_transition, scene_from_observation,
};
pub use session::{
    ActionPrompt, BattleInteraction, BattleSession, BattleSessionPhase, BattleSessionSnapshot,
    FinishedPrompt, ReplacementPrompt, SessionError,
};

#[cfg(test)]
mod tests;
