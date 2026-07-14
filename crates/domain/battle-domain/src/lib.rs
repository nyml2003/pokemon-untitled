//! Deterministic, platform-independent rules for a generation-three-style battle.

#![forbid(unsafe_code)]

mod battle;
mod model;
mod rules;
mod stats;

pub use battle::{
    Action, Battle, BattleCommand, BattleError, BattleEvent, BattleOutcome, BattlePhase,
    DamageSource, IllegalActionReason, ReplacementSides, SubmitOutcome, UsedMove,
};
pub use model::{
    Accuracy, BattleStats, MAX_MOVES, Move, MoveCategory, MoveId, MoveSlot, Pokemon, PokemonId,
    PokemonType, Side, TEAM_SIZE, Team, TeamSlot, ValidationError,
};
pub use rules::{DamageCategory, TypeEffectiveness, damage_category, type_effectiveness};
pub use stats::{
    CalculatedStats, EffortValues, IndividualValues, MAX_EFFORT_VALUE, MAX_INDIVIDUAL_VALUE,
    MAX_TOTAL_EFFORT_VALUE, Nature, NonHpStat, StatBlock, StatName, StatProjectionError,
    TrainingValues, calculate_gen3_stats,
};

#[cfg(test)]
mod tests;
