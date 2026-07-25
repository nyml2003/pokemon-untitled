use super::super::common::{FOUNDATION_THEME, FoundationPageAction};
use super::foundation_common::{
    foundation_action_button, foundation_dialogue, foundation_info_panel, party_rows,
};
use game_foundation::{Direction as FoundationDirection, GameState, ThinSliceContent};
use game_ui_kit::{
    PanelTone, TextTone, column as ui_column, panel as ui_panel, row as ui_row, text as ui_text,
};
use punctum_ui::{Dimension, Insets, UiBuildError, UiNode, UiStyle};

pub(super) fn foundation_journey(
    content: &ThinSliceContent,
    state: &GameState,
) -> Result<UiNode<FoundationPageAction>, UiBuildError> {
    let mut party = party_rows(content, state);
    if party.is_empty() {
        party.push(ui_text(
            &FOUNDATION_THEME,
            TextTone::Muted,
            "尚未获得伙伴",
            19,
            Dimension::Fill,
        ));
    }
    let encounter = match (state.pending_encounter(), state.active_battle()) {
        (Some(position), _) => format!("草丛遭遇  {}, {}", position.x(), position.y()),
        (_, Some(battle)) => battle
            .trainer()
            .and_then(|trainer| content.trainer(trainer))
            .map(|trainer| {
                let opponent = trainer
                    .pokemon()
                    .first()
                    .map(|pokemon| format!("{} Lv{}", pokemon.species(), pokemon.level()))
                    .unwrap_or_else(|| String::from("未知队伍"));
                format!("对战中  {} / {}", trainer.name(), opponent)
            })
            .unwrap_or_else(|| format!("战斗中  {}", battle.battle().as_str())),
        (None, None) => String::from("探索中"),
    };
    let dialogue = state
        .last_message()
        .map(foundation_dialogue)
        .unwrap_or_else(|| String::from("尚无对话"));
    let movement_actions = [
        foundation_action_button(
            "↑",
            "foundation-move-up",
            FoundationPageAction::Move(FoundationDirection::Up),
        )?,
        foundation_action_button(
            "←",
            "foundation-move-left",
            FoundationPageAction::Move(FoundationDirection::Left),
        )?,
        foundation_action_button(
            "↓",
            "foundation-move-down",
            FoundationPageAction::Move(FoundationDirection::Down),
        )?,
        foundation_action_button(
            "→",
            "foundation-move-right",
            FoundationPageAction::Move(FoundationDirection::Right),
        )?,
    ];
    let journey_actions = [
        foundation_action_button(
            "交互",
            "foundation-interact",
            FoundationPageAction::Interact,
        )?,
        foundation_action_button(
            "遭遇",
            "foundation-encounter",
            FoundationPageAction::Encounter,
        )?,
        foundation_action_button(
            "推进",
            "foundation-resolve",
            FoundationPageAction::ResolveBattle,
        )?,
        foundation_action_button("存档", "foundation-save", FoundationPageAction::Save)?,
    ];
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
                    foundation_info_panel("地点", state.map().as_str()),
                    foundation_info_panel(
                        "坐标",
                        format!("{}, {}", state.position().x(), state.position().y()),
                    ),
                    foundation_info_panel("状态", encounter),
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
                std::iter::once(ui_text(
                    &FOUNDATION_THEME,
                    TextTone::Default,
                    format!("队伍  对话: {dialogue}"),
                    16,
                    Dimension::Fill,
                ))
                .chain(party),
            ),
            ui_column(
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(60),
                    gap: 4,
                    ..UiStyle::default()
                },
                [
                    ui_row(
                        UiStyle {
                            width: Dimension::Fill,
                            height: Dimension::Fill,
                            gap: 4,
                            ..UiStyle::default()
                        },
                        movement_actions,
                    ),
                    ui_row(
                        UiStyle {
                            width: Dimension::Fill,
                            height: Dimension::Fill,
                            gap: 4,
                            ..UiStyle::default()
                        },
                        journey_actions,
                    ),
                ],
            ),
        ],
    ))
}
