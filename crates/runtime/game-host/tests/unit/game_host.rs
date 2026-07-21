use game_data::CurrentDataSet;
use game_foundation::{Direction, GameCommand as FoundationCommand};
use game_session::ProductSession;

use super::{CreatureGameApp, product_command};

#[test]
fn product_route_maps_world_commands_but_rejects_summary_battle_resolution() {
    assert!(matches!(
        product_command(FoundationCommand::Move {
            direction: Direction::Left,
        }),
        Ok(game_session::ProductCommand::Move(Direction::Left))
    ));
    assert!(
        product_command(FoundationCommand::ResolveBattle {
            outcome: game_foundation::BattleOutcome::Victory,
            hp: 1,
            pp: 1,
        })
        .is_err()
    );
}

#[test]
fn checked_in_product_content_package_loads() -> Result<(), Box<dyn std::error::Error>> {
    let package = super::load_product_content_package()?;
    assert_eq!(package.manifest().storage_key(), "starter-region@1");
    let trainer = package
        .content()
        .trainer(
            &game_foundation::TrainerId::new("route-rival")
                .map_err(|error| std::io::Error::other(format!("trainer id: {error:?}")))?,
        )
        .ok_or("route rival is missing")?;
    assert_eq!(
        trainer
            .pokemon()
            .first()
            .map(game_foundation::TrainerPokemon::species),
        Some("Bulbasaur")
    );
    let data = CurrentDataSet::embedded()?;
    let product = ProductSession::from_package(data, package)
        .map_err(|error| std::io::Error::other(format!("product session: {error:?}")))?;
    assert_eq!(
        product.snapshot().content_package().storage_key(),
        "starter-region@1"
    );
    Ok(())
}

#[test]
#[ignore = "known asset gap: pokemon/0351/form/00/normal/back/{00,01} is absent"]
fn complete_game_atlas_fits_wgpu_texture_limits() {
    let app = CreatureGameApp::new().unwrap();
    let size = app.assets.atlas_size();
    assert!(size.width <= 8_192, "atlas width was {}", size.width);
    assert!(size.height <= 8_192, "atlas height was {}", size.height);
}
