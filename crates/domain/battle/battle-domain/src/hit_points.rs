use crate::error::ValidationError;

/// 血条所处阶段，决定展示与特效。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HitPointsPhase {
    /// 满血。
    Full,
    /// 大于一半。
    High,
    /// 介于 21% 与 50% 之间（黄血）。
    Mid,
    /// 不大于 20%（红血）。
    Low,
    /// 已倒下。
    Zero,
}

/// 血条聚合根，封装当前 HP、最大 HP 与锁定状态。
///
/// 字段全部私有，任何变化只能通过方法产生；`damage`/`heal` 返回扣除或回复的实际值。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HitPoints {
    current: u32,
    max: u32,
    locked: bool,
}

impl HitPoints {
    /// 创建血条；当前 HP 不得超过最大 HP，最大 HP 必须为正。
    pub fn new(current: u32, max: u32) -> Result<Self, ValidationError> {
        if max == 0 {
            return Err(ValidationError::ZeroMaxHp);
        }
        if current > max {
            return Err(ValidationError::CurrentHpExceedsMax { current, max });
        }
        Ok(Self {
            current,
            max,
            locked: false,
        })
    }

    /// 最大 HP。
    pub const fn max(self) -> u32 {
        self.max
    }

    /// 从不校验失败的角度构造血条：把当前 HP 夹在 `[0, max]`，最大 HP 至少为 1。
    ///
    /// 用于从外部可信数据构建展示用的血条，不改变领域内的校验入口 [`HitPoints::new`]。
    pub const fn clamped(current: u32, max: u32) -> Self {
        let max = if max == 0 { 1 } else { max };
        let current = if current > max { max } else { current };
        Self {
            current,
            max,
            locked: false,
        }
    }

    /// 当前 HP。
    pub const fn current(self) -> u32 {
        self.current
    }

    /// 是否已倒下。
    pub const fn is_zero(self) -> bool {
        self.current == 0
    }

    /// 是否被锁定为目标。
    pub const fn is_locked(self) -> bool {
        self.locked
    }

    /// 标记为被锁定，返回新血条。
    pub const fn lock(self) -> Self {
        Self {
            locked: true,
            ..self
        }
    }

    /// 解除锁定，返回新血条。
    pub const fn unlock(self) -> Self {
        Self {
            locked: false,
            ..self
        }
    }

    /// 当前 HP 占最大 HP 的整数百分比，封顶 100。
    pub fn percent(self) -> u8 {
        (u64::from(self.current).saturating_mul(100) / u64::from(self.max.max(1))).min(100) as u8
    }

    /// 当前所处阶段。
    pub const fn phase(self) -> HitPointsPhase {
        if self.current == 0 {
            HitPointsPhase::Zero
        } else if self.current == self.max {
            HitPointsPhase::Full
        } else if self.current.saturating_mul(2) > self.max {
            HitPointsPhase::High
        } else if self.current.saturating_mul(5) > self.max {
            HitPointsPhase::Mid
        } else {
            HitPointsPhase::Low
        }
    }

    /// 扣除伤害，返回新血条与实际扣除量；伤害不会把 HP 扣到零以下。
    pub fn damage(self, amount: u32) -> (Self, u32) {
        let actual = amount.min(self.current);
        (
            Self {
                current: self.current - actual,
                ..self
            },
            actual,
        )
    }

    /// 回复 HP，返回新血条与实际回复量；回复不会超过最大 HP。
    pub fn heal(self, amount: u32) -> (Self, u32) {
        let room = self.max - self.current;
        let actual = amount.min(room);
        (
            Self {
                current: self.current + actual,
                ..self
            },
            actual,
        )
    }
}
