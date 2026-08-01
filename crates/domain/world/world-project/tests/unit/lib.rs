use std::collections::BTreeSet;

use map_project::{
    AtomicTileId, CompositeTile, CompositeTileId, MapError, MapProject, MapProjectId,
};

use super::{
    PlacedMap, STANDARD_MAP_HEIGHT, STANDARD_MAP_WIDTH, WorldChunkCoord, WorldProject,
    WorldProjectError,
};

fn known_tiles() -> Result<BTreeSet<AtomicTileId>, MapError> {
    Ok(BTreeSet::from([AtomicTileId::new("tile-0001")?]))
}

fn map(id: &str) -> Result<MapProject, MapError> {
    let tile = AtomicTileId::new("tile-0001")?;
    Ok(MapProject::blank(
        MapProjectId::new(id)?,
        STANDARD_MAP_WIDTH,
        STANDARD_MAP_HEIGHT,
        Some(CompositeTile::new(
            CompositeTileId::new("ground")?,
            vec![tile],
        )),
    ))
}

fn placed(x: i32, y: i32, id: &str) -> Result<PlacedMap, MapError> {
    Ok(PlacedMap::new(WorldChunkCoord::new(x, y), map(id)?))
}

#[test]
fn rejects_empty_world_and_missing_initial_map() -> Result<(), Box<dyn std::error::Error>> {
    let known = known_tiles()?;
    assert_eq!(
        WorldProject::new(WorldChunkCoord::new(0, 0), [], &known),
        Err(WorldProjectError::EmptyWorld)
    );
    assert_eq!(
        WorldProject::new(WorldChunkCoord::new(1, 0), [placed(0, 0, "a")?], &known),
        Err(WorldProjectError::InitialMapMissing(WorldChunkCoord::new(
            1, 0
        )))
    );
    Ok(())
}

#[test]
fn rejects_duplicate_coordinates_and_map_ids() -> Result<(), Box<dyn std::error::Error>> {
    let known = known_tiles()?;
    assert_eq!(
        WorldProject::new(
            WorldChunkCoord::new(0, 0),
            [placed(0, 0, "a")?, placed(0, 0, "b")?],
            &known,
        ),
        Err(WorldProjectError::DuplicateCoordinate(
            WorldChunkCoord::new(0, 0)
        ))
    );
    assert_eq!(
        WorldProject::new(
            WorldChunkCoord::new(0, 0),
            [placed(0, 0, "a")?, placed(1, 0, "a")?],
            &known,
        ),
        Err(WorldProjectError::DuplicateMapId(MapProjectId::new("a")?))
    );
    Ok(())
}

#[test]
fn rejects_invalid_and_non_standard_maps() -> Result<(), Box<dyn std::error::Error>> {
    let known = known_tiles()?;
    let mut invalid = map("invalid")?;
    invalid.format_version = "unsupported".into();
    assert!(matches!(
        WorldProject::new(
            WorldChunkCoord::new(0, 0),
            [PlacedMap::new(WorldChunkCoord::new(0, 0), invalid)],
            &known,
        ),
        Err(WorldProjectError::Map(_))
    ));

    let mut small = map("small")?;
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
            map: MapProjectId::new("small")?,
            width: STANDARD_MAP_WIDTH - 1,
            height: STANDARD_MAP_HEIGHT,
        })
    );
    Ok(())
}

#[test]
fn exposes_a_row_major_window_with_empty_neighbor_slots() -> Result<(), Box<dyn std::error::Error>>
{
    let known = known_tiles()?;
    let center = WorldChunkCoord::new(0, 0);
    let world = WorldProject::new(
        center,
        [
            placed(0, 0, "center")?,
            placed(-1, -1, "northwest")?,
            placed(1, 0, "east")?,
        ],
        &known,
    )?;
    let window = world.preload_window(world.initial())?;
    assert_eq!(window[0].coordinate, WorldChunkCoord::new(-1, -1));
    assert_eq!(
        window[0].project.map(|project| project.id.as_str()),
        Some("northwest")
    );
    assert!(window[1].is_empty());
    assert_eq!(
        window[4].project.map(|project| project.id.as_str()),
        Some("center")
    );
    assert_eq!(
        window[5].project.map(|project| project.id.as_str()),
        Some("east")
    );
    assert!(window[8].is_empty());
    assert_eq!(
        world
            .map_at(WorldChunkCoord::new(1, 0))
            .map(|project| project.id.as_str()),
        Some("east")
    );
    assert_eq!(world.maps().count(), 3);
    Ok(())
}

#[test]
fn rejects_a_window_that_cannot_contain_all_nine_coordinates()
-> Result<(), Box<dyn std::error::Error>> {
    let known = known_tiles()?;
    let edge = WorldChunkCoord::new(i32::MAX, 0);
    let world = WorldProject::new(edge, [PlacedMap::new(edge, map("edge")?)], &known)?;
    assert_eq!(
        world.preload_window(edge),
        Err(WorldProjectError::WindowOutOfBounds(edge))
    );
    Ok(())
}

#[test]
fn exposes_world_size_and_pixel_origins() -> Result<(), Box<dyn std::error::Error>> {
    let known = known_tiles()?;
    let world = WorldProject::new(
        WorldChunkCoord::new(0, 0),
        [
            placed(0, 0, "center")?,
            placed(-1, -1, "northwest")?,
            placed(1, 0, "east")?,
        ],
        &known,
    )?;
    assert_eq!(world.size(), Ok((216, 112)));
    assert_eq!(world.origin_of(WorldChunkCoord::new(-1, -1)), Ok((0, 0)));
    assert_eq!(world.origin_of(WorldChunkCoord::new(0, 0)), Ok((72, 56)));
    assert_eq!(world.origin_of(WorldChunkCoord::new(1, 0)), Ok((144, 56)));
    Ok(())
}

#[test]
fn rejects_a_size_or_origin_that_exceeds_u16_pixels() -> Result<(), Box<dyn std::error::Error>> {
    let known = known_tiles()?;
    let world = WorldProject::new(
        WorldChunkCoord::new(0, 0),
        [placed(0, 0, "west")?, placed(i32::MAX, 0, "far-east")?],
        &known,
    )?;
    assert_eq!(world.size(), Err(WorldProjectError::LayoutOverflow));
    assert_eq!(
        world.origin_of(WorldChunkCoord::new(i32::MAX, 0)),
        Err(WorldProjectError::LayoutOverflow)
    );
    Ok(())
}
