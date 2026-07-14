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
        let asset = AssetKey::new(format!("map/tile/{canonical_name}"))
            .expect("a non-empty tile id always produces a valid asset key");
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
    default_project(ids).map_err(|error| TileAssetsError::Project(error.to_string()))
}

fn default_project(ids: &[AtomicTileId]) -> Result<MapProject, map_project::MapError> {
    let tile = |name: &str| {
        ids.iter()
            .find(|id| id.as_str() == name)
            .or_else(|| ids.first())
            .cloned()
            .expect("the caller rejects an empty tile set")
    };
    let ground_id = CompositeTileId::new("material-0000")?;
    let flower_id = CompositeTileId::new("material-0001")?;
    let grass_id = CompositeTileId::new("material-0002")?;
    let rock_id = CompositeTileId::new("material-0003")?;
    let border_id = CompositeTileId::new("material-0004")?;
    let mut project = MapProject::blank(
        MapProjectId::new("demo-map")?,
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
mod tests {
    use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};

    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        PngEncoder::new(&mut bytes)
            .write_image(
                &vec![255; (width * height * 4) as usize],
                width,
                height,
                ExtendedColorType::Rgba8,
            )
            .unwrap();
        bytes
    }

    fn source(name: &str, bytes: Vec<u8>) -> TileSource {
        TileSource {
            name: name.into(),
            bytes,
        }
    }

    #[test]
    fn tile_assembly_is_sorted_and_validated() {
        let assets = build_tile_assets(vec![
            source("tile-0102", png(16, 16)),
            source("tile-0101", png(16, 16)),
        ])
        .unwrap();
        assert_eq!(assets.ids[0].as_str(), "tile-0101");
        assert_eq!(assets.ids[1].as_str(), "tile-0102");
        assert_eq!(assets.images[0].1.size(), PixelSize::new(16, 16));
        assert_eq!(assets.images[0].0.as_str(), "map/tile/0101");
        assert_eq!(assets.catalog.len(), 2);

        for error in [
            build_tile_assets(Vec::new()).unwrap_err(),
            build_tile_assets(vec![source("bad", b"not png".to_vec())]).unwrap_err(),
            build_tile_assets(vec![source("bad", png(1, 1))]).unwrap_err(),
            build_tile_assets(vec![source(" ", png(16, 16))]).unwrap_err(),
            build_tile_assets(vec![
                source("duplicate", png(16, 16)),
                source("duplicate", png(16, 16)),
            ])
            .unwrap_err(),
        ] {
            assert!(!error.to_string().is_empty());
        }
    }

    #[test]
    fn project_loading_uses_the_same_known_tile_set_for_defaults_and_json() {
        let assets = build_tile_assets(vec![source("tile-0102", png(16, 16))]).unwrap();
        let project = project_from_json_or_default(None, &assets.ids).unwrap();
        assert_eq!((project.width, project.height), (24, 16));

        let known = assets.ids.iter().cloned().collect();
        let json = project.to_json_pretty(&known).unwrap();
        assert_eq!(
            project_from_json_or_default(Some(&json), &assets.ids)
                .unwrap()
                .id,
            project.id
        );
        assert!(project_from_json_or_default(None, &[]).is_err());
        let error = project_from_json_or_default(Some("{}"), &assets.ids).unwrap_err();
        assert!(!error.to_string().is_empty());
    }
}
