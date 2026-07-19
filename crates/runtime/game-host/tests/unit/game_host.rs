use super::CreatureGameApp;

#[test]
#[ignore = "known asset gap: pokemon/0351/form/00/normal/back/{00,01} is absent"]
fn complete_game_atlas_fits_wgpu_texture_limits() {
    let app = CreatureGameApp::new().unwrap();
    let size = app.assets.atlas_size();
    assert!(size.width <= 8_192, "atlas width was {}", size.width);
    assert!(size.height <= 8_192, "atlas height was {}", size.height);
}
