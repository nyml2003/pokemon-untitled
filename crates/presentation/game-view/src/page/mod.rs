//! 玩家页面和基座页面的 UI 投影。

mod assets;
mod common;
mod foundation;
mod foundation_bag;
mod foundation_common;
mod foundation_journey;
mod foundation_trainer_card;
mod party;
mod pause;
mod pause_bag;
mod pause_trainer_card;
mod pokedex;
mod save_confirm;
mod shop;
mod world;

use game_page_model::{PageIntent, PageModel};
use punctum_ui::{UiBuildError, UiTree};

pub use assets::{
    page_party_pokemon_asset, page_pokedex_icon_asset, page_pokedex_pokemon_asset,
    page_world_player_asset, page_world_tile_asset,
};
pub use foundation::project_foundation;

/// 将渲染无关的页面模型投影为玩家页面 UI tree。
pub fn project_page_model(model: &PageModel) -> Result<UiTree<PageIntent>, UiBuildError> {
    project_page_model_with_notice(model, None)
}

/// 将页面模型与适配层提供的短反馈投影为玩家页面 UI tree。
pub fn project_page_model_with_notice(
    model: &PageModel,
    notice: Option<&str>,
) -> Result<UiTree<PageIntent>, UiBuildError> {
    match model {
        PageModel::World(world) => world::project_page_world(world, notice),
        PageModel::Pause(pause) => pause::project_page_pause(pause, notice),
        PageModel::Shop(shop) => shop::project_page_shop(shop, notice),
        PageModel::SaveConfirm(save) => save_confirm::project_page_save_confirm(save, notice),
    }
}
