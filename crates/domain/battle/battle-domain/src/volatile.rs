use std::collections::HashMap;

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
