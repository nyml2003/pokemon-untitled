use super::super::common::{FOUNDATION_THEME, FoundationPageAction};
use super::foundation_common::trainer_card_row;
use game_foundation::{GameState, ThinSliceContent};
use game_ui_kit::{PanelTone, TextTone, panel as ui_panel, text as ui_text};
use punctum_ui::{Dimension, Insets, UiBuildError, UiNode, UiStyle};

pub(super) fn foundation_trainer_card(
    content: &ThinSliceContent,
    state: &GameState,
) -> Result<UiNode<FoundationPageAction>, UiBuildError> {
    let experience = state
        .party()
        .iter()
        .map(|creature| creature.experience())
        .sum::<u32>();
    let lead = state.party().first().map_or_else(
        || String::from("未登记"),
        |creature| {
            content
                .creature(creature.template())
                .map(|template| template.species().to_owned())
                .unwrap_or_else(|| creature.template().as_str().to_owned())
        },
    );
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
        [ui_panel(
            &FOUNDATION_THEME,
            PanelTone::Card,
            UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                gap: 6,
                padding: Insets::all(12),
                border_radius: FOUNDATION_THEME.large_radius,
                ..UiStyle::default()
            },
            [
                ui_text(
                    &FOUNDATION_THEME,
                    TextTone::Ink,
                    "训练家卡片",
                    22,
                    Dimension::Fill,
                ),
                ui_text(
                    &FOUNDATION_THEME,
                    TextTone::MutedInk,
                    "LOCAL PLAYER",
                    14,
                    Dimension::Fill,
                ),
                trainer_card_row("伙伴", format!("{}  ·  {lead}", state.party().len())),
                trainer_card_row("经验", experience.to_string()),
                trainer_card_row("金钱", state.money().amount().to_string()),
                trainer_card_row("训练师胜场", state.defeated_trainers().len().to_string()),
                trainer_card_row("事件记录", state.flags().len().to_string()),
            ],
        )],
    ))
}
