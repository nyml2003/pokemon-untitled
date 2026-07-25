use super::super::common::{FOUNDATION_THEME, FoundationPageAction};
use super::foundation_common::{foundation_action_button, foundation_info_panel};
use game_foundation::{GameState, ThinSliceContent};
use game_ui_kit::{PanelTone, TextTone, panel as ui_panel, row as ui_row, text as ui_text};
use punctum_ui::{CrossAlign, Dimension, Insets, MainAlign, UiBuildError, UiNode, UiStyle};

pub(super) fn foundation_bag(
    content: &ThinSliceContent,
    state: &GameState,
) -> Result<UiNode<FoundationPageAction>, UiBuildError> {
    let mut entries = state
        .inventory()
        .entries()
        .iter()
        .map(|(item, quantity)| {
            let category = content
                .item(item)
                .map(|definition| format!("{:?}", definition.category()))
                .unwrap_or_else(|| String::from("未知"));
            ui_row(
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(42),
                    main_align: MainAlign::SpaceBetween,
                    cross_align: CrossAlign::Center,
                    ..UiStyle::default()
                },
                [
                    ui_text(
                        &FOUNDATION_THEME,
                        TextTone::Default,
                        item.as_str(),
                        19,
                        Dimension::Fill,
                    ),
                    ui_text(
                        &FOUNDATION_THEME,
                        TextTone::Muted,
                        category,
                        16,
                        Dimension::Fill,
                    ),
                    ui_text(
                        &FOUNDATION_THEME,
                        TextTone::Default,
                        format!("x{quantity}"),
                        19,
                        Dimension::Fill,
                    ),
                ],
            )
        })
        .collect::<Vec<_>>();
    if entries.is_empty() {
        entries.push(ui_text(
            &FOUNDATION_THEME,
            TextTone::Muted,
            "背包为空",
            19,
            Dimension::Fill,
        ));
    }
    Ok(ui_panel(
        &FOUNDATION_THEME,
        PanelTone::Screen,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            gap: 6,
            padding: Insets::all(8),
            ..UiStyle::default()
        },
        [
            ui_row(
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(46),
                    gap: 4,
                    ..UiStyle::default()
                },
                [
                    foundation_info_panel("金钱", state.money().amount().to_string()),
                    foundation_info_panel(
                        "容量",
                        format!(
                            "{}/{}",
                            state.inventory().entries().len(),
                            state.inventory().capacity()
                        ),
                    ),
                    foundation_action_button(
                        "购买伤药",
                        "foundation-buy-potion",
                        FoundationPageAction::BuyPotion,
                    )?,
                ],
            ),
            ui_panel(
                &FOUNDATION_THEME,
                PanelTone::Panel,
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    gap: 4,
                    padding: Insets::all(8),
                    border_radius: FOUNDATION_THEME.medium_radius,
                    ..UiStyle::default()
                },
                entries,
            ),
        ],
    ))
}
