use super::super::assets::{move_category_icon_asset, type_icon_asset};
use super::super::common::POKEDEX_THEME;
use super::assets::page_pokedex_icon_asset;
use super::common::page_notice;
use battle_session::{MoveCategory, PokemonType};
use game_assets::AssetKey;
use game_page_model::{
    NationalDexNumber, PageIntent, PokedexEntryModel, PokedexMoveCategory, PokedexMoveModel,
    PokedexPageModel,
};
use game_ui::{
    MoveFilterModel, PokedexDetailMode, PokedexFilterModel, PokedexFilterOverlay, PokedexScene,
    PokedexVisualState,
};
use game_ui_kit::{
    SpriteAppearance, TextTone, image as ui_image, row as ui_row, screen as ui_screen,
    sprite as ui_sprite, stack as ui_stack, text as ui_text,
};
use punctum_ui::{
    Dimension, Insets, Position, UiBuildError, UiColor, UiContentId, UiNode, UiPixelOffset, UiSize,
    UiStyle, UiTree,
};

mod browse;
mod filter;
mod moves;
mod profile;
mod rail;

const POKEDEX_MOTION_STEP: i32 = 1000;

pub(super) fn project_pause_pokedex(
    pokedex: &PokedexPageModel,
    notice: Option<&str>,
    visual: Option<PokedexVisualState>,
    viewport: UiSize,
) -> Result<UiTree<PageIntent>, UiBuildError> {
    let selected_index = selected_index(pokedex);
    let visual = visual.unwrap_or_else(|| PokedexVisualState {
        scene: PokedexScene::Browse,
        scene_position: 0,
        detail_mode: PokedexDetailMode::Facts,
        wheel_position: index_position(selected_index),
        visible_entry_indices: (0..pokedex.entries.len()).collect(),
        visible_move_indices: (0..pokedex.moves.len()).collect(),
        filter_overlay: PokedexFilterOverlay::Compact,
        pokedex_filter: PokedexFilterModel::default(),
        move_filter: MoveFilterModel::default(),
        pokedex_ability_cursor: 0,
        move_accuracy_cursor: 0,
        form_scroll_y: 0,
    });
    let shared_transition = is_browse_detail_transition(&visual);
    let selected_visible_index = selected_visible_index(pokedex, &visual.visible_entry_indices);
    let mut layers = vec![
        scene_layer(
            PokedexScene::Browse,
            &visual,
            viewport,
            browse::project(
                pokedex,
                &visual.visible_entry_indices,
                visual.wheel_position,
                visual.scene == PokedexScene::Browse,
                shared_transition,
            )?,
        ),
        scene_layer(
            PokedexScene::Detail,
            &visual,
            viewport,
            detail_scene(pokedex, &visual, selected_visible_index, shared_transition)?,
        ),
    ];
    layers.extend(shared_transition_icons(pokedex, &visual, viewport));
    if matches!(visual.filter_overlay, PokedexFilterOverlay::Compact) && !shared_transition {
        layers.push(
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Auto,
                    height: Dimension::Px(34),
                    position: Position::Absolute { left: 16, top: 16 },
                    ..UiStyle::default()
                })
                .with_children([filter::compact_entry(&visual)?]),
        );
    }
    if let Some(overlay) = filter::expanded(pokedex, &visual)? {
        layers.push(overlay);
    }
    UiTree::new(ui_screen(
        &POKEDEX_THEME,
        [
            ui_stack(
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    clip: true,
                    ..UiStyle::default()
                },
                layers,
            ),
            page_notice(notice),
        ],
    ))
}

fn detail_scene(
    pokedex: &PokedexPageModel,
    visual: &PokedexVisualState,
    selected_visible_index: usize,
    hide_transition_icons: bool,
) -> Result<UiNode<PageIntent>, UiBuildError> {
    let content = match visual.detail_mode {
        PokedexDetailMode::Facts => profile::project_content(pokedex),
        PokedexDetailMode::Moves => moves::project_content(
            pokedex,
            &visual.visible_move_indices,
            selected_visible_move_index(pokedex, &visual.visible_move_indices),
            visual.scene == PokedexScene::Detail,
        )?,
    };
    Ok(ui_row(
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            padding: Insets::all(28),
            gap: 28,
            ..UiStyle::default()
        },
        [
            rail::project(
                pokedex,
                &visual.visible_entry_indices,
                selected_visible_index,
                hide_transition_icons,
                !hide_transition_icons,
            )?,
            content,
        ],
    ))
}

fn is_browse_detail_transition(visual: &PokedexVisualState) -> bool {
    (1..POKEDEX_MOTION_STEP).contains(&visual.scene_position)
}

fn shared_transition_icons(
    pokedex: &PokedexPageModel,
    visual: &PokedexVisualState,
    viewport: UiSize,
) -> Vec<UiNode<PageIntent>> {
    if !is_browse_detail_transition(visual) {
        return Vec::new();
    }
    let progress = i64::from(visual.scene_position);
    let selected = selected_visible_index(pokedex, &visual.visible_entry_indices);
    browse::visible_entries(&visual.visible_entry_indices, visual.wheel_position)
        .into_iter()
        .filter_map(|(display_index, index)| {
            let entry = pokedex.entries.get(index)?;
            Some(shared_transition_icon(
                entry,
                display_index,
                selected,
                visual.wheel_position,
                progress,
                viewport,
            ))
        })
        .collect()
}

fn shared_transition_icon(
    entry: &PokedexEntryModel,
    index: usize,
    selected: usize,
    wheel_position: i32,
    progress: i64,
    viewport: UiSize,
) -> UiNode<PageIntent> {
    let (start_size, start_center_x, start_center_y) =
        browse::transition_icon_geometry(index, wheel_position, viewport);
    let relative_index = i64::try_from(index)
        .map_or(0, |index| index)
        .saturating_sub(i64::try_from(selected).map_or(0, |selected| selected));
    let icon_size = interpolate_u32(start_size, rail::SELECTED_ICON_SIZE, progress);
    let (rail_x, rail_y) = rail::icon_origin(viewport, relative_index);
    let end_center_x = rail_x.saturating_add(rail::SELECTED_ICON_SIZE / 2);
    let center_x = interpolate_u32(start_center_x, end_center_x, progress);
    let end_center_y = rail_y.saturating_add(rail::SELECTED_ICON_SIZE / 2);
    let center_y = interpolate_u32(start_center_y, end_center_y, progress);
    let left = center_x.saturating_sub(icon_size / 2);
    let top = center_y.saturating_sub(icon_size / 2);
    UiNode::auto()
        .with_style(UiStyle {
            width: Dimension::Px(icon_size),
            height: Dimension::Px(icon_size),
            position: Position::Absolute { left, top },
            ..UiStyle::default()
        })
        .with_children([pokedex_icon(
            entry.number.value(),
            icon_size,
            icon_size,
            entry.known,
        )])
}

fn interpolate_u32(start: u32, end: u32, progress: i64) -> u32 {
    let progress = progress.clamp(0, i64::from(POKEDEX_MOTION_STEP));
    let distance = i64::from(end) - i64::from(start);
    let value = i64::from(start) + distance * progress / i64::from(POKEDEX_MOTION_STEP);
    value.clamp(0, i64::from(u32::MAX)) as u32
}

fn scene_layer(
    scene: PokedexScene,
    visual: &PokedexVisualState,
    viewport: UiSize,
    child: UiNode<PageIntent>,
) -> UiNode<PageIntent> {
    let delta = i64::from(scene_position(scene)) - i64::from(visual.scene_position);
    let offset = (i64::from(viewport.width.max(1)) * delta / i64::from(POKEDEX_MOTION_STEP))
        .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
    UiNode::auto()
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            position: Position::Absolute { left: 0, top: 0 },
            visual_offset: UiPixelOffset::new(offset, 0),
            ..UiStyle::default()
        })
        .with_children([child])
}

const fn scene_position(scene: PokedexScene) -> i32 {
    match scene {
        PokedexScene::Browse => 0,
        PokedexScene::Detail => POKEDEX_MOTION_STEP,
    }
}

pub(super) fn selected_index(pokedex: &PokedexPageModel) -> usize {
    pokedex
        .entries
        .iter()
        .position(|entry| entry.number == pokedex.selected.number)
        .map_or(0, |index| index)
}

fn selected_visible_index(pokedex: &PokedexPageModel, visible_indices: &[usize]) -> usize {
    visible_indices
        .iter()
        .position(|index| {
            pokedex
                .entries
                .get(*index)
                .is_some_and(|entry| entry.number == pokedex.selected.number)
        })
        .unwrap_or(0)
}

fn selected_visible_move_index(pokedex: &PokedexPageModel, visible_indices: &[usize]) -> usize {
    visible_indices
        .iter()
        .position(|index| *index == pokedex.selected_move)
        .unwrap_or(0)
}

pub(super) fn index_position(index: usize) -> i32 {
    i32::try_from(index).map_or(i32::MAX, |index| index.saturating_mul(POKEDEX_MOTION_STEP))
}

pub(super) fn wheel_center_index(position: i32, entry_count: usize) -> usize {
    if entry_count == 0 {
        return 0;
    }
    let position = position.max(0) / POKEDEX_MOTION_STEP;
    usize::try_from(position).map_or(entry_count - 1, |index| index.min(entry_count - 1))
}

pub(super) fn wheel_distance(index: usize, position: i32) -> i32 {
    index_position(index).saturating_sub(position)
}

pub(super) fn pokedex_icon(
    number: u16,
    width: u32,
    height: u32,
    known: bool,
) -> UiNode<PageIntent> {
    let appearance = if known {
        SpriteAppearance::Plain
    } else {
        SpriteAppearance::Tinted(UiColor::new(28, 34, 45, 255))
    };
    match page_pokedex_icon_asset(number) {
        Some(asset) => ui_sprite(
            UiContentId::from_resource_key(asset.as_str()),
            UiStyle::fixed(width, height),
            appearance,
        ),
        None => UiNode::auto().with_style(UiStyle::fixed(width, height)),
    }
}

pub(super) fn type_icons(types: &[String]) -> Vec<UiNode<PageIntent>> {
    types
        .iter()
        .filter_map(|name| pokedex_type_asset(name))
        .map(|asset| {
            ui_image(
                UiContentId::from_resource_key(asset.as_str()),
                UiStyle::fixed(64, 32),
            )
        })
        .collect()
}

pub(super) fn move_category_asset(category: PokedexMoveCategory) -> AssetKey {
    let category = match category {
        PokedexMoveCategory::Physical => MoveCategory::Physical,
        PokedexMoveCategory::Special => MoveCategory::Special,
        PokedexMoveCategory::Status => MoveCategory::Status,
    };
    move_category_icon_asset(category)
}

pub(super) fn format_move_details(item: &PokedexMoveModel) -> String {
    let power = item
        .power
        .map_or_else(|| String::from("威力 --"), |value| format!("威力 {value}"));
    let accuracy = item
        .accuracy
        .map_or_else(|| String::from("命中 --"), |value| format!("命中 {value}%"));
    let pp = item
        .pp
        .map_or_else(|| String::from("PP --"), |value| format!("PP {value}"));
    format!("{}  {power}  {accuracy}  {pp}", item.move_type)
}

pub(super) fn number_text(number: NationalDexNumber) -> String {
    format!("NO.{:03}", number.value())
}

pub(super) fn dex_number_text_node(
    number: NationalDexNumber,
    complete: bool,
    size: u32,
) -> UiNode<PageIntent> {
    let content = if complete {
        number_text(number)
    } else {
        format!("{:03}", number.value())
    };
    let width = u32::try_from(content.chars().count())
        .map_or(u32::MAX, |count| count.saturating_mul(size))
        .max(1);
    ui_text(
        &POKEDEX_THEME,
        TextTone::Muted,
        content,
        size,
        Dimension::Px(width),
    )
}

pub(super) fn name_or_unknown(name: Option<&str>) -> &str {
    name.unwrap_or("???")
}

pub(super) fn text_node(
    content: impl Into<String>,
    tone: TextTone,
    size: u32,
) -> UiNode<PageIntent> {
    ui_text(&POKEDEX_THEME, tone, content, size, Dimension::Fill)
}

pub(super) fn compact_text_node(
    content: impl Into<String>,
    tone: TextTone,
    size: u32,
) -> UiNode<PageIntent> {
    let content = content.into();
    let width = content.chars().fold(0_u32, |width, character| {
        let character_width = if character.is_ascii() {
            size.saturating_mul(3).saturating_div(5).max(1)
        } else {
            size.max(1)
        };
        width.saturating_add(character_width)
    });
    ui_text(
        &POKEDEX_THEME,
        tone,
        content,
        size,
        Dimension::Px(width.max(1)),
    )
}

fn pokedex_type_asset(name: &str) -> Option<AssetKey> {
    let pokemon_type = match name {
        "一般" | "Normal" => PokemonType::Normal,
        "格斗" | "Fighting" => PokemonType::Fighting,
        "飞行" | "Flying" => PokemonType::Flying,
        "毒" | "Poison" => PokemonType::Poison,
        "地面" | "Ground" => PokemonType::Ground,
        "岩石" | "Rock" => PokemonType::Rock,
        "虫" | "Bug" => PokemonType::Bug,
        "幽灵" | "Ghost" => PokemonType::Ghost,
        "钢" | "Steel" => PokemonType::Steel,
        "火" | "Fire" => PokemonType::Fire,
        "水" | "Water" => PokemonType::Water,
        "草" | "Grass" => PokemonType::Grass,
        "电" | "Electric" => PokemonType::Electric,
        "超能力" | "Psychic" => PokemonType::Psychic,
        "冰" | "Ice" => PokemonType::Ice,
        "龙" | "Dragon" => PokemonType::Dragon,
        "恶" | "Dark" => PokemonType::Dark,
        _ => return None,
    };
    Some(type_icon_asset(pokemon_type))
}
