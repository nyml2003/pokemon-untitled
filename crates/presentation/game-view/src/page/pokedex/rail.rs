use super::*;
use game_page_model::PokedexEntryModel;
use game_ui_kit::{column as ui_column, row as ui_row};
use punctum_ui::{CrossAlign, Dimension, Insets, MainAlign, UiContent, UiNode, UiSize, UiStyle};

const RAIL_WIDTH: u32 = 96;
const RAIL_ITEM_HEIGHT: u32 = 64;
pub(super) const SELECTED_ICON_SIZE: u32 = 30;

pub(super) fn project(
    pokedex: &PokedexPageModel,
    hide_transition_icons: bool,
) -> UiNode<PageIntent> {
    let selected = selected_index(pokedex);
    let entries = (0..5)
        .map(|slot| rail_slot(pokedex, selected, slot, hide_transition_icons))
        .collect::<Vec<_>>();
    ui_column(
        UiStyle {
            width: Dimension::Px(RAIL_WIDTH),
            height: Dimension::Fill,
            main_align: MainAlign::Center,
            cross_align: CrossAlign::Center,
            ..UiStyle::default()
        },
        entries,
    )
}

pub(super) fn icon_origin(viewport: UiSize, relative_index: i64) -> (u32, u32) {
    let center_y = i64::from(viewport.height.saturating_sub(SELECTED_ICON_SIZE) / 2);
    let y = center_y
        .saturating_add(relative_index.saturating_mul(i64::from(RAIL_ITEM_HEIGHT)))
        .clamp(0, i64::from(u32::MAX)) as u32;
    (49, y)
}

fn rail_slot(
    pokedex: &PokedexPageModel,
    selected: usize,
    slot: usize,
    hide_transition_icons: bool,
) -> UiNode<PageIntent> {
    let index = if slot < 2 {
        selected.checked_sub(2 - slot)
    } else {
        selected.checked_add(slot - 2)
    };
    let current = slot == 2;
    let entry = index.and_then(|index| pokedex.entries.get(index));
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
    if current {
        node.with_content(UiContent::Fill(POKEDEX_THEME.selected))
    } else {
        node
    }
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
