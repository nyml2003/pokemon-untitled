use game_view::{
    page_party_pokemon_asset, page_pokedex_icon_asset, page_pokedex_pokemon_asset,
    page_world_player_asset, page_world_tile_asset,
};

use super::load_page_assets;

#[test]
fn page_assets_register_every_runtime_page_slot() -> Result<(), Box<dyn std::error::Error>> {
    let assets = load_page_assets()?;
    let treecko = page_party_pokemon_asset("Treecko").ok_or("Treecko page slot is missing")?;
    let bulbasaur = page_pokedex_pokemon_asset(1).ok_or("Bulbasaur page slot is missing")?;
    let bulbasaur_icon = page_pokedex_icon_asset(1).ok_or("Bulbasaur icon slot is missing")?;
    for key in [
        page_world_player_asset(),
        page_world_tile_asset(8),
        treecko,
        bulbasaur,
        bulbasaur_icon,
    ] {
        assert!(
            assets.resource(&key).is_some(),
            "missing page asset {}",
            key.as_str()
        );
    }
    Ok(())
}
