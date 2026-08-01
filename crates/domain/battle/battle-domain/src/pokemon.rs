use std::collections::HashMap;

use crate::model::{
    Ability, BattleStats, MAX_MOVES, MajorStatus, Move, PokemonType, StatStages, ValidationError,
};
use crate::stats::StatBlock;

/// 战斗单位的唯一标识，区分同种族的个体。
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BattleUnitId(String);

impl BattleUnitId {
    pub fn new(value: impl Into<String>) -> Result<Self, ValidationError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(ValidationError::EmptyPokemonId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 全国图鉴编号。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NationalDexId(u16);

impl NationalDexId {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
}

/// 形态/变体编号。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FormId(u32);

impl FormId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
}

/// 注入的物种数据，不可变，由装配层从图鉴/种族数据转换后拷贝注入。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Species {
    /// 物种显示名。
    pub name: String,
    /// 六维种族值：HP、攻击、防御、特攻、特防、速度。
    pub base_stats: StatBlock<u16>,
    /// 全国图鉴编号。
    pub national_dex_id: NationalDexId,
    /// 变体编号。
    pub form_id: FormId,
    /// 属性列表。
    pub types: Vec<PokemonType>,
    /// 默认特性列表。
    pub default_abilities: Vec<Ability>,
}

impl Species {
    pub fn new(
        name: impl Into<String>,
        base_stats: StatBlock<u16>,
        national_dex_id: NationalDexId,
        form_id: FormId,
        types: Vec<PokemonType>,
        default_abilities: Vec<Ability>,
    ) -> Result<Self, ValidationError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(ValidationError::EmptyPokemonName);
        }
        if types.is_empty() {
            return Err(ValidationError::EmptySpeciesType);
        }
        Ok(Self {
            name,
            base_stats,
            national_dex_id,
            form_id,
            types,
            default_abilities,
        })
    }

    /// 物种显示名。
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 六维种族值。
    pub const fn base_stats(&self) -> StatBlock<u16> {
        self.base_stats
    }

    /// 全国图鉴编号。
    pub const fn national_dex_id(&self) -> NationalDexId {
        self.national_dex_id
    }

    /// 形态/变体编号。
    pub const fn form_id(&self) -> FormId {
        self.form_id
    }

    /// 属性列表。
    pub fn types(&self) -> &[PokemonType] {
        &self.types
    }

    /// 默认特性列表。
    pub fn default_abilities(&self) -> &[Ability] {
        &self.default_abilities
    }
}

/// 一种临时状态种类，作为容器的键。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum VolatileStatus {
    /// 替身。
    Substitute,
    /// 连续守住。
    ProtectStreak,
}

/// 通用临时状态容器，按种类索引；状态只在生效时存在。
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct VolatileStatuses(HashMap<VolatileStatus, u32>);

impl VolatileStatuses {
    /// 返回指定临时状态的值，未生效时返回 `None`。
    pub fn get(&self, status: VolatileStatus) -> Option<u32> {
        self.0.get(&status).copied()
    }

    /// 设置指定临时状态的值。
    pub fn set(&mut self, status: VolatileStatus, value: u32) {
        self.0.insert(status, value);
    }

    /// 清除指定临时状态。
    pub fn remove(&mut self, status: VolatileStatus) {
        self.0.remove(&status);
    }

    /// 是否没有任何临时状态生效。
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// 战斗中一切可能变化的数据。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BattleState {
    /// 当前等级。
    pub level: u8,
    /// 最终战斗能力值。
    pub stats: BattleStats,
    /// 最大 HP。
    pub max_hp: u32,
    /// 当前 HP。
    pub current_hp: u32,
    /// 已携带招式（含剩余 PP）。
    pub moves: Vec<Move>,
    /// 当前生效的特性。
    pub ability: Vec<Ability>,
    /// 主要异常状态（含剩余回合）。
    pub major_status: Option<MajorStatus>,
    /// 能力阶级。
    pub stages: StatStages,
    /// 临时状态容器。
    pub volatile_statuses: VolatileStatuses,
}

impl BattleState {
    /// 创建战斗状态。
    ///
    /// 等级必须在 1 至 100 之间，当前 HP 不得超过最大 HP，且招式列表必须含一至四个不同标识的招式。
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        level: u8,
        stats: BattleStats,
        max_hp: u32,
        current_hp: u32,
        moves: Vec<Move>,
        ability: Vec<Ability>,
        major_status: Option<MajorStatus>,
        stages: StatStages,
    ) -> Result<Self, ValidationError> {
        if !(1..=100).contains(&level) {
            return Err(ValidationError::InvalidLevel { level });
        }
        if max_hp == 0 {
            return Err(ValidationError::ZeroMaxHp);
        }
        if current_hp > max_hp {
            return Err(ValidationError::CurrentHpExceedsMax {
                current: current_hp,
                max: max_hp,
            });
        }
        if moves.is_empty() || moves.len() > MAX_MOVES {
            return Err(ValidationError::InvalidMoveCount { count: moves.len() });
        }
        for left in 0..moves.len() {
            for right in (left + 1)..moves.len() {
                if moves[left].id() == moves[right].id() {
                    return Err(ValidationError::DuplicateMoveId {
                        id: moves[left].id().clone(),
                    });
                }
            }
        }
        Ok(Self {
            level,
            stats,
            max_hp,
            current_hp,
            moves,
            ability,
            major_status,
            stages,
            volatile_statuses: VolatileStatuses::default(),
        })
    }

    /// 当前等级。
    pub const fn level(&self) -> u8 {
        self.level
    }

    /// 最终战斗能力值。
    pub const fn stats(&self) -> BattleStats {
        self.stats
    }

    /// 最大 HP。
    pub const fn max_hp(&self) -> u32 {
        self.max_hp
    }

    /// 当前 HP。
    pub const fn current_hp(&self) -> u32 {
        self.current_hp
    }

    /// 已携带招式（含剩余 PP）。
    pub fn moves(&self) -> &[Move] {
        &self.moves
    }

    /// 当前生效的特性。
    pub fn ability(&self) -> &[Ability] {
        &self.ability
    }

    /// 主要异常状态（含剩余回合）。
    pub const fn major_status(&self) -> Option<MajorStatus> {
        self.major_status
    }

    /// 能力阶级。
    pub const fn stages(&self) -> StatStages {
        self.stages
    }

    /// 临时状态容器。
    pub fn volatile_statuses(&self) -> &VolatileStatuses {
        &self.volatile_statuses
    }

    /// 是否已经倒下。
    pub const fn is_fainted(&self) -> bool {
        self.current_hp == 0
    }
}

/// 一只进入战斗的宝可梦：注入的物种数据 + 个体标识 + 可变状态。
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
