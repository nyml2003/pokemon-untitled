//! 纯 Gen3 产品视图投影。
//!
//! 该入口只组织内部投影模块，并保持 game-view 的稳定根级 API。

#[path = "assets.rs"]
mod assets;
#[path = "battle_ui.rs"]
mod battle_ui;
#[path = "common.rs"]
mod common;
#[path = "page/mod.rs"]
mod page;
#[path = "scene.rs"]
mod scene;

pub use assets::{
    BattleSpriteResources, move_category_icon_asset, opponent_front_asset, pill_ui_asset,
    player_back_asset, pokemon_icon_asset, rounded_ui_asset, type_icon_asset,
};
pub use battle_ui::{project_battle_ui, project_console_ui, project_pokedex};
pub use common::{
    BattleAnimation, CANVAS_HEIGHT, CANVAS_WIDTH, FoundationPage, FoundationPageAction, GameView,
    LayerKind, ProjectionError, TextLabel, TextRole, ViewCell, ViewImage, ViewLayer,
};
pub use page::{
    page_party_pokemon_asset, page_pokedex_icon_asset, page_pokedex_pokemon_asset,
    page_world_player_asset, page_world_tile_asset, project_foundation, project_page_model,
    project_page_model_with_notice, project_page_model_with_visual_state,
};
pub use scene::{
    compose_world, project_battle, project_console, project_world, project_world_animated,
    project_world_presented, with_console, world_character_asset,
};

#[cfg(test)]
pub(crate) use common::{HP_LOW, SPEECH_BUBBLE, TEXT};
#[cfg(test)]
pub(crate) use punctum_ui::UiColor;
pub(crate) use scene::{
    active_pokemon, battle_animation, battle_message, creature_tint, prompt_data,
    visible_console_start,
};
#[cfg(test)]
pub(crate) use scene::{draw_hp_bar, effectiveness_message, outcome_message, used_move_name};

#[cfg(test)]
#[path = "../tests/unit/projection.rs"]
mod tests;
