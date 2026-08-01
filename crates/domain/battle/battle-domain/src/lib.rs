//! 第三世代风格对战的确定性领域数据模型。
//!
//! 本 crate 建模战斗单位、物种数据、战斗状态、招式、能力值与值对象。
//! 它不访问随机源、文件、网络、窗口或真实时间。
//! 战斗计算逻辑（伤害公式、属性克制、状态结算）由上层规则层基于本层数据实现。
//! 调用方应将 `BattleEvent` 作为展示或持久化的事实记录，不应反向修改领域状态。

#![forbid(unsafe_code)]

mod battle;
mod model;
mod pokemon;
mod rules;
mod stats;

pub use battle::{
    Action, Battle, BattleCommand, BattleError, BattleEvent, BattleOutcome, BattlePhase,
    DamageSource, DeterministicRng, IllegalActionReason, PendingCommand, ReplacementSides,
    SubmitOutcome, UsedMove,
};
pub use model::{
    Ability, Accuracy, BattleStat, BattleStats, EffectTarget, FixedDamage, MAX_MOVES,
    MAX_STAT_STAGE, MIN_STAT_STAGE, MajorStatus, MajorStatusKind, Move, MoveCategory, MoveEffect,
    MoveId, MoveSlot, PokemonId, PokemonType, Side, StageChanges, StatStages, TEAM_SIZE, Team,
    TeamSlot, ValidationError, Weather, WeatherAccuracyModifier, WeatherMoveModifier, WeatherState,
};
pub use pokemon::{
    BattleState, BattleUnit, BattleUnitId, FormId, NationalDexId, Species, VolatileStatus,
    VolatileStatuses,
};
pub use rules::TypeEffectiveness;
pub use stats::{
    CalculatedStats, EffortValues, IndividualValues, MAX_EFFORT_VALUE, MAX_INDIVIDUAL_VALUE,
    MAX_TOTAL_EFFORT_VALUE, Nature, NonHpStat, StatBlock, StatName, StatProjectionError,
    TrainingValues, calculate_gen3_stats,
};

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
