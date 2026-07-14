//! Pure projection from product and presentation snapshots to an explicit layered game view.

#![forbid(unsafe_code)]

use game_data::PokedexData;
use game_session::{GameScene, GameSnapshot};
use game_ui::{CommandConsoleView, PresentationSnapshot};
use game_view::{
    BattleSpriteResources, CANVAS_HEIGHT, CANVAS_WIDTH, GameView, compose_world, project_battle,
    project_pokedex, with_console,
};
use map_project::MapProject;
use map_render::{
    AtomicTileCatalog, MapCamera, MapGridLayout, MapRenderError, MapRenderInput, project_map,
};
use punctum_gpu::{PixelOffset, PixelSize, Viewport};
use punctum_grid::{GridPos, GridSize};

pub struct SceneViewInput<'a> {
    pub game: &'a GameSnapshot,
    pub presentation: PresentationSnapshot,
    pub console: Option<&'a CommandConsoleView>,
    pub pokedex: &'a PokedexData,
    pub map_project: &'a MapProject,
    pub map_catalog: &'a AtomicTileCatalog,
    pub viewport: Viewport,
}

pub struct ProjectedScene {
    pub view: GameView,
    pub viewport: Viewport,
}

pub fn project_scene(input: SceneViewInput<'_>) -> Result<ProjectedScene, MapRenderError> {
    let viewport = input.viewport;
    let view = if let Some(pokedex) = input.presentation.pokedex {
        project_pokedex(input.pokedex, pokedex.selected_index)
    } else {
        match input.game.scene() {
            GameScene::World => {
                let camera = world_camera(input.game.world().player());
                let scene = project_map(MapRenderInput {
                    project: input.map_project,
                    catalog: input.map_catalog,
                    camera,
                    pixel_offset: invert_pixel_offset(input.presentation.world_pixel_offset),
                    viewport,
                    layout: MapGridLayout::new(
                        GridSize::new(CANVAS_WIDTH, CANVAS_HEIGHT),
                        GridSize::new(2, 2),
                    ),
                })?;
                compose_world(
                    scene.into_layer(),
                    GridPos::new(camera.col, camera.row),
                    input.game.world(),
                    input.presentation.world_animation,
                    input.presentation.sprite_frame,
                    input.console,
                )
            }
            GameScene::Battle => {
                let battle = input.game.battle().expect("battle scene owns a battle");
                let battle_view = project_battle(
                    battle.session(),
                    input.presentation.battle_ui,
                    BattleSpriteResources::for_slots(
                        battle.own_sprite_slot(),
                        battle.opponent_sprite_slot(),
                    ),
                    input.presentation.sprite_frame,
                );
                with_console(battle_view.layers().to_vec(), input.console)
            }
        }
    };
    Ok(ProjectedScene { view, viewport })
}

pub fn game_viewport(target_size: PixelSize) -> Viewport {
    let cell_size = (target_size.width / CANVAS_WIDTH)
        .min(target_size.height / CANVAS_HEIGHT)
        .max(1);
    let width = i64::from(CANVAS_WIDTH) * i64::from(cell_size);
    let height = i64::from(CANVAS_HEIGHT) * i64::from(cell_size);
    Viewport::new(
        target_size,
        PixelOffset::new(
            ((i64::from(target_size.width) - width) / 2) as i32,
            ((i64::from(target_size.height) - height) / 2) as i32,
        ),
        PixelSize::new(cell_size, cell_size),
    )
    .expect("the game viewport always has a positive integer cell size")
}

fn world_camera(player: world_application::Position) -> MapCamera {
    const VIEW_COLS: u16 = 16;
    const VIEW_ROWS: u16 = 12;
    MapCamera::new(
        i32::from(player.x()) - i32::from(VIEW_COLS / 2),
        i32::from(player.y()) - i32::from(VIEW_ROWS / 2),
    )
}

const fn invert_pixel_offset(offset: PixelOffset) -> PixelOffset {
    PixelOffset::new(-offset.x, -offset.y)
}

#[cfg(test)]
mod tests {
    use game_assets::AssetKey;
    use game_data::CurrentDataSet;
    use game_session::{GameCommand, GameSession};
    use game_ui::{BattleUiState, PresentationSnapshot, WorldAnimation};
    use map_project::{AtomicTileId, CompositeTile, CompositeTileId, MapProjectId};
    use map_render::AtomicTileAsset;
    use punctum_gpu::PixelOffset;
    use world_application::{Direction, WorldApplication};

    use super::*;

    fn map() -> (MapProject, AtomicTileCatalog) {
        let atomic = AtomicTileId::new("ground").unwrap();
        let material = CompositeTile::new(
            CompositeTileId::new("ground").unwrap(),
            vec![atomic.clone()],
        );
        let project = MapProject::blank(MapProjectId::new("test").unwrap(), 24, 16, Some(material));
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
    fn world_motion_offsets_the_map_but_not_the_character() {
        let (project, catalog) = map();
        let game = GameSession::new(
            CurrentDataSet::embedded().unwrap(),
            WorldApplication::demo().unwrap(),
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
        let map_layer = &projected.view.layers()[0];
        let character_layer = &projected.view.layers()[1];
        assert!(
            map_layer
                .images
                .iter()
                .any(|image| image.pixel_offset == PixelOffset::new(-12, 6))
        );
        assert!(
            character_layer
                .images
                .iter()
                .all(|image| image.pixel_offset == PixelOffset::new(0, 0))
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
        let console = CommandConsoleView::default();
        let projected = project_scene(SceneViewInput {
            game: &game.snapshot(),
            presentation: presentation(PixelOffset::new(0, 0)),
            console: Some(&console),
            pokedex: &PokedexData::embedded_gen3().unwrap(),
            map_project: &project,
            map_catalog: &catalog,
            viewport: game_viewport(PixelSize::new(1000, 720)),
        })
        .unwrap();
        assert_eq!(projected.viewport.cell_size, PixelSize::new(30, 30));
        assert_eq!(projected.viewport.origin, PixelOffset::new(20, 0));
        assert_eq!(
            projected.view.layers().last().unwrap().kind,
            game_view::LayerKind::Console
        );

        let tiny = game_viewport(PixelSize::new(1, 1));
        assert_eq!(tiny.cell_size, PixelSize::new(1, 1));
    }
}
