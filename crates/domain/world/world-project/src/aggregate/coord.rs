/// 世界分块坐标，作为世界地图集合的键与布局计算的载体。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldChunkCoord {
    pub(crate) x: i32,
    pub(crate) y: i32,
}

impl WorldChunkCoord {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// 返回沿 x、y 方向偏移后的坐标；结果溢出 `i32` 时返回 `None`。
    pub(crate) const fn offset(self, x: i32, y: i32) -> Option<Self> {
        match (self.x.checked_add(x), self.y.checked_add(y)) {
            (Some(x), Some(y)) => Some(Self::new(x, y)),
            _ => None,
        }
    }
}
