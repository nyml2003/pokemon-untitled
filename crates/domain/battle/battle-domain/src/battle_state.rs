use crate::enums::{Ability, BattleStats, MajorStatus, StatStages};
use crate::error::ValidationError;
use crate::id::MAX_MOVES;
use crate::moves::Move;
use crate::volatile::VolatileStatuses;

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
