use std::{error::Error, path::PathBuf};

use game_asset_plan::{assemble_assets, asset_requests};
use game_assets::{AssetKey, DecodedImage};
use game_data::PokedexData;
use game_fs_assets::{load_catalog, read_asset_requests};
use game_native_target::NativeAssets;
use game_session::DemoSpriteManifest;
use world_application::WorldObservation;

pub fn load_game_assets(
    manifest: &DemoSpriteManifest,
    pokedex: &PokedexData,
    world: &WorldObservation,
    map_images: Vec<(AssetKey, DecodedImage)>,
) -> Result<NativeAssets, Box<dyn Error>> {
    let root = asset_root();
    let catalog = load_catalog(&root)?;
    let sources = read_asset_requests(&root, &catalog, asset_requests(manifest, pokedex, world))?;
    Ok(assemble_assets(sources, map_images)?)
}

fn asset_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../assets")
}

#[cfg(test)]
#[path = "../tests/unit/sprites.rs"]
mod tests;
