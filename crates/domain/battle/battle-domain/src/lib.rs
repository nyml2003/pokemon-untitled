//! 第三世代风格对战的确定性领域规则。
//!
//! 本 crate 只建模队伍、宝可梦、招式和回合状态。
//! 它不访问随机源、文件、网络、窗口或真实时间。
//! 调用方传入同一队伍和种子时，`Battle` 会产生相同的状态和事件序列。
//!
//! `Battle::submit` 原子地接受一方命令。
//! 当双方命令齐备时，它会结算整个回合或强制替换，并通过 `SubmitOutcome` 返回新增事件。
//! 调用方应将 `BattleEvent` 作为展示或持久化的事实记录，不应反向修改领域状态。

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
    Ability, Accuracy, BattleStat, BattleStats, EffectTarget, FixedDamage, MAX_MOVES,
    MAX_STAT_STAGE, MIN_STAT_STAGE, MajorStatus, MajorStatusKind, Move, MoveCategory, MoveEffect,
    MoveId, MoveSlot, Pokemon, PokemonId, PokemonType, Side, StageChanges, StatStages, TEAM_SIZE,
    Team, TeamSlot, ValidationError, Weather, WeatherAccuracyModifier, WeatherMoveModifier,
    WeatherState,
};
pub use rules::{DamageCategory, TypeEffectiveness, damage_category, type_effectiveness};
pub use stats::{
    CalculatedStats, EffortValues, IndividualValues, MAX_EFFORT_VALUE, MAX_INDIVIDUAL_VALUE,
    MAX_TOTAL_EFFORT_VALUE, Nature, NonHpStat, StatBlock, StatName, StatProjectionError,
    TrainingValues, calculate_gen3_stats,
};

#[cfg(test)]
mod tests;
