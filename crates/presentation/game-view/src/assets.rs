//! 游戏视图使用的资源 key 和战斗精灵资源组。

use super::common::ViewImage;
use battle_session::{MoveCategory, PokemonType};
use game_assets::AssetKey;
use punctum_gpu::Rgba8;
use punctum_grid::{GridPos, GridRect, GridSize};
pub(crate) fn pokedex_type(id: u16) -> Option<PokemonType> {
    Some(match id {
        1 => PokemonType::Normal,
        2 => PokemonType::Fighting,
        3 => PokemonType::Flying,
        4 => PokemonType::Poison,
        5 => PokemonType::Ground,
        6 => PokemonType::Rock,
        7 => PokemonType::Bug,
        8 => PokemonType::Ghost,
        9 => PokemonType::Steel,
        10 => PokemonType::Fire,
        11 => PokemonType::Water,
        12 => PokemonType::Grass,
        13 => PokemonType::Electric,
        14 => PokemonType::Psychic,
        15 => PokemonType::Ice,
        16 => PokemonType::Dragon,
        17 => PokemonType::Dark,
        _ => return None,
    })
}

/// 返回指定宝可梦属性的战斗图标资源键。
pub fn type_icon_asset(pokemon_type: PokemonType) -> AssetKey {
    let name = match pokemon_type {
        PokemonType::Normal => "normal",
        PokemonType::Fighting => "fighting",
        PokemonType::Flying => "flying",
        PokemonType::Poison => "poison",
        PokemonType::Ground => "ground",
        PokemonType::Rock => "rock",
        PokemonType::Bug => "bug",
        PokemonType::Ghost => "ghost",
        PokemonType::Steel => "steel",
        PokemonType::Fire => "fire",
        PokemonType::Water => "water",
        PokemonType::Grass => "grass",
        PokemonType::Electric => "electric",
        PokemonType::Psychic => "psychic",
        PokemonType::Ice => "ice",
        PokemonType::Dragon => "dragon",
        PokemonType::Dark => "dark",
    };
    AssetKey::from_resource_template(format!("ui/battle/type/{name}"))
}

pub(crate) fn move_category_icon_image(col: u32, row: u32, category: MoveCategory) -> ViewImage {
    ViewImage::new(
        GridRect::new(GridPos::new(col as i32, row as i32), GridSize::new(2, 1)),
        move_category_icon_asset(category),
        Rgba8::new(255, 255, 255, 255),
        20,
    )
}

/// 返回指定招式分类的战斗图标资源键。
pub fn move_category_icon_asset(category: MoveCategory) -> AssetKey {
    let name = match category {
        MoveCategory::Physical => "physical",
        MoveCategory::Special => "special",
        MoveCategory::Status => "status",
    };
    AssetKey::from_resource_template(format!("ui/battle/move-category/{name}"))
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// 一场战斗中双方精灵资源键的两个动画帧。
pub struct BattleSpriteResources {
    pub(crate) own: [AssetKey; 2],
    pub(crate) opponent: [AssetKey; 2],
}

impl BattleSpriteResources {
    /// 为双方队伍槽位生成两个动画帧的精灵资源键。
    pub fn for_slots(own_slot: usize, opponent_slot: usize) -> Self {
        Self {
            own: [
                player_back_asset(own_slot, 0),
                player_back_asset(own_slot, 1),
            ],
            opponent: [
                opponent_front_asset(opponent_slot, 0),
                opponent_front_asset(opponent_slot, 1),
            ],
        }
    }
}

/// 返回玩家后视精灵的资源键。
/// `frame` 按两个动画帧循环取模。
pub fn player_back_asset(slot: usize, frame: usize) -> AssetKey {
    AssetKey::from_resource_template(format!("battle/player/{slot}/back/{}", frame % 2))
}

/// 返回对手正视精灵的资源键。
/// `frame` 按两个动画帧循环取模。
pub fn opponent_front_asset(slot: usize, frame: usize) -> AssetKey {
    AssetKey::from_resource_template(format!("battle/opponent/{slot}/front/{}", frame % 2))
}

/// 返回队伍宝可梦图标的资源键。
/// `frame` 按两个动画帧循环取模。
pub fn pokemon_icon_asset(slot: usize, frame: usize) -> AssetKey {
    AssetKey::from_resource_template(format!("battle/team/{slot}/icon/{}", frame % 2))
}

/// 返回圆角矩形 UI 资源键。
pub fn rounded_ui_asset() -> AssetKey {
    AssetKey::from_resource_template("ui/rounded-rect".into())
}

/// 返回胶囊形 UI 资源键。
pub fn pill_ui_asset() -> AssetKey {
    AssetKey::from_resource_template("ui/pill".into())
}
