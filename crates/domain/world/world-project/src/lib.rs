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
mod tests {
    use std::collections::BTreeSet;

    use map_project::{AtomicTileId, CompositeTile, CompositeTileId, MapProject, MapProjectId};

    use super::{
        PlacedMap, STANDARD_MAP_HEIGHT, STANDARD_MAP_WIDTH, WorldChunkCoord, WorldProject,
        WorldProjectError,
    };

    fn known_tiles() -> BTreeSet<AtomicTileId> {
        BTreeSet::from([AtomicTileId::new("tile-0001").unwrap()])
    }

    fn map(id: &str) -> MapProject {
        let tile = AtomicTileId::new("tile-0001").unwrap();
        MapProject::blank(
            MapProjectId::new(id).unwrap(),
            STANDARD_MAP_WIDTH,
            STANDARD_MAP_HEIGHT,
            Some(CompositeTile::new(
                CompositeTileId::new("ground").unwrap(),
                vec![tile],
            )),
        )
    }

    fn placed(x: i32, y: i32, id: &str) -> PlacedMap {
        PlacedMap::new(WorldChunkCoord::new(x, y), map(id))
    }

    #[test]
    fn rejects_empty_world_and_missing_initial_map() {
        let known = known_tiles();
        assert_eq!(
            WorldProject::new(WorldChunkCoord::new(0, 0), [], &known),
            Err(WorldProjectError::EmptyWorld)
        );
        assert_eq!(
            WorldProject::new(WorldChunkCoord::new(1, 0), [placed(0, 0, "a")], &known),
            Err(WorldProjectError::InitialMapMissing(WorldChunkCoord::new(
                1, 0
            )))
        );
    }

    #[test]
    fn rejects_duplicate_coordinates_and_map_ids() {
        let known = known_tiles();
        assert_eq!(
            WorldProject::new(
                WorldChunkCoord::new(0, 0),
                [placed(0, 0, "a"), placed(0, 0, "b")],
                &known,
            ),
            Err(WorldProjectError::DuplicateCoordinate(
                WorldChunkCoord::new(0, 0)
            ))
        );
        assert_eq!(
            WorldProject::new(
                WorldChunkCoord::new(0, 0),
                [placed(0, 0, "a"), placed(1, 0, "a")],
                &known,
            ),
            Err(WorldProjectError::DuplicateMapId(
                MapProjectId::new("a").unwrap()
            ))
        );
    }

    #[test]
    fn rejects_invalid_and_non_standard_maps() {
        let known = known_tiles();
        let mut invalid = map("invalid");
        invalid.format_version = "unsupported".into();
        assert!(matches!(
            WorldProject::new(
                WorldChunkCoord::new(0, 0),
                [PlacedMap::new(WorldChunkCoord::new(0, 0), invalid)],
                &known,
            ),
            Err(WorldProjectError::Map(_))
        ));

        let mut small = map("small");
        small.width -= 1;
        small.visual_cells.pop();
        small.collision_cells.pop();
        small.event_cells.pop();
        assert_eq!(
            WorldProject::new(
                WorldChunkCoord::new(0, 0),
                [PlacedMap::new(WorldChunkCoord::new(0, 0), small)],
                &known,
            ),
            Err(WorldProjectError::UnexpectedMapSize {
                map: MapProjectId::new("small").unwrap(),
                width: STANDARD_MAP_WIDTH - 1,
                height: STANDARD_MAP_HEIGHT,
            })
        );
    }

    #[test]
    fn exposes_a_row_major_window_with_empty_neighbor_slots() {
        let known = known_tiles();
        let center = WorldChunkCoord::new(0, 0);
        assert_eq!((center.x(), center.y()), (0, 0));
        let world = WorldProject::new(
            center,
            [
                placed(0, 0, "center"),
                placed(-1, -1, "northwest"),
                placed(1, 0, "east"),
            ],
            &known,
        )
        .unwrap();
        let window = world.preload_window(world.initial()).unwrap();
        assert_eq!(window[0].coordinate(), WorldChunkCoord::new(-1, -1));
        assert_eq!(window[0].project().unwrap().id.as_str(), "northwest");
        assert!(window[1].is_empty());
        assert_eq!(window[4].project().unwrap().id.as_str(), "center");
        assert_eq!(window[5].project().unwrap().id.as_str(), "east");
        assert!(window[8].is_empty());
        assert_eq!(
            world
                .map_at(WorldChunkCoord::new(1, 0))
                .unwrap()
                .id
                .as_str(),
            "east"
        );
        assert_eq!(world.maps().count(), 3);
    }

    #[test]
    fn rejects_a_window_that_cannot_contain_all_nine_coordinates() {
        let known = known_tiles();
        let edge = WorldChunkCoord::new(i32::MAX, 0);
        let world = WorldProject::new(edge, [PlacedMap::new(edge, map("edge"))], &known).unwrap();
        assert_eq!(
            world.preload_window(edge),
            Err(WorldProjectError::WindowOutOfBounds(edge))
        );
    }
}
