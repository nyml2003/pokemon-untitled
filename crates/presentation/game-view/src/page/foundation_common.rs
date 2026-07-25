use super::super::common::{FOUNDATION_THEME, FoundationPage, FoundationPageAction};
use game_foundation::{GameState, ThinSliceContent};
use game_ui_kit::{
    PanelTone, TextTone, button as ui_button, panel as ui_panel, row as ui_row, text as ui_text,
};
use punctum_ui::{CrossAlign, Dimension, Insets, MainAlign, UiBuildError, UiKey, UiNode, UiStyle};

pub(super) fn foundation_tab(
    label: &str,
    key: &str,
    target: FoundationPage,
    selected: FoundationPage,
) -> Result<UiNode<FoundationPageAction>, UiBuildError> {
    let node = ui_button(
        &FOUNDATION_THEME,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(30),
            main_align: MainAlign::Center,
            cross_align: CrossAlign::Center,
            border_radius: FOUNDATION_THEME.small_radius,
            ..UiStyle::default()
        },
        target == selected,
        [ui_text(
            &FOUNDATION_THEME,
            if target == selected {
                TextTone::Selected
            } else {
                TextTone::Default
            },
            label,
            15,
            Dimension::Fill,
        )],
    )
    .with_key(UiKey::new(key)?);
    Ok(node.with_action(FoundationPageAction::SelectPage(target)))
}

pub(super) fn foundation_dialogue(message: &str) -> String {
    const MAX_VISIBLE_CHARACTERS: usize = 40;
    let mut characters = message.chars();
    let visible = characters
        .by_ref()
        .take(MAX_VISIBLE_CHARACTERS)
        .collect::<String>();
    if characters.next().is_some() {
        format!("{visible}...")
    } else {
        visible
    }
}

pub(super) fn foundation_info_panel(
    label: impl Into<String>,
    value: impl Into<String>,
) -> UiNode<FoundationPageAction> {
    ui_panel(
        &FOUNDATION_THEME,
        PanelTone::Panel,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            gap: 2,
            padding: Insets::all(4),
            border_radius: FOUNDATION_THEME.small_radius,
            ..UiStyle::default()
        },
        [
            ui_text(
                &FOUNDATION_THEME,
                TextTone::Muted,
                label,
                12,
                Dimension::Fill,
            ),
            ui_text(
                &FOUNDATION_THEME,
                TextTone::Default,
                value,
                15,
                Dimension::Fill,
            ),
        ],
    )
}

pub(super) fn trainer_card_row(
    label: impl Into<String>,
    value: impl Into<String>,
) -> UiNode<FoundationPageAction> {
    ui_row(
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(24),
            main_align: MainAlign::SpaceBetween,
            cross_align: CrossAlign::Center,
            ..UiStyle::default()
        },
        [
            ui_text(
                &FOUNDATION_THEME,
                TextTone::MutedInk,
                label,
                14,
                Dimension::Fill,
            ),
            ui_text(&FOUNDATION_THEME, TextTone::Ink, value, 15, Dimension::Fill),
        ],
    )
}

pub(super) fn party_rows(
    content: &ThinSliceContent,
    state: &GameState,
) -> Vec<UiNode<FoundationPageAction>> {
    state
        .party()
        .iter()
        .map(|creature| {
            let definition = content.creature(creature.template());
            let name = definition
                .map(|template| template.species())
                .unwrap_or(creature.template().as_str());
            let max_hp = definition
                .map(|template| template.max_hp())
                .unwrap_or(creature.hp());
            ui_row(
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(32),
                    main_align: MainAlign::SpaceBetween,
                    cross_align: CrossAlign::Center,
                    ..UiStyle::default()
                },
                [
                    ui_text(
                        &FOUNDATION_THEME,
                        TextTone::Default,
                        name,
                        15,
                        Dimension::Fill,
                    ),
                    ui_text(
                        &FOUNDATION_THEME,
                        TextTone::Muted,
                        format!("HP {}/{}", creature.hp(), max_hp),
                        14,
                        Dimension::Fill,
                    ),
                    ui_text(
                        &FOUNDATION_THEME,
                        TextTone::Muted,
                        format!("PP {}", creature.pp()),
                        14,
                        Dimension::Fill,
                    ),
                    ui_text(
                        &FOUNDATION_THEME,
                        TextTone::Default,
                        format!("EXP {}", creature.experience()),
                        14,
                        Dimension::Fill,
                    ),
                ],
            )
        })
        .collect()
}

pub(super) fn foundation_action_button(
    label: &str,
    key: &str,
    action: FoundationPageAction,
) -> Result<UiNode<FoundationPageAction>, UiBuildError> {
    let node = ui_button(
        &FOUNDATION_THEME,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            main_align: MainAlign::Center,
            cross_align: CrossAlign::Center,
            border_radius: FOUNDATION_THEME.small_radius,
            ..UiStyle::default()
        },
        false,
        [ui_text(
            &FOUNDATION_THEME,
            TextTone::Default,
            label,
            15,
            Dimension::Fill,
        )],
    )
    .with_key(UiKey::new(key)?);
    Ok(node.with_action(action))
}
