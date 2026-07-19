use game_assets::AssetKey;
use game_data::CurrentDataSet;
use game_session::{GameCommand, GameSession};
use game_ui::{
    BattleUiState, PokedexAction, PokedexUiSnapshot, PresentationSnapshot, WorldAnimation,
};
use map_project::{
    AtomicTileId, CharacterAppearanceId, CompositeTile, CompositeTileId, MapActor, MapActorId,
    MapDirection, MapProjectId, TilePosition,
};
use map_render::AtomicTileAsset;
use punctum_gpu::PixelOffset;
use world_application::{Direction, WorldApplication};

use super::*;

fn map() -> (MapProject, AtomicTileCatalog) {
    map_with_npc(TilePosition::new(12, 7))
}

fn map_with_npc(npc_position: TilePosition) -> (MapProject, AtomicTileCatalog) {
    let atomic = AtomicTileId::new("ground").unwrap();
    let material = CompositeTile::new(
        CompositeTileId::new("ground").unwrap(),
        vec![atomic.clone()],
    );
    let mut project = MapProject::blank(MapProjectId::new("test").unwrap(), 24, 16, Some(material));
    project.actors.push(MapActor::new(
        MapActorId::new("guide").unwrap(),
        npc_position,
        MapDirection::Left,
        CharacterAppearanceId::new("dppt/000").unwrap(),
    ));
    let catalog = AtomicTileCatalog::new([AtomicTileAsset {
        id: atomic,
        asset: AssetKey::new("map/tile/ground").unwrap(),
    }])
    .unwrap();
    (project, catalog)
}

fn presentation(offset: PixelOffset) -> PresentationSnapshot {
    PresentationSnapshot {
        battle_ui: BattleUiState::default(),
        pokedex: None,
        world_animation: WorldAnimation::Walk,
        sprite_frame: 1,
        world_pixel_offset: offset,
    }
}

#[test]
fn world_motion_keeps_the_player_centered_and_npcs_attached_to_the_map() {
    let (project, catalog) = map();
    let game = GameSession::new(
        CurrentDataSet::embedded().unwrap(),
        WorldApplication::from_map_project(&project).unwrap(),
        17,
    )
    .unwrap();
    let projected = project_scene(SceneViewInput {
        game: &game.snapshot(),
        presentation: presentation(PixelOffset::new(12, -6)),
        console: None,
        pokedex: &PokedexData::embedded_gen3().unwrap(),
        map_project: &project,
        map_catalog: &catalog,
        viewport: game_viewport(PixelSize::new(960, 720)),
    })
    .unwrap();
    let SceneFrame::Grid(view) = projected.frame else {
        panic!("world uses the grid path")
    };
    let map_layer = &view.layers()[0];
    let character_layer = &view.layers()[1];
    assert!(
        map_layer
            .images
            .iter()
            .any(|image| image.pixel_offset == PixelOffset::new(-12, 6))
    );
    let player = character_layer
        .images
        .iter()
        .find(|image| image.asset.as_str().starts_with("character/red/"))
        .unwrap();
    assert_eq!(player.pixel_offset, PixelOffset::new(0, 0));
    let npc = character_layer
        .images
        .iter()
        .find(|image| image.asset.as_str().starts_with("character/dppt/000/"))
        .unwrap();
    assert_eq!(npc.asset.as_str(), "character/dppt/000/1/0");
    assert_eq!(npc.pixel_offset, PixelOffset::new(-12, 6));
}

#[test]
fn pokedex_scene_keeps_typed_actions_after_layout() {
    let (project, catalog) = map();
    let game = GameSession::new(
        CurrentDataSet::embedded().unwrap(),
        WorldApplication::from_map_project(&project).unwrap(),
        17,
    )
    .unwrap();
    let mut view = presentation(PixelOffset::new(0, 0));
    view.pokedex = Some(PokedexUiSnapshot { selected_index: 0 });
    let projected = project_scene(SceneViewInput {
        game: &game.snapshot(),
        presentation: view,
        console: None,
        pokedex: &PokedexData::embedded_gen3().unwrap(),
        map_project: &project,
        map_catalog: &catalog,
        viewport: game_viewport(PixelSize::new(960, 720)),
    })
    .unwrap();
    let SceneFrame::Pokedex(frame) = projected.frame else {
        panic!("Pokedex keeps its typed UI frame")
    };
    assert!(matches!(
        frame.action_hits()[0].action,
        PokedexAction::SelectEntry { index: 0 }
    ));
}

#[test]
fn offscreen_npcs_are_not_sent_to_the_grid_frame() {
    let (project, catalog) = map_with_npc(TilePosition::new(23, 7));
    let game = GameSession::new(
        CurrentDataSet::embedded().unwrap(),
        WorldApplication::from_map_project(&project).unwrap(),
        17,
    )
    .unwrap();
    let projected = project_scene(SceneViewInput {
        game: &game.snapshot(),
        presentation: presentation(PixelOffset::new(0, 0)),
        console: None,
        pokedex: &PokedexData::embedded_gen3().unwrap(),
        map_project: &project,
        map_catalog: &catalog,
        viewport: game_viewport(PixelSize::new(960, 720)),
    })
    .unwrap();
    let SceneFrame::Grid(view) = projected.frame else {
        panic!("world uses the grid path")
    };
    assert!(
        view.layers()[1]
            .images
            .iter()
            .all(|image| !image.asset.as_str().starts_with("character/dppt/000/"))
    );
}

#[test]
fn battle_and_viewport_projection_are_deterministic() {
    let (project, catalog) = map();
    let mut game = GameSession::new_demo(CurrentDataSet::embedded().unwrap(), 17).unwrap();
    for _ in 0..4 {
        let (next, result) = game.transition(GameCommand::StepWorld(Direction::Right));
        result.unwrap();
        game = next;
    }
    assert_eq!(game.snapshot().scene(), GameScene::Battle);
    let projected = project_scene(SceneViewInput {
        game: &game.snapshot(),
        presentation: presentation(PixelOffset::new(0, 0)),
        console: None,
        pokedex: &PokedexData::embedded_gen3().unwrap(),
        map_project: &project,
        map_catalog: &catalog,
        viewport: game_viewport(PixelSize::new(1000, 720)),
    })
    .unwrap();
    assert_eq!(projected.viewport.cell_size, PixelSize::new(30, 30));
    assert_eq!(projected.viewport.origin, PixelOffset::new(20, 0));
    let SceneFrame::Ui(frame) = projected.frame else {
        panic!("battle uses the pixel UI path")
    };
    assert!(frame.commands().len() > 12);
    assert_eq!(frame.action_hits().len(), 4);

    let tiny = game_viewport(PixelSize::new(1, 1));
    assert_eq!(tiny.cell_size, PixelSize::new(1, 1));
}
