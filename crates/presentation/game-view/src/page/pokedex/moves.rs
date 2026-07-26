use super::*;
use game_ui_kit::{
    PanelTone, column as ui_column, image as ui_image, panel as ui_panel, row as ui_row,
};
use punctum_ui::{
    CrossAlign, Dimension, Insets, KeyboardSingleColumnFixedHeightScrollView, MainAlign,
    UiBuildError, UiContent, UiContentId, UiKey, UiNode, UiStyle,
};

const MOVE_VISIBLE_ITEMS: usize = 7;
const MOVE_ITEM_HEIGHT: u32 = 52;

pub(super) fn project(
    pokedex: &PokedexPageModel,
    interactive: bool,
    hide_transition_icons: bool,
) -> Result<UiNode<PageIntent>, UiBuildError> {
    let mut scroll = KeyboardSingleColumnFixedHeightScrollView::new(
        pokedex.moves.len(),
        MOVE_VISIBLE_ITEMS,
        MOVE_ITEM_HEIGHT,
    )
    .with_gap(4)
    .with_overscan(2);
    scroll.select(pokedex.selected_move);
    let rows = scroll
        .render_range()
        .filter_map(|index| pokedex.moves.get(index).map(|item| (index, item)))
        .map(|(index, item)| move_row(index, item, index == scroll.selected_index(), interactive))
        .collect::<Result<Vec<_>, _>>()?;
    let list = if pokedex.moves.is_empty() {
        text_node("没有可显示的技能记录", TextTone::Muted, 15)
    } else {
        scroll.node(
            UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                gap: 4,
                ..UiStyle::default()
            },
            rows,
        )
    };
    let summary = pokedex.moves.get(scroll.selected_index()).map_or_else(
        || {
            ui_panel(
                &POKEDEX_THEME,
                PanelTone::Panel,
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(62),
                    padding: Insets::symmetric(12, 8),
                    ..UiStyle::default()
                },
                [text_node("--", TextTone::Muted, 14)],
            )
        },
        move_summary,
    );
    Ok(ui_row(
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            padding: Insets::all(28),
            gap: 28,
            ..UiStyle::default()
        },
        [
            rail::project(pokedex, hide_transition_icons),
            ui_column(
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    gap: 10,
                    ..UiStyle::default()
                },
                [list, summary],
            ),
        ],
    ))
}

fn move_row(
    index: usize,
    item: &PokedexMoveModel,
    selected: bool,
    interactive: bool,
) -> Result<UiNode<PageIntent>, UiBuildError> {
    let node = UiNode::auto()
        .with_key(UiKey::new(format!("page-pokedex-move-{index}"))?)
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            padding: Insets::symmetric(10, 5),
            ..UiStyle::default()
        })
        .with_children([ui_row(
            UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                gap: 10,
                cross_align: CrossAlign::Center,
                ..UiStyle::default()
            },
            [
                ui_image(
                    UiContentId::from_resource_key(move_category_asset(item.category).as_str()),
                    UiStyle::fixed(48, 21),
                ),
                ui_column(
                    UiStyle {
                        width: Dimension::Fill,
                        height: Dimension::Fill,
                        gap: 1,
                        main_align: MainAlign::Center,
                        ..UiStyle::default()
                    },
                    [
                        text_node(
                            format!("{:02}  {}", index.saturating_add(1), item.name),
                            if selected {
                                TextTone::Selected
                            } else {
                                TextTone::Default
                            },
                            16,
                        ),
                        text_node(format_move_details(item), TextTone::Muted, 12),
                    ],
                ),
            ],
        )]);
    let node = if selected {
        node.with_content(UiContent::Fill(POKEDEX_THEME.selected))
    } else {
        node
    };
    Ok(if interactive {
        node.with_action(PageIntent::SelectPokedexMove(index))
    } else {
        node
    })
}

fn move_summary(item: &PokedexMoveModel) -> UiNode<PageIntent> {
    ui_panel(
        &POKEDEX_THEME,
        PanelTone::Panel,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(62),
            padding: Insets::symmetric(12, 8),
            ..UiStyle::default()
        },
        [
            text_node(item.name.as_str(), TextTone::Default, 16),
            text_node(format_move_details(item), TextTone::Muted, 13),
        ],
    )
}
