//! 第三世代风格对战的确定性领域数据模型。
//!
//! 本 crate 建模战斗单位、物种数据、战斗状态、招式、能力值与值对象。
//! 它不访问随机源、文件、网络、窗口或真实时间。
//! 战斗计算逻辑（伤害公式、属性克制、状态结算）由上层规则层基于本层数据实现。
//! 调用方应将 `BattleEvent` 作为展示或持久化的事实记录，不应反向修改领域状态。

#![forbid(unsafe_code)]

mod battle;
mod battle_state;
mod battle_unit;
mod enums;
mod error;
mod id;
mod moves;
mod species;
mod stats;
mod team;
mod volatile;

pub use battle::{
    Action, Battle, BattleCommand, BattleError, BattleEvent, BattleOutcome, BattlePhase,
    DamageSource, DeterministicRng, IllegalActionReason, PendingCommand, ReplacementSides,
    SubmitOutcome, UsedMove,
};
pub use battle_state::BattleState;
pub use battle_unit::BattleUnit;
pub use enums::{
    Ability, Accuracy, BattleStat, BattleStats, EffectTarget, FixedDamage, MAX_STAT_STAGE,
    MIN_STAT_STAGE, MajorStatus, MajorStatusKind, MoveCategory, MoveEffect, PokemonType, Side,
    StageChanges, StatStages, TypeEffectiveness, Weather, WeatherAccuracyModifier,
    WeatherMoveModifier, WeatherState,
};
pub use error::ValidationError;
pub use id::{
    BattleUnitId, FormId, MAX_MOVES, MoveId, MoveSlot, NationalDexId, PokemonId, TEAM_SIZE,
    TeamSlot,
};
pub use moves::Move;
pub use species::Species;
pub use stats::{
    CalculatedStats, EffortValues, IndividualValues, MAX_EFFORT_VALUE, MAX_INDIVIDUAL_VALUE,
    MAX_TOTAL_EFFORT_VALUE, Nature, NonHpStat, StatBlock, StatName, StatProjectionError,
    TrainingValues, calculate_gen3_stats,
};
pub use team::Team;
pub use volatile::{VolatileStatus, VolatileStatuses};

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
