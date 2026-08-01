use map_project::MapProject;

use crate::aggregate::WorldChunkCoord;

/// 放置在世界中的一张地图，包含世界坐标与地图工程。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlacedMap {
    pub coordinate: WorldChunkCoord,
    pub project: MapProject,
}

impl PlacedMap {
    pub const fn new(coordinate: WorldChunkCoord, project: MapProject) -> Self {
        Self {
            coordinate,
            project,
        }
    }
}

/// 预载窗口中的一格，包含其世界坐标与可选的地图引用。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreloadSlot<'a> {
    pub coordinate: WorldChunkCoord,
    pub project: Option<&'a MapProject>,
}

impl<'a> PreloadSlot<'a> {
    /// 返回该格是否没有地图。
    pub const fn is_empty(self) -> bool {
        self.project.is_none()
    }
}
