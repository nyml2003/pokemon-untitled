use crate::battle_state::BattleState;
use crate::enums::{Ability, BattleStats, MajorStatus, PokemonType, StatStages};
use crate::error::ValidationError;
use crate::id::BattleUnitId;
use crate::moves::Move;
use crate::species::Species;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BattleUnit {
    /// 注入的物种数据，只读。
    pub species: Species,
    /// 个体标识。
    pub id: BattleUnitId,
    /// 可变战斗状态。
    pub state: BattleState,
}

impl BattleUnit {
    /// 创建战斗单位。
    pub fn new(
        species: Species,
        id: BattleUnitId,
        state: BattleState,
    ) -> Result<Self, ValidationError> {
        Ok(Self { species, id, state })
    }

    /// 个体标识。
    pub fn id(&self) -> &BattleUnitId {
        &self.id
    }

    /// 注入的物种数据。
    pub fn species(&self) -> &Species {
        &self.species
    }

    /// 战斗状态。
    pub fn state(&self) -> &BattleState {
        &self.state
    }

    /// 物种显示名。
    pub fn name(&self) -> &str {
        self.species.name()
    }

    /// 属性列表。
    pub fn types(&self) -> &[PokemonType] {
        self.species.types()
    }

    /// 当前等级。
    pub const fn level(&self) -> u8 {
        self.state.level()
    }

    /// 当前生效的特性。
    pub fn ability(&self) -> &[Ability] {
        self.state.ability()
    }

    /// 最终战斗能力值。
    pub const fn stats(&self) -> BattleStats {
        self.state.stats()
    }

    /// 最大 HP。
    pub const fn max_hp(&self) -> u32 {
        self.state.max_hp()
    }

    /// 当前 HP。
    pub const fn current_hp(&self) -> u32 {
        self.state.current_hp()
    }

    /// 已携带招式。
    pub fn moves(&self) -> &[Move] {
        self.state.moves()
    }

    /// 主要异常状态。
    pub const fn major_status(&self) -> Option<MajorStatus> {
        self.state.major_status()
    }

    /// 能力阶级。
    pub const fn stages(&self) -> StatStages {
        self.state.stages()
    }

    /// 是否已经倒下。
    pub const fn is_fainted(&self) -> bool {
        self.state.is_fainted()
    }
}
