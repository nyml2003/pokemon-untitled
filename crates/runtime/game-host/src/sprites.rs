use std::{error::Error, fs, path::PathBuf};

use game_asset_plan::{assemble_assets_with_extra, asset_requests};
use game_assets::{AssetKey, DecodedImage, decode_png};
use game_data::PokedexData;
use game_fs_assets::{load_catalog, read_asset_requests};
use game_native_target::NativeAssets;
use game_session::DemoSpriteManifest;
use game_view::{page_party_pokemon_asset, page_world_player_asset, page_world_tile_asset};
use world_application::WorldObservation;

pub fn load_host_assets(
    manifest: &DemoSpriteManifest,
    pokedex: &PokedexData,
    world: &WorldObservation,
    map_images: Vec<(AssetKey, DecodedImage)>,
) -> Result<NativeAssets, Box<dyn Error>> {
    let root = asset_root();
    let catalog = load_catalog(&root)?;
    let sources = read_asset_requests(&root, &catalog, asset_requests(manifest, pokedex, world))?;
    Ok(assemble_assets_with_extra(
        sources,
        map_images,
        page_images(&root)?,
    )?)
}

fn page_images(root: &std::path::Path) -> Result<Vec<(AssetKey, DecodedImage)>, Box<dyn Error>> {
    let treecko_party = page_party_pokemon_asset("Treecko").ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Treecko page asset slot is not registered",
        )
    })?;
    let mut sources = vec![
        (
            page_world_player_asset(),
            String::from("source/character/red/down/stand/00.png"),
        ),
        (
            treecko_party,
            String::from("source/pokemon/0263/form/00/normal/front/00.png"),
        ),
    ];
    for tile in [8_u16, 9, 10, 11, 12, 13] {
        sources.push((
            page_world_tile_asset(tile),
            format!("source/map/tile/{tile:04}.png"),
        ));
    }
    let mut images = Vec::new();
    for (key, relative_path) in sources {
        let path = root.join(relative_path);
        let bytes = fs::read(&path).map_err(|error| {
            std::io::Error::new(error.kind(), format!("{}: {error}", path.display()))
        })?;
        let image = decode_png(&bytes).map_err(|error| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("{}: {error}", path.display()),
            )
        })?;
        images.push((key, image));
    }
    Ok(images)
}

fn asset_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../assets")
}
