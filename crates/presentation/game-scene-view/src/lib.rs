//! Pure projection from product and presentation snapshots to an explicit layered game view.

#![forbid(unsafe_code)]

use game_data::PokedexData;
use game_session::{GameScene, GameSnapshot};
use game_ui::{CommandConsoleView, PokedexAction, PresentationSnapshot};
use game_view::{
    BattleSpriteResources, CANVAS_HEIGHT, CANVAS_WIDTH, GameView, compose_world, project_battle_ui,
    project_console_ui, project_pokedex,
};
use map_project::MapProject;
use map_render::{
    AtomicTileCatalog, MapCamera, MapGridLayout, MapRenderError, MapRenderInput, project_map,
};
use punctum_gpu::{PixelOffset, PixelSize, Viewport};
use punctum_grid::{GridPos, GridSize};
use punctum_ui::{UiBuildError, UiFrame, UiLayoutError, UiSize};
use std::{error::Error, fmt};

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
    pub frame: SceneFrame,
    pub viewport: Viewport,
}

pub enum SceneFrame {
    Grid(GameView),
    Ui(UiFrame),
    Pokedex(UiFrame<PokedexAction>),
    GridWithUi {
        base: GameView,
        overlay: UiFrame,
    },
    UiWithUi {
        base: UiFrame,
        overlay: UiFrame,
    },
    PokedexWithUi {
        base: UiFrame<PokedexAction>,
        overlay: UiFrame,
    },
}

#[derive(Debug)]
pub enum SceneViewError {
    Map(MapRenderError),
    UiBuild(UiBuildError),
    UiLayout(UiLayoutError),
}

impl fmt::Display for SceneViewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Map(error) => write!(formatter, "map projection failed: {error}"),
            Self::UiBuild(error) => write!(formatter, "Pokedex UI construction failed: {error}"),
            Self::UiLayout(error) => write!(formatter, "Pokedex UI layout failed: {error}"),
        }
    }
}
impl Error for SceneViewError {}

pub fn project_scene(input: SceneViewInput<'_>) -> Result<ProjectedScene, SceneViewError> {
    let viewport = input.viewport;
    let ui_size = UiSize::new(viewport.target_size.width, viewport.target_size.height);
    let console = match input.console {
        Some(console) => Some(
            project_console_ui(console)
                .map_err(SceneViewError::UiBuild)?
                .resolve(ui_size)
                .map_err(SceneViewError::UiLayout)?,
        ),
        None => None,
    };
    let frame = if let Some(pokedex) = input.presentation.pokedex {
        let base = project_pokedex(input.pokedex, pokedex.selected_index)
            .map_err(SceneViewError::UiBuild)?
            .resolve(ui_size)
            .map_err(SceneViewError::UiLayout)?;
        match console {
            Some(overlay) => SceneFrame::PokedexWithUi { base, overlay },
            None => SceneFrame::Pokedex(base),
        }
    } else {
        let base = match input.game.scene() {
            GameScene::World => {
                let camera = world_camera(input.game.world().player());
                let map_pixel_offset = invert_pixel_offset(input.presentation.world_pixel_offset);
                let scene = project_map(MapRenderInput {
                    project: input.map_project,
                    catalog: input.map_catalog,
                    camera,
                    pixel_offset: map_pixel_offset,
                    viewport,
                    layout: MapGridLayout::new(
                        GridSize::new(CANVAS_WIDTH, CANVAS_HEIGHT),
                        GridSize::new(2, 2),
                    ),
                })
                .map_err(SceneViewError::Map)?;
                compose_world(
                    scene.into_layer(),
                    GridPos::new(camera.col, camera.row),
                    input.game.world(),
                    input.presentation.world_animation,
                    input.presentation.sprite_frame,
                    map_pixel_offset,
                    None,
                )
            }
            GameScene::Battle => {
                let battle = input.game.battle().expect("battle scene owns a battle");
                let base = project_battle_ui(
                    battle.session(),
                    input.presentation.battle_ui,
                    BattleSpriteResources::for_slots(
                        battle.own_sprite_slot(),
                        battle.opponent_sprite_slot(),
                    ),
                    input.presentation.sprite_frame,
                )
                .map_err(SceneViewError::UiBuild)?
                .resolve(ui_size)
                .map_err(SceneViewError::UiLayout)?;
                return Ok(ProjectedScene {
                    frame: match console {
                        Some(overlay) => SceneFrame::UiWithUi { base, overlay },
                        None => SceneFrame::Ui(base),
                    },
                    viewport,
                });
            }
        };
        match console {
            Some(overlay) => SceneFrame::GridWithUi { base, overlay },
            None => SceneFrame::Grid(base),
        }
    };
    Ok(ProjectedScene { frame, viewport })
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
        let mut project =
            MapProject::blank(MapProjectId::new("test").unwrap(), 24, 16, Some(material));
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
}
