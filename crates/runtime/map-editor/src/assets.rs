use std::{collections::BTreeSet, error::Error, fs, path::PathBuf};

use game_assets::{AssetKey, DecodedImage};
use game_fs_assets::{load_catalog, read_optional_text, read_tile_sources};
use game_native_target::NativeAssets;
use map_assets::{build_tile_assets, project_from_json_or_default};
use map_project::{AtomicTileId, MapProject};
use map_project_storage::{FILE_EXTENSION, MapProjectReader};
use map_render::AtomicTileCatalog;
use map_tile_semantics::TileSemanticsCatalog;
use punctum_gpu::Rgba8;

pub struct EditorAssets {
    pub native: NativeAssets,
    pub catalog: AtomicTileCatalog,
    pub project_ids: Vec<AtomicTileId>,
    pub ids: Vec<AtomicTileId>,
    pub semantics: TileSemanticsCatalog,
}

pub fn load_assets() -> Result<EditorAssets, Box<dyn Error>> {
    let root = asset_root();
    let catalog = load_catalog(&root)?;
    let assets = build_tile_assets(read_tile_sources(&root, &catalog)?)?;
    let mut images = vec![(
        AssetKey::from_resource_template("solid/white".into()),
        DecodedImage::solid(Rgba8::new(255, 255, 255, 255)),
    )];
    images.extend(assets.images);
    let native = NativeAssets::new(images)?;
    let hidden_palette_tile = AtomicTileId::new("tile-0030")?;
    let project_ids = assets.ids;
    let known = project_ids.iter().cloned().collect::<BTreeSet<_>>();
    let semantics_path = root.join("source/map/tile/tile-semantics-v1.json");
    let semantics = TileSemanticsCatalog::from_json(&fs::read_to_string(semantics_path)?, &known)?;
    let ids = project_ids
        .iter()
        .filter(|id| *id != &hidden_palette_tile)
        .cloned()
        .collect();
    Ok(EditorAssets {
        native,
        catalog: assets.catalog,
        project_ids,
        ids,
        semantics,
    })
}

pub fn load_project(
    path: &std::path::Path,
    ids: &[AtomicTileId],
) -> Result<MapProject, Box<dyn Error>> {
    if path
        .extension()
        .is_some_and(|extension| extension == FILE_EXTENSION)
    {
        let known = ids.iter().cloned().collect::<BTreeSet<_>>();
        return Ok(MapProjectReader::read(&fs::read(path)?, &known)?);
    }
    let json = read_optional_text(path)?;
    Ok(project_from_json_or_default(json.as_deref(), ids)?)
}

pub fn default_project_path() -> PathBuf {
    let maps = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../maps");
    let compressed = maps.join("demo-map.g3mp");
    if compressed.exists() {
        compressed
    } else {
        maps.join("demo-map.json")
    }
}

fn asset_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../assets")
}

#[cfg(test)]
#[path = "../tests/unit/assets.rs"]
mod tests;
