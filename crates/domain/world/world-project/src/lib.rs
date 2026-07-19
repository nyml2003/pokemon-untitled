//! Validated fixed-size map placement and deterministic 3x3 preload windows.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use map_project::{AtomicTileId, MapError, MapProject, MapProjectId};

pub const STANDARD_MAP_WIDTH: u16 = 72;
pub const STANDARD_MAP_HEIGHT: u16 = 56;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorldChunkCoord {
    x: i32,
    y: i32,
}

impl WorldChunkCoord {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub const fn x(self) -> i32 {
        self.x
    }

    pub const fn y(self) -> i32 {
        self.y
    }

    const fn offset(self, x: i32, y: i32) -> Option<Self> {
        match (self.x.checked_add(x), self.y.checked_add(y)) {
            (Some(x), Some(y)) => Some(Self::new(x, y)),
            _ => None,
        }
    }
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreloadSlot<'a> {
    coordinate: WorldChunkCoord,
    project: Option<&'a MapProject>,
}

impl<'a> PreloadSlot<'a> {
    pub const fn coordinate(self) -> WorldChunkCoord {
        self.coordinate
    }

    pub const fn project(self) -> Option<&'a MapProject> {
        self.project
    }

    pub const fn is_empty(self) -> bool {
        self.project.is_none()
    }
}

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

    /// Returns the center map and eight neighbors in stable row-major order.
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorldProjectError {
    EmptyWorld,
    DuplicateCoordinate(WorldChunkCoord),
    DuplicateMapId(MapProjectId),
    InitialMapMissing(WorldChunkCoord),
    WindowOutOfBounds(WorldChunkCoord),
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

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
