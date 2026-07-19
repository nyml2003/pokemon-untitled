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
