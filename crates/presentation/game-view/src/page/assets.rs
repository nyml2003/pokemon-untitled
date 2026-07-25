use game_assets::AssetKey;
use game_page_model::PageIntent;
use game_ui_kit::{column as ui_column, image as ui_image, row as ui_row};
use punctum_ui::{Dimension, UiContentId, UiNode, UiStyle};

/// 页面 demo 使用的世界角色资源槽位。
pub fn page_world_player_asset() -> AssetKey {
    AssetKey::from_resource_template("page/world/player".into())
}

/// 页面 demo 使用的地图装饰 tile 资源槽位。
pub fn page_world_tile_asset(tile: u16) -> AssetKey {
    AssetKey::from_resource_template(format!("page/world/tile/{tile:04}"))
}

pub(super) fn world_tree_image() -> UiNode<PageIntent> {
    let rows = [[8_u16, 9], [10, 11], [12, 13]]
        .into_iter()
        .map(|row| {
            ui_row(
                UiStyle {
                    width: Dimension::Px(128),
                    height: Dimension::Px(64),
                    ..UiStyle::default()
                },
                row.into_iter().map(|tile| {
                    ui_image(
                        UiContentId::from_resource_key(page_world_tile_asset(tile).as_str()),
                        UiStyle::fixed(64, 64),
                    )
                }),
            )
        })
        .collect::<Vec<_>>();
    ui_column(
        UiStyle {
            width: Dimension::Px(128),
            height: Dimension::Px(192),
            ..UiStyle::default()
        },
        rows,
    )
}

/// 将当前页面 demo 中已有的队伍物种映射到可替换的资源槽位。
pub fn page_party_pokemon_asset(species: &str) -> Option<AssetKey> {
    match species {
        "Treecko" => Some(AssetKey::from_resource_template(
            "page/party/treecko".into(),
        )),
        _ => None,
    }
}

/// 返回已有图鉴立绘对应的页面资源槽位。
pub fn page_pokedex_pokemon_asset(number: u16) -> Option<AssetKey> {
    (1..=386)
        .contains(&number)
        .then(|| AssetKey::from_resource_template(format!("pokedex/{number}")))
}

/// 返回图鉴索引使用的宝可梦图标资源槽位。
pub fn page_pokedex_icon_asset(number: u16) -> Option<AssetKey> {
    (1..=386)
        .contains(&number)
        .then(|| AssetKey::from_resource_template(format!("pokedex-icon/{number}")))
}
