//! Pure tile asset and map project assembly.

#![forbid(unsafe_code)]

use std::{collections::BTreeSet, error::Error, fmt};

use game_assets::{AssetKey, DecodedImage, decode_png};
use map_project::{
    AtomicTileId, Collision, CompositeTile, CompositeTileId, MapEventKind, MapProject,
    MapProjectId, TilePosition,
};
use map_render::{AtomicTileAsset, AtomicTileCatalog};
use punctum_gpu::PixelSize;

pub struct TileSource {
    pub name: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug)]
pub struct TileAssets {
    pub ids: Vec<AtomicTileId>,
    pub catalog: AtomicTileCatalog,
    pub images: Vec<(AssetKey, DecodedImage)>,
}

pub fn build_tile_assets(mut sources: Vec<TileSource>) -> Result<TileAssets, TileAssetsError> {
    if sources.is_empty() {
        return Err(TileAssetsError::Empty);
    }
    sources.sort_by(|left, right| left.name.cmp(&right.name));
    let mut ids = Vec::with_capacity(sources.len());
    let mut resources = Vec::with_capacity(sources.len());
    let mut images = Vec::with_capacity(sources.len());
    for source in sources {
        let id = AtomicTileId::new(&source.name).map_err(|error| TileAssetsError::InvalidId {
            name: source.name.clone(),
            message: error.to_string(),
        })?;
        let canonical_name = id.as_str().strip_prefix("tile-").unwrap_or(id.as_str());
        let asset = AssetKey::from_resource_template(format!("map/tile/{canonical_name}"));
        let image = decode_png(&source.bytes).map_err(|error| TileAssetsError::Decode {
            name: source.name.clone(),
            message: error.to_string(),
        })?;
        if image.size() != PixelSize::new(16, 16) {
            return Err(TileAssetsError::WrongSize {
                name: source.name,
                actual: image.size(),
            });
        }
        ids.push(id.clone());
        resources.push(AtomicTileAsset {
            id,
            asset: asset.clone(),
        });
        images.push((asset, image));
    }
    let catalog = AtomicTileCatalog::new(resources)
        .map_err(|error| TileAssetsError::Catalog(error.to_string()))?;
    Ok(TileAssets {
        ids,
        catalog,
        images,
    })
}

pub fn project_from_json_or_default(
    json: Option<&str>,
    ids: &[AtomicTileId],
) -> Result<MapProject, TileAssetsError> {
    if ids.is_empty() {
        return Err(TileAssetsError::Empty);
    }
    if let Some(json) = json {
        let known = ids.iter().cloned().collect::<BTreeSet<_>>();
        return MapProject::from_json(json, &known)
            .map_err(|error| TileAssetsError::Project(error.to_string()));
    }
    default_project(ids)
}

fn default_project(ids: &[AtomicTileId]) -> Result<MapProject, TileAssetsError> {
    let fallback = ids.first().ok_or(TileAssetsError::Empty)?;
    let tile = |name: &str| {
        ids.iter()
            .find(|id| id.as_str() == name)
            .cloned()
            .unwrap_or_else(|| fallback.clone())
    };
    let project_error = |error: map_project::MapError| TileAssetsError::Project(error.to_string());
    let ground_id = CompositeTileId::new("material-0000").map_err(project_error)?;
    let flower_id = CompositeTileId::new("material-0001").map_err(project_error)?;
    let grass_id = CompositeTileId::new("material-0002").map_err(project_error)?;
    let rock_id = CompositeTileId::new("material-0003").map_err(project_error)?;
    let border_id = CompositeTileId::new("material-0004").map_err(project_error)?;
    let mut project = MapProject::blank(
        MapProjectId::new("demo-map").map_err(project_error)?,
        24,
        16,
        Some(CompositeTile::new(
            ground_id.clone(),
            vec![tile("tile-0102")],
        )),
    );
    project.materials.extend([
        CompositeTile::new(flower_id.clone(), vec![tile("tile-0101")]),
        CompositeTile::new(grass_id.clone(), vec![tile("tile-0102"), tile("tile-0110")]),
        CompositeTile::new(rock_id.clone(), vec![tile("tile-0102"), tile("tile-0223")]),
        CompositeTile::new(border_id.clone(), vec![tile("tile-0251")]),
    ]);
    project.player_spawn = TilePosition::new(3, 6);
    for y in 0..project.height {
        for x in 0..project.width {
            let border = x == 0 || y == 0 || x + 1 == project.width || y + 1 == project.height;
            let grass = ((6..=10).contains(&x) && (2..=7).contains(&y))
                || ((15..=20).contains(&x) && (8..=13).contains(&y));
            let rocks = matches!(
                (x, y),
                (3, 3) | (4, 3) | (12, 5) | (12, 6) | (18, 4) | (19, 4)
            );
            let index = usize::from(y * project.width + x);
            let (material, collision, event) = if border {
                (Some(border_id.clone()), Collision::Blocked, None)
            } else if rocks {
                (Some(rock_id.clone()), Collision::Blocked, None)
            } else if grass {
                (
                    Some(grass_id.clone()),
                    Collision::Walkable,
                    Some(MapEventKind::Encounter),
                )
            } else if (x + y) % 7 == 0 {
                (Some(flower_id.clone()), Collision::Walkable, None)
            } else {
                (Some(ground_id.clone()), Collision::Walkable, None)
            };
            project.visual_cells[index].material = material;
            project.collision_cells[index] = collision;
            project.event_cells[index] = event;
        }
    }
    Ok(project)
}

#[derive(Debug)]
pub enum TileAssetsError {
    Empty,
    InvalidId { name: String, message: String },
    Decode { name: String, message: String },
    WrongSize { name: String, actual: PixelSize },
    Catalog(String),
    Project(String),
}

impl fmt::Display for TileAssetsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("tile set is empty"),
            Self::InvalidId { name, message } => {
                write!(formatter, "invalid tile id {name}: {message}")
            }
            Self::Decode { name, message } => {
                write!(formatter, "cannot decode tile {name}: {message}")
            }
            Self::WrongSize { name, actual } => {
                write!(formatter, "tile {name} is {actual:?}, expected 16x16")
            }
            Self::Catalog(message) => write!(formatter, "invalid tile catalog: {message}"),
            Self::Project(message) => write!(formatter, "invalid map project: {message}"),
        }
    }
}

impl Error for TileAssetsError {}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
