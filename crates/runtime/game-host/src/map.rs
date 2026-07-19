use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fs, io,
    path::{Component, Path, PathBuf},
};

use game_assets::{AssetKey, DecodedImage};
use game_fs_assets::{load_catalog, read_tile_sources};
use map_assets::{build_tile_assets, project_from_json_or_default};
use map_project::{
    AtomicTileId, Collision, CompositeTile, CompositeTileId, MapActor, MapEventKind, MapProject,
    MapProjectId, TilePosition, VisualCell,
};
use map_render::AtomicTileCatalog;
use map_tile_semantics::TileSemanticsCatalog;
use serde::Deserialize;
use world_project::{
    PlacedMap, STANDARD_MAP_HEIGHT, STANDARD_MAP_WIDTH, WorldChunkCoord, WorldProject,
};

pub struct LoadedMap {
    pub project: MapProject,
    pub catalog: AtomicTileCatalog,
    pub images: Vec<(AssetKey, DecodedImage)>,
}

#[derive(Deserialize)]
struct WorldManifest {
    format_version: String,
    initial: [i32; 2],
    maps: Vec<WorldManifestEntry>,
}

#[derive(Deserialize)]
struct WorldManifestEntry {
    coordinate: [i32; 2],
    file: String,
}

pub fn load_map() -> Result<LoadedMap, Box<dyn Error>> {
    let root = asset_root();
    let catalog = load_catalog(&root)?;
    let assets = build_tile_assets(read_tile_sources(&root, &catalog)?)?;
    let known = assets.ids.iter().cloned().collect::<BTreeSet<_>>();
    let semantics = load_semantics(&root, &known)?;
    let project = load_region(&region_manifest_path(), &assets.ids, &semantics)?;
    Ok(LoadedMap {
        project,
        catalog: assets.catalog,
        images: assets.images,
    })
}

fn asset_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../assets")
}

fn region_manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../maps/verdant-route/world.json")
}

fn load_region(
    path: &Path,
    ids: &[AtomicTileId],
    semantics: &TileSemanticsCatalog,
) -> Result<MapProject, Box<dyn Error>> {
    let manifest: WorldManifest = serde_json::from_str(&fs::read_to_string(path)?)?;
    if manifest.format_version != "world-map-layout-v1" {
        return Err(invalid_data("unsupported world map layout format"));
    }
    let root = path
        .parent()
        .ok_or_else(|| invalid_data("world map layout has no parent directory"))?;
    let known = ids.iter().cloned().collect::<BTreeSet<_>>();
    let placed_maps = manifest
        .maps
        .into_iter()
        .map(|entry| {
            let map_path = map_path(root, &entry.file)?;
            let json = fs::read_to_string(&map_path)?;
            let project = project_from_json_or_default(Some(&json), ids)?;
            validate_semantics(&project, semantics)?;
            Ok(PlacedMap::new(
                WorldChunkCoord::new(entry.coordinate[0], entry.coordinate[1]),
                project,
            ))
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    let initial = WorldChunkCoord::new(manifest.initial[0], manifest.initial[1]);
    let world = WorldProject::new(initial, placed_maps, &known)
        .map_err(|error| invalid_data(&format!("invalid world project: {error:?}")))?;
    compose_world_map(&world, &known)
}

fn load_semantics(
    root: &Path,
    known: &BTreeSet<AtomicTileId>,
) -> Result<TileSemanticsCatalog, Box<dyn Error>> {
    let path = root.join("source/map/tile/tile-semantics-v1.json");
    Ok(TileSemanticsCatalog::from_json(
        &fs::read_to_string(path)?,
        known,
    )?)
}

fn validate_semantics(
    project: &MapProject,
    semantics: &TileSemanticsCatalog,
) -> Result<(), Box<dyn Error>> {
    let diagnostics = semantics.lint(project);
    if diagnostics.is_empty() {
        return Ok(());
    }
    Err(invalid_data(&format!(
        "map {} has {} tile semantic error(s): {:?}",
        project.id,
        diagnostics.len(),
        diagnostics[0]
    )))
}

fn map_path(root: &Path, file: &str) -> Result<PathBuf, Box<dyn Error>> {
    let relative = Path::new(file);
    if relative
        .extension()
        .is_none_or(|extension| extension != "json")
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(invalid_data("world map entry must be a local JSON file"));
    }
    Ok(root.join(relative))
}

fn compose_world_map(
    world: &WorldProject,
    known: &BTreeSet<AtomicTileId>,
) -> Result<MapProject, Box<dyn Error>> {
    let coordinates = world
        .maps()
        .map(|(coordinate, _)| coordinate)
        .collect::<Vec<_>>();
    let first = coordinates
        .first()
        .ok_or_else(|| invalid_data("world project has no maps"))?;
    let (mut min_x, mut max_x) = (first.x(), first.x());
    let (mut min_y, mut max_y) = (first.y(), first.y());
    for coordinate in &coordinates[1..] {
        min_x = min_x.min(coordinate.x());
        max_x = max_x.max(coordinate.x());
        min_y = min_y.min(coordinate.y());
        max_y = max_y.max(coordinate.y());
    }
    let width = checked_extent(min_x, max_x, STANDARD_MAP_WIDTH)?;
    let height = checked_extent(min_y, max_y, STANDARD_MAP_HEIGHT)?;
    let cell_count = usize::from(width) * usize::from(height);
    let mut visual_cells = vec![VisualCell::new(None); cell_count];
    let mut collision_cells = vec![Collision::Blocked; cell_count];
    let mut event_cells: Vec<Option<MapEventKind>> = vec![None; cell_count];
    let mut actors = Vec::new();
    let mut materials = BTreeMap::<CompositeTileId, CompositeTile>::new();

    for (coordinate, map) in world.maps() {
        let origin_x = checked_origin(coordinate.x(), min_x, STANDARD_MAP_WIDTH)?;
        let origin_y = checked_origin(coordinate.y(), min_y, STANDARD_MAP_HEIGHT)?;
        for material in &map.materials {
            match materials.get(&material.id) {
                Some(existing) if existing != material => {
                    return Err(invalid_data(&format!(
                        "material {} differs between region maps",
                        material.id
                    )));
                }
                Some(_) => {}
                None => {
                    materials.insert(material.id.clone(), material.clone());
                }
            }
        }
        for row in 0..map.height {
            for column in 0..map.width {
                let source = usize::from(row) * usize::from(map.width) + usize::from(column);
                let target = usize::from(origin_y + row) * usize::from(width)
                    + usize::from(origin_x + column);
                visual_cells[target] = map.visual_cells[source].clone();
                collision_cells[target] = map.collision_cells[source];
                event_cells[target] = map.event_cells[source];
            }
        }
        for actor in &map.actors {
            actors.push(MapActor::new(
                actor.id.clone(),
                TilePosition::new(origin_x + actor.position.x(), origin_y + actor.position.y()),
                actor.facing,
                actor.appearance.clone(),
            ));
        }
    }

    let initial = world.initial();
    let initial_map = world
        .map_at(initial)
        .ok_or_else(|| invalid_data("world project initial map is missing"))?;
    let player_spawn = TilePosition::new(
        checked_origin(initial.x(), min_x, STANDARD_MAP_WIDTH)? + initial_map.player_spawn.x(),
        checked_origin(initial.y(), min_y, STANDARD_MAP_HEIGHT)? + initial_map.player_spawn.y(),
    );
    let project = MapProject {
        format_version: map_project::FORMAT_VERSION.into(),
        id: MapProjectId::new("verdant-route")?,
        tile_size: initial_map.tile_size,
        width,
        height,
        materials: materials.into_values().collect(),
        visual_cells,
        collision_cells,
        event_cells,
        player_spawn,
        actors,
    };
    project.validate(known)?;
    Ok(project)
}

fn checked_extent(min: i32, max: i32, map_extent: u16) -> Result<u16, Box<dyn Error>> {
    let map_count = i64::from(max) - i64::from(min) + 1;
    u16::try_from(map_count * i64::from(map_extent))
        .map_err(|_| invalid_data("world map layout exceeds map-project dimensions"))
}

fn checked_origin(coordinate: i32, minimum: i32, map_extent: u16) -> Result<u16, Box<dyn Error>> {
    u16::try_from((i64::from(coordinate) - i64::from(minimum)) * i64::from(map_extent))
        .map_err(|_| invalid_data("world map coordinate is outside the composed layout"))
}

fn invalid_data(message: &str) -> Box<dyn Error> {
    Box::new(io::Error::new(
        io::ErrorKind::InvalidData,
        message.to_owned(),
    ))
}

#[cfg(test)]
#[path = "../tests/unit/map.rs"]
mod tests;
