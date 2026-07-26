use super::*;
use game_page_model::PokedexEntryModel;
use game_ui_kit::column as ui_column;
use punctum_ui::{
    CrossAlign, Dimension, MainAlign, Position, UiBuildError, UiKey, UiNode, UiSize, UiStyle,
};

const WHEEL_STAGE_WIDTH: u32 = 560;
const WHEEL_STAGE_HEIGHT: u32 = 720;
const FOCUS_ICON_CENTER_Y: u32 = 330;
const UPPER_ADJACENT_CENTER_Y: u32 = 80;
const LOWER_ADJACENT_CENTER_Y: u32 = 634;
const WHEEL_VISIBLE_NEIGHBORS: usize = 1;
const ADJACENT_ICON_SIZE: u32 = 160;
pub(super) const FOCUS_ICON_SIZE: u32 = 320;
const FOCUS_NAME_HEIGHT: u32 = 32;
const FOCUS_NUMBER_HEIGHT: u32 = 20;
const FOCUS_CONTENT_GAP: u32 = 4;

pub(super) fn project(
    pokedex: &PokedexPageModel,
    visible_indices: &[usize],
    wheel_position: i32,
    interactive: bool,
    hide_transition_icons: bool,
) -> Result<UiNode<PageIntent>, UiBuildError> {
    let entries = visible_entries(visible_indices, wheel_position)
        .into_iter()
        .filter_map(|(display_index, index)| {
            pokedex
                .entries
                .get(index)
                .map(|entry| (display_index, entry))
        })
        .map(|(display_index, entry)| {
            wheel_entry(
                display_index,
                entry,
                wheel_position,
                interactive,
                hide_transition_icons,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ui_column(
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            main_align: MainAlign::Center,
            cross_align: CrossAlign::Center,
            ..UiStyle::default()
        },
        [ui_stack(
            UiStyle {
                width: Dimension::Px(WHEEL_STAGE_WIDTH),
                height: Dimension::Px(WHEEL_STAGE_HEIGHT),
                clip: true,
                ..UiStyle::default()
            },
            entries,
        )],
    ))
}

pub(super) fn visible_entries(
    visible_indices: &[usize],
    wheel_position: i32,
) -> Vec<(usize, usize)> {
    let center = wheel_center_index(wheel_position, visible_indices.len());
    let start = center.saturating_sub(WHEEL_VISIBLE_NEIGHBORS);
    let end = center
        .saturating_add(WHEEL_VISIBLE_NEIGHBORS + 1)
        .min(visible_indices.len());
    (start..end)
        .filter_map(|display_index| {
            visible_indices
                .get(display_index)
                .copied()
                .map(|index| (display_index, index))
        })
        .collect()
}

pub(super) fn transition_icon_geometry(
    index: usize,
    wheel_position: i32,
    viewport: UiSize,
) -> (u32, u32, u32) {
    let distance = wheel_distance(index, wheel_position);
    let icon_size = icon_size(distance);
    let (stage_x, stage_y) = stage_origin(viewport);
    let center_x = stage_x.saturating_add(WHEEL_STAGE_WIDTH / 2);
    let center_y =
        stage_y.saturating_add(icon_top(distance, icon_size).saturating_add(icon_size / 2));
    (icon_size, center_x, center_y)
}

fn stage_origin(viewport: UiSize) -> (u32, u32) {
    (
        viewport.width.saturating_sub(WHEEL_STAGE_WIDTH) / 2,
        viewport.height.saturating_sub(WHEEL_STAGE_HEIGHT) / 2,
    )
}

fn wheel_entry(
    index: usize,
    entry: &PokedexEntryModel,
    wheel_position: i32,
    interactive: bool,
    hide_transition_icons: bool,
) -> Result<UiNode<PageIntent>, UiBuildError> {
    let distance = wheel_distance(index, wheel_position);
    let icon_size = icon_size(distance);
    let show_focus_details = is_focus(distance);
    let content = wheel_entry_content(entry, icon_size, show_focus_details, hide_transition_icons);
    let node = UiNode::auto()
        .with_key(UiKey::new(format!(
            "page-pokedex-index-{}",
            entry.number.value()
        ))?)
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(content_height(icon_size, show_focus_details)),
            position: Position::Absolute {
                left: 0,
                top: icon_top(distance, icon_size),
            },
            ..UiStyle::default()
        })
        .with_children([content]);
    Ok(if interactive {
        node.with_action(PageIntent::SelectPokedexEntry(entry.number))
    } else {
        node
    })
}

fn wheel_entry_content(
    entry: &PokedexEntryModel,
    icon_size: u32,
    show_focus_details: bool,
    hide_transition_icons: bool,
) -> UiNode<PageIntent> {
    let style = UiStyle {
        width: Dimension::Fill,
        height: Dimension::Px(content_height(icon_size, show_focus_details)),
        gap: FOCUS_CONTENT_GAP,
        main_align: MainAlign::Start,
        cross_align: CrossAlign::Center,
        ..UiStyle::default()
    };
    let icon = focus_icon(entry, icon_size, hide_transition_icons);
    if show_focus_details {
        ui_column(
            style,
            [
                icon,
                compact_text_node(
                    name_or_unknown(entry.name.as_deref()),
                    TextTone::Default,
                    26,
                ),
                dex_number_text_node(entry.number, true, 14),
            ],
        )
    } else {
        ui_column(style, [icon])
    }
}

fn content_height(icon_size: u32, show_focus_details: bool) -> u32 {
    if show_focus_details {
        icon_size
            .saturating_add(FOCUS_NAME_HEIGHT)
            .saturating_add(FOCUS_NUMBER_HEIGHT)
            .saturating_add(FOCUS_CONTENT_GAP.saturating_mul(2))
    } else {
        icon_size
    }
}

fn is_focus(distance: i32) -> bool {
    distance.unsigned_abs() < (POKEDEX_MOTION_STEP as u32 / 2)
}

fn icon_size(distance: i32) -> u32 {
    let distance = distance.unsigned_abs();
    let step = POKEDEX_MOTION_STEP as u32;
    if distance <= step {
        interpolate_size(FOCUS_ICON_SIZE, ADJACENT_ICON_SIZE, distance, step)
    } else {
        ADJACENT_ICON_SIZE
    }
}

fn icon_top(distance: i32, icon_size: u32) -> u32 {
    let distance_from_focus = distance.unsigned_abs();
    let step = POKEDEX_MOTION_STEP as u32;
    let adjacent_center = if distance < 0 {
        UPPER_ADJACENT_CENTER_Y
    } else {
        LOWER_ADJACENT_CENTER_Y
    };
    let center = if distance_from_focus <= step {
        interpolate_size(
            FOCUS_ICON_CENTER_Y,
            adjacent_center,
            distance_from_focus,
            step,
        )
    } else {
        adjacent_center
    };
    center.saturating_sub(icon_size / 2)
}

fn interpolate_size(start: u32, end: u32, progress: u32, total: u32) -> u32 {
    let total = total.max(1);
    if start >= end {
        start.saturating_sub(start.saturating_sub(end).saturating_mul(progress) / total)
    } else {
        start.saturating_add(end.saturating_sub(start).saturating_mul(progress) / total)
    }
}

fn focus_icon(entry: &PokedexEntryModel, icon_size: u32, hide_icon: bool) -> UiNode<PageIntent> {
    if hide_icon {
        UiNode::auto().with_style(UiStyle::fixed(icon_size, icon_size))
    } else {
        pokedex_icon(entry.number.value(), icon_size, icon_size, entry.known)
    }
}
