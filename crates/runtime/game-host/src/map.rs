use std::{error::Error, path::PathBuf};

use game_assets::{AssetKey, DecodedImage};
use game_fs_assets::{load_catalog, read_optional_text, read_tile_sources};
use map_assets::{build_tile_assets, project_from_json_or_default};
use map_project::MapProject;
use map_render::AtomicTileCatalog;

pub struct LoadedMap {
    pub project: MapProject,
    pub catalog: AtomicTileCatalog,
    pub images: Vec<(AssetKey, DecodedImage)>,
}

pub fn load_map() -> Result<LoadedMap, Box<dyn Error>> {
    let root = asset_root();
    let catalog = load_catalog(&root)?;
    let assets = build_tile_assets(read_tile_sources(&root, &catalog)?)?;
    let json = read_optional_text(&project_path())?;
    let project = project_from_json_or_default(json.as_deref(), &assets.ids)?;
    Ok(LoadedMap {
        project,
        catalog: assets.catalog,
        images: assets.images,
    })
}

fn asset_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../assets")
}

fn project_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../maps/demo-map.json")
}
