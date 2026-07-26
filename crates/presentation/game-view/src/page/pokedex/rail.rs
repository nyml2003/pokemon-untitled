use super::*;
use game_page_model::PokedexEntryModel;
use game_ui_kit::{column as ui_column, row as ui_row};
use punctum_ui::{
    CrossAlign, Dimension, Insets, MainAlign, UiBuildError, UiContent, UiKey, UiNode, UiSize,
    UiStyle,
};

const RAIL_WIDTH: u32 = 96;
const RAIL_ITEM_HEIGHT: u32 = 64;
const RAIL_SLOT_COUNT: u32 = 5;
const PAGE_PADDING: u32 = 28;
const RAIL_HORIZONTAL_PADDING: u32 = 4;
const RAIL_VERTICAL_PADDING: u32 = 2;
const RAIL_CONTENT_GAP: u32 = 4;
const RAIL_NUMBER_WIDTH: u32 = 33;
pub(super) const SELECTED_ICON_SIZE: u32 = 30;

pub(super) fn project(
    pokedex: &PokedexPageModel,
    visible_indices: &[usize],
    selected: usize,
    hide_transition_icons: bool,
    interactive: bool,
) -> Result<UiNode<PageIntent>, UiBuildError> {
    let entries = (0..5)
        .map(|slot| {
            rail_slot(
                pokedex,
                visible_indices,
                selected,
                slot,
                hide_transition_icons,
                interactive,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ui_column(
        UiStyle {
            width: Dimension::Px(RAIL_WIDTH),
            height: Dimension::Fill,
            main_align: MainAlign::Center,
            cross_align: CrossAlign::Center,
            ..UiStyle::default()
        },
        entries,
    ))
}

pub(super) fn icon_origin(viewport: UiSize, relative_index: i64) -> (u32, u32) {
    let content_width = viewport
        .width
        .saturating_sub(PAGE_PADDING.saturating_mul(2));
    let rail_width = content_width.min(RAIL_WIDTH);
    let slot_content_width = rail_width.saturating_sub(RAIL_HORIZONTAL_PADDING.saturating_mul(2));
    let row_width = SELECTED_ICON_SIZE
        .saturating_add(RAIL_CONTENT_GAP)
        .saturating_add(RAIL_NUMBER_WIDTH);
    let x = PAGE_PADDING
        .saturating_add(RAIL_HORIZONTAL_PADDING)
        .saturating_add(slot_content_width.saturating_sub(row_width) / 2);

    let available_height = viewport
        .height
        .saturating_sub(PAGE_PADDING.saturating_mul(2));
    let rail_height = RAIL_ITEM_HEIGHT.saturating_mul(RAIL_SLOT_COUNT);
    let rail_top = PAGE_PADDING.saturating_add(available_height.saturating_sub(rail_height) / 2);
    let slot = relative_index
        .saturating_add(2)
        .clamp(0, i64::from(RAIL_SLOT_COUNT - 1));
    let slot = u32::try_from(slot).map_or(0, |slot| slot);
    let slot_offset = slot.saturating_mul(RAIL_ITEM_HEIGHT);
    let slot_height = available_height
        .saturating_sub(slot_offset)
        .min(RAIL_ITEM_HEIGHT);
    let slot_content_height = slot_height.saturating_sub(RAIL_VERTICAL_PADDING.saturating_mul(2));
    let y = rail_top
        .saturating_add(slot_offset)
        .saturating_add(RAIL_VERTICAL_PADDING)
        .saturating_add(slot_content_height.saturating_sub(SELECTED_ICON_SIZE) / 2);
    (x, y)
}

fn rail_slot(
    pokedex: &PokedexPageModel,
    visible_indices: &[usize],
    selected: usize,
    slot: usize,
    hide_transition_icons: bool,
    interactive: bool,
) -> Result<UiNode<PageIntent>, UiBuildError> {
    let index = if slot < 2 {
        selected.checked_sub(2 - slot)
    } else {
        selected.checked_add(slot - 2)
    };
    let current = slot == 2;
    let entry = index
        .and_then(|display_index| visible_indices.get(display_index))
        .and_then(|index| pokedex.entries.get(*index));
    let hide_icon =
        hide_transition_icons && index.is_some_and(|index| index.abs_diff(selected) <= 1);
    let node = UiNode::auto()
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(RAIL_ITEM_HEIGHT),
            padding: Insets::symmetric(4, 2),
            main_align: MainAlign::Center,
            cross_align: CrossAlign::Center,
            ..UiStyle::default()
        })
        .with_children([rail_slot_content(entry, hide_icon)]);
    let node = if current {
        node.with_content(UiContent::Fill(POKEDEX_THEME.selected))
    } else {
        node
    };
    let Some(entry) = entry else {
        return Ok(node);
    };
    if interactive {
        return Ok(node
            .with_key(UiKey::new(format!(
                "page-pokedex-detail-{}",
                entry.number.value()
            ))?)
            .with_action(PageIntent::SelectPokedexEntry(entry.number)));
    }
    Ok(node)
}

fn rail_slot_content(entry: Option<&PokedexEntryModel>, hide_icon: bool) -> UiNode<PageIntent> {
    let Some(entry) = entry else {
        return UiNode::auto().with_style(UiStyle::fixed(1, 1));
    };
    let icon = if hide_icon {
        UiNode::auto().with_style(UiStyle::fixed(SELECTED_ICON_SIZE, SELECTED_ICON_SIZE))
    } else {
        pokedex_icon(
            entry.number.value(),
            SELECTED_ICON_SIZE,
            SELECTED_ICON_SIZE,
            entry.known,
        )
    };
    ui_row(
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            gap: 4,
            main_align: MainAlign::Center,
            cross_align: CrossAlign::Center,
            ..UiStyle::default()
        },
        [icon, dex_number_text_node(entry.number, false, 11)],
    )
}
