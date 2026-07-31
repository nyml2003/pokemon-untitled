use game_data::CurrentDataSet;
use game_page_model::{PageModel, PageState, project_page};
use game_session::ProductSession;

use super::load_product_content_package;

#[test]
fn checked_in_product_content_package_loads() -> Result<(), Box<dyn std::error::Error>> {
    let package = load_product_content_package()?;
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
fn product_snapshot_projects_the_initial_player_page() -> Result<(), Box<dyn std::error::Error>> {
    let package = load_product_content_package()?;
    let content = package.content().clone();
    let product = ProductSession::from_package(CurrentDataSet::embedded()?, package)
        .map_err(|error| std::io::Error::other(format!("product session: {error:?}")))?;
    let page = project_page(&content, &product.snapshot(), PageState::world().route())?;
    assert!(matches!(page, PageModel::World(_)));
    Ok(())
}
