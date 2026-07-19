//! Pure projection from product and presentation snapshots to an explicit layered game view.

#![forbid(unsafe_code)]

use game_data::PokedexData;
use game_session::{GameScene, GameSnapshot};
use game_ui::{CommandConsoleView, PokedexAction, PresentationSnapshot};
use game_view::{
    BattleSpriteResources, CANVAS_HEIGHT, CANVAS_WIDTH, GameView, ProjectionError, compose_world,
    project_battle_ui, project_console_ui, project_pokedex,
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
    Projection(ProjectionError),
    InconsistentBattleScene,
    UiBuild(UiBuildError),
    UiLayout(UiLayoutError),
}

impl fmt::Display for SceneViewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Map(error) => write!(formatter, "map projection failed: {error}"),
            Self::Projection(error) => write!(formatter, "game view projection failed: {error}"),
            Self::InconsistentBattleScene => {
                formatter.write_str("battle scene is missing its battle session")
            }
            Self::UiBuild(error) => write!(formatter, "Pokedex UI construction failed: {error}"),
            Self::UiLayout(error) => write!(formatter, "Pokedex UI layout failed: {error}"),
        }
    }
}
impl Error for SceneViewError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Map(error) => Some(error),
            Self::Projection(error) => Some(error),
            Self::UiBuild(error) => Some(error),
            Self::UiLayout(error) => Some(error),
            Self::InconsistentBattleScene => None,
        }
    }
}

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
                .map_err(SceneViewError::Projection)?
            }
            GameScene::Battle => {
                let battle = input
                    .game
                    .battle()
                    .ok_or(SceneViewError::InconsistentBattleScene)?;
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
    Viewport {
        target_size,
        origin: PixelOffset::new(
            ((i64::from(target_size.width) - width) / 2) as i32,
            ((i64::from(target_size.height) - height) / 2) as i32,
        ),
        cell_size: PixelSize::new(cell_size, cell_size),
    }
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
#[path = "../tests/unit/lib.rs"]
mod tests;
