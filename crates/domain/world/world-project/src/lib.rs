//! 已验证的固定尺寸地图放置与确定性的 3x3 预载窗口。
//!
//! 本 crate 只建模世界地图的坐标放置、预载窗口与合成布局。
//! 它不访问文件、网络、窗口或真实时间，瓦片数据由 `map-project` 校验传入。

#![forbid(unsafe_code)]

mod aggregate;

pub const STANDARD_MAP_WIDTH: u16 = 72;
pub const STANDARD_MAP_HEIGHT: u16 = 56;

pub use aggregate::{PlacedMap, PreloadSlot, WorldChunkCoord, WorldProject, WorldProjectError};

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
