use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use map_project::{AtomicTileId, MapError, MapProject, MapProjectId};

use crate::aggregate::{PlacedMap, PreloadSlot, WorldChunkCoord};
use crate::{STANDARD_MAP_HEIGHT, STANDARD_MAP_WIDTH};

/// 已验证的世界地图聚合，包含初始地图与按坐标索引的地图集合。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldProject {
    initial: WorldChunkCoord,
    maps: BTreeMap<WorldChunkCoord, MapProject>,
}

impl WorldProject {
    pub fn new(
        initial: WorldChunkCoord,
        placed_maps: impl IntoIterator<Item = PlacedMap>,
        known_tiles: &BTreeSet<AtomicTileId>,
    ) -> Result<Self, WorldProjectError> {
        let mut map_ids = BTreeSet::new();
        let mut maps = BTreeMap::new();
        for placed in placed_maps {
            if maps.contains_key(&placed.coordinate) {
                return Err(WorldProjectError::DuplicateCoordinate(placed.coordinate));
            }
            if !map_ids.insert(placed.project.id.clone()) {
                return Err(WorldProjectError::DuplicateMapId(placed.project.id));
            }
            if (placed.project.width, placed.project.height)
                != (STANDARD_MAP_WIDTH, STANDARD_MAP_HEIGHT)
            {
                return Err(WorldProjectError::UnexpectedMapSize {
                    map: placed.project.id,
                    width: placed.project.width,
                    height: placed.project.height,
                });
            }
            placed.project.validate(known_tiles)?;
            maps.insert(placed.coordinate, placed.project);
        }
        if maps.is_empty() {
            return Err(WorldProjectError::EmptyWorld);
        }
        if !maps.contains_key(&initial) {
            return Err(WorldProjectError::InitialMapMissing(initial));
        }
        Ok(Self { initial, maps })
    }

    pub const fn initial(&self) -> WorldChunkCoord {
        self.initial
    }

    pub fn map_at(&self, coordinate: WorldChunkCoord) -> Option<&MapProject> {
        self.maps.get(&coordinate)
    }

    pub fn maps(&self) -> impl Iterator<Item = (WorldChunkCoord, &MapProject)> {
        self.maps
            .iter()
            .map(|(coordinate, project)| (*coordinate, project))
    }

    /// 返回把全部地图合成一张大地图所需的总宽高；坐标跨度超出 `u16` 时返回 `LayoutOverflow`。
    pub fn size(&self) -> Result<(u16, u16), WorldProjectError> {
        let (minimum, maximum) = self.bounds().ok_or(WorldProjectError::EmptyWorld)?;
        let width = checked_extent(minimum.x, maximum.x, STANDARD_MAP_WIDTH)
            .ok_or(WorldProjectError::LayoutOverflow)?;
        let height = checked_extent(minimum.y, maximum.y, STANDARD_MAP_HEIGHT)
            .ok_or(WorldProjectError::LayoutOverflow)?;
        Ok((width, height))
    }

    /// 返回指定地图在合成大地图中的像素起点；坐标跨度超出 `u16` 时返回 `LayoutOverflow`。
    pub fn origin_of(&self, coordinate: WorldChunkCoord) -> Result<(u16, u16), WorldProjectError> {
        let (minimum, _) = self.bounds().ok_or(WorldProjectError::EmptyWorld)?;
        let origin_x = checked_origin(coordinate.x, minimum.x, STANDARD_MAP_WIDTH)
            .ok_or(WorldProjectError::LayoutOverflow)?;
        let origin_y = checked_origin(coordinate.y, minimum.y, STANDARD_MAP_HEIGHT)
            .ok_or(WorldProjectError::LayoutOverflow)?;
        Ok((origin_x, origin_y))
    }

    /// 世界内所有地图坐标的最小与最大值。
    fn bounds(&self) -> Option<(WorldChunkCoord, WorldChunkCoord)> {
        let first = self.maps.keys().next().copied()?;
        let (mut min_x, mut min_y, mut max_x, mut max_y) = (first.x, first.y, first.x, first.y);
        for &coordinate in self.maps.keys() {
            min_x = min_x.min(coordinate.x);
            min_y = min_y.min(coordinate.y);
            max_x = max_x.max(coordinate.x);
            max_y = max_y.max(coordinate.y);
        }
        Some((
            WorldChunkCoord::new(min_x, min_y),
            WorldChunkCoord::new(max_x, max_y),
        ))
    }

    /// 以稳定的行优先顺序返回中心地图与八个相邻地图。
    pub fn preload_window(
        &self,
        center: WorldChunkCoord,
    ) -> Result<[PreloadSlot<'_>; 9], WorldProjectError> {
        Ok([
            self.preload_slot(center, -1, -1)?,
            self.preload_slot(center, 0, -1)?,
            self.preload_slot(center, 1, -1)?,
            self.preload_slot(center, -1, 0)?,
            self.preload_slot(center, 0, 0)?,
            self.preload_slot(center, 1, 0)?,
            self.preload_slot(center, -1, 1)?,
            self.preload_slot(center, 0, 1)?,
            self.preload_slot(center, 1, 1)?,
        ])
    }

    fn preload_slot(
        &self,
        center: WorldChunkCoord,
        x: i32,
        y: i32,
    ) -> Result<PreloadSlot<'_>, WorldProjectError> {
        let coordinate = center
            .offset(x, y)
            .ok_or(WorldProjectError::WindowOutOfBounds(center))?;
        Ok(PreloadSlot {
            coordinate,
            project: self.map_at(coordinate),
        })
    }
}

fn checked_extent(minimum: i32, maximum: i32, map_extent: u16) -> Option<u16> {
    let map_count = i64::from(maximum) - i64::from(minimum) + 1;
    u16::try_from(map_count * i64::from(map_extent)).ok()
}

fn checked_origin(coordinate: i32, minimum: i32, map_extent: u16) -> Option<u16> {
    u16::try_from((i64::from(coordinate) - i64::from(minimum)) * i64::from(map_extent)).ok()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorldProjectError {
    EmptyWorld,
    DuplicateCoordinate(WorldChunkCoord),
    DuplicateMapId(MapProjectId),
    InitialMapMissing(WorldChunkCoord),
    WindowOutOfBounds(WorldChunkCoord),
    LayoutOverflow,
    UnexpectedMapSize {
        map: MapProjectId,
        width: u16,
        height: u16,
    },
    Map(MapError),
}

impl From<MapError> for WorldProjectError {
    fn from(error: MapError) -> Self {
        Self::Map(error)
    }
}

impl fmt::Display for WorldProjectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyWorld => formatter.write_str("world must contain at least one map"),
            Self::DuplicateCoordinate(coordinate) => {
                write!(formatter, "duplicate map coordinate {coordinate:?}")
            }
            Self::DuplicateMapId(id) => write!(formatter, "duplicate map id {id}"),
            Self::InitialMapMissing(coordinate) => {
                write!(formatter, "initial map at {coordinate:?} is missing")
            }
            Self::WindowOutOfBounds(coordinate) => {
                write!(
                    formatter,
                    "preload window for {coordinate:?} is out of bounds"
                )
            }
            Self::LayoutOverflow => {
                formatter.write_str("world map layout exceeds u16 pixel coordinates")
            }
            Self::UnexpectedMapSize { map, width, height } => {
                write!(formatter, "map {map} has unexpected size {width}x{height}")
            }
            Self::Map(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for WorldProjectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Map(error) => Some(error),
            _ => None,
        }
    }
}
