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
mod tests {
    use game_data::CurrentDataSet;
    use game_data::PokedexData;
    use game_session::GameSession;
    use game_ui::WorldAnimation;
    use game_view::{
        opponent_front_asset, player_back_asset, pokemon_icon_asset, world_character_asset,
    };
    use world_application::WorldApplication;

    use crate::map::load_map;

    use super::{NativeAssets, asset_root, load_game_assets};

    #[test]
    fn generated_roster_loads_every_stable_sprite_key() {
        let game = GameSession::new_demo(CurrentDataSet::embedded().unwrap(), 17).unwrap();
        let assets = load_game_assets(
            &game.sprite_manifest().unwrap(),
            &PokedexData::embedded_gen3().unwrap(),
            game.snapshot().world(),
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

    #[test]
    fn checked_in_npc_uses_a_distinct_runtime_atlas_resource() {
        let loaded_map = load_map().unwrap();
        let world = WorldApplication::from_map_project(&loaded_map.project).unwrap();
        let game = GameSession::new(CurrentDataSet::embedded().unwrap(), world, 17).unwrap();
        let snapshot = game.snapshot();
        let npc = snapshot
            .world()
            .actors()
            .iter()
            .find(|actor| actor.appearance().as_str() == "dppt/000")
            .unwrap();
        let assets = load_game_assets(
            &game.sprite_manifest().unwrap(),
            &PokedexData::embedded_gen3().unwrap(),
            snapshot.world(),
            loaded_map.images,
        )
        .unwrap();
        let npc_key =
            world_character_asset(npc.appearance(), npc.facing(), WorldAnimation::Stand, 0);
        let player_key = world_character_asset(
            snapshot.world().actors()[0].appearance(),
            snapshot.world().actors()[0].facing(),
            WorldAnimation::Stand,
            0,
        );
        let npc_resource = assets.resource(&npc_key).unwrap();
        let player_resource = assets.resource(&player_key).unwrap();
        assert_ne!(npc_key, player_key);
        assert_ne!(npc_resource, player_resource);
        assert_ne!(
            assets.atlas().resource(npc_resource),
            assets.atlas().resource(player_resource)
        );
        let expected = game_assets::decode_png(
            &std::fs::read(asset_root().join("source/character/dppt/000/left/stand/00.png"))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(atlas_pixels(&assets, npc_resource), expected.rgba8());
    }

    fn atlas_pixels(assets: &NativeAssets, resource: punctum_gpu::ResourceId) -> Vec<u8> {
        let rect = assets.atlas().resource(resource).unwrap();
        let atlas = assets.atlas();
        let row_bytes = atlas.size().width as usize * 4;
        let image_row_bytes = rect.width as usize * 4;
        let mut pixels = Vec::with_capacity(image_row_bytes * rect.height as usize);
        for row in 0..rect.height as usize {
            let start = (rect.y as usize + row) * row_bytes + rect.x as usize * 4;
            pixels.extend_from_slice(&atlas.rgba8()[start..start + image_row_bytes]);
        }
        pixels
    }
}
