//! 将游戏、战斗、图鉴和控制台快照投影为 Gen3 风格视图。
//!
//! 该 crate 只生成 `GameView`、像素 UI 树和资源键。
//! 它不持有游戏状态、不读取资源，也不提交渲染命令。

#![forbid(unsafe_code)]

mod projection;

pub use projection::{
    BattleAnimation, BattleSpriteResources, CANVAS_HEIGHT, CANVAS_WIDTH, GameView, LayerKind,
    TextLabel, TextRole, ViewCell, ViewImage, ViewLayer, compose_world, move_category_icon_asset,
    opponent_front_asset, pill_ui_asset, player_back_asset, pokemon_icon_asset, project_battle,
    project_battle_ui, project_console, project_console_ui, project_pokedex, project_world,
    project_world_animated, project_world_presented, rounded_ui_asset, type_icon_asset,
    with_console, world_character_asset,
};
