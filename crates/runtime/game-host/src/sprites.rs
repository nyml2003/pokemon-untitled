use std::{error::Error, path::PathBuf};

use game_asset_plan::{assemble_assets, asset_requests};
use game_assets::{AssetKey, DecodedImage};
use game_data::PokedexData;
use game_fs_assets::{load_catalog, read_asset_requests};
use game_native_target::NativeAssets;
use game_session::DemoSpriteManifest;

pub fn load_game_assets(
    manifest: &DemoSpriteManifest,
    pokedex: &PokedexData,
    map_images: Vec<(AssetKey, DecodedImage)>,
) -> Result<NativeAssets, Box<dyn Error>> {
    let root = asset_root();
    let catalog = load_catalog(&root)?;
    let sources = read_asset_requests(&root, &catalog, asset_requests(manifest, pokedex))?;
    Ok(assemble_assets(sources, map_images)?)
}

fn asset_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../assets")
}

#[cfg(test)]
mod tests {
    use game_data::CurrentDataSet;
    use game_data::PokedexData;
    use game_session::GameSession;
    use game_view::{opponent_front_asset, player_back_asset, pokemon_icon_asset};

    use super::load_game_assets;

    #[test]
    fn generated_roster_loads_every_stable_sprite_key() {
        let game = GameSession::new_demo(CurrentDataSet::embedded().unwrap(), 17).unwrap();
        let assets = load_game_assets(
            &game.sprite_manifest().unwrap(),
            &PokedexData::embedded_hoenn().unwrap(),
            Vec::new(),
        )
        .unwrap();
        for slot in 0..battle_application::TEAM_SIZE {
            for frame in 0..2 {
                assert!(assets.resource(&player_back_asset(slot, frame)).is_some());
                assert!(assets.resource(&pokemon_icon_asset(slot, frame)).is_some());
                assert!(
                    assets
                        .resource(&opponent_front_asset(slot, frame))
                        .is_some()
                );
            }
        }
    }
}
