use game_data::CurrentDataSet;
use game_session::GameSession;
use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};
use map_project::{
    AtomicTileId, CharacterAppearanceId, CompositeTile, CompositeTileId, MapActor, MapActorId,
    MapDirection, MapProject, MapProjectId, TilePosition,
};
use world_application::WorldApplication;

use super::*;

fn manifest() -> DemoSpriteManifest {
    GameSession::new_demo(CurrentDataSet::embedded().unwrap(), 17)
        .unwrap()
        .sprite_manifest()
        .unwrap()
}

fn world() -> world_application::WorldObservation {
    GameSession::new_demo(CurrentDataSet::embedded().unwrap(), 17)
        .unwrap()
        .snapshot()
        .world()
        .clone()
}

fn world_with_npc() -> world_application::WorldObservation {
    let tile = AtomicTileId::new("tile-0001").unwrap();
    let material = CompositeTileId::new("ground").unwrap();
    let mut project = MapProject::blank(
        MapProjectId::new("npc-test").unwrap(),
        3,
        1,
        Some(CompositeTile::new(material, vec![tile])),
    );
    project.player_spawn = TilePosition::new(0, 0);
    project.actors.push(MapActor::new(
        MapActorId::new("guide").unwrap(),
        TilePosition::new(1, 0),
        MapDirection::Left,
        CharacterAppearanceId::new("dppt/000").unwrap(),
    ));
    WorldApplication::from_map_project(&project)
        .unwrap()
        .observe()
        .unwrap()
}

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

#[test]
fn manifest_expands_to_stable_resource_and_asset_key_requests() {
    let pokedex = PokedexData::embedded_gen3().unwrap();
    let requests = asset_requests(&manifest(), &pokedex, &world());
    assert_eq!(requests.len(), 466);
    assert_eq!(
        requests[0].asset_key.as_str(),
        "character/red/down/stand/00"
    );
    assert_eq!(requests[0].resource_key.as_str(), "character/red/0/0");
    assert!(requests.iter().any(|request| {
        request.resource_key == player_back_asset(0, 0)
            && request.asset_key.as_str().contains("/normal/back/00")
    }));
    assert!(requests.iter().any(|request| {
        request.resource_key == pokemon_icon_asset(0, 0)
            && request.asset_key.as_str().contains("/normal/front/00")
    }));
    assert!(requests.iter().any(|request| {
        request.asset_key.as_str() == "ui/battle/move-category/status"
            && request.resource_key.as_str() == "ui/battle/move-category/status"
    }));
}

#[test]
fn npc_appearances_request_only_stand_and_walk_frames() {
    let requests = asset_requests(
        &manifest(),
        &PokedexData::embedded_gen3().unwrap(),
        &world_with_npc(),
    );
    let npc_requests = requests
        .iter()
        .filter(|request| {
            request
                .asset_key
                .as_str()
                .starts_with("character/dppt/000/")
        })
        .collect::<Vec<_>>();
    assert_eq!(npc_requests.len(), 12);
    assert!(
        npc_requests
            .iter()
            .all(|request| !request.asset_key.as_str().contains("/run/"))
    );
    assert!(npc_requests.iter().any(|request| {
        request.resource_key.as_str() == "character/dppt/000/1/2"
            && request.asset_key.as_str() == "character/dppt/000/left/walk/02"
    }));
}

#[test]
fn assembly_decodes_and_validates_declared_sizes() {
    let requests = asset_requests(
        &manifest(),
        &PokedexData::embedded_gen3().unwrap(),
        &world(),
    );
    let sources = requests
        .into_iter()
        .map(|request| AssetBytes {
            bytes: png(
                request.expected_size.map_or(1, |size| size.width),
                request.expected_size.map_or(1, |size| size.height),
            ),
            request,
        })
        .collect();
    let assets = assemble_assets(sources, Vec::new()).unwrap();
    assert!(assets.atlas_size().width > 0);
    assert!(assets.resource(&rounded_ui_asset()).is_some());
    assert!(assets.resource(&pill_ui_asset()).is_some());

    let errors = [
        assemble_assets(
            vec![AssetBytes {
                request: AssetRequest {
                    resource_key: AssetKey::new("bad/png").unwrap(),
                    asset_key: AssetKey::new("bad/png").unwrap(),
                    expected_size: None,
                },
                bytes: Vec::new(),
            }],
            Vec::new(),
        )
        .err()
        .unwrap(),
        assemble_assets(
            vec![AssetBytes {
                request: AssetRequest {
                    resource_key: AssetKey::new("bad/size").unwrap(),
                    asset_key: AssetKey::new("bad/size").unwrap(),
                    expected_size: Some(PixelSize::new(32, 32)),
                },
                bytes: png(1, 1),
            }],
            Vec::new(),
        )
        .err()
        .unwrap(),
        assemble_assets(
            vec![AssetBytes {
                request: AssetRequest {
                    resource_key: AssetKey::new("solid/white").unwrap(),
                    asset_key: AssetKey::new("solid/white").unwrap(),
                    expected_size: None,
                },
                bytes: png(1, 1),
            }],
            Vec::new(),
        )
        .err()
        .unwrap(),
    ];
    for error in errors {
        assert!(!error.to_string().is_empty());
    }
}

#[test]
fn generated_ui_masks_have_transparent_corners_and_opaque_centers() {
    for mask in [
        rounded_mask(64, 64, 6).unwrap(),
        rounded_mask(128, 64, 32).unwrap(),
    ] {
        let center =
            ((mask.size().height / 2 * mask.size().width + mask.size().width / 2) * 4 + 3) as usize;
        assert_eq!(mask.rgba8()[3], 0);
        assert_eq!(mask.rgba8()[center], 255);
    }
}
