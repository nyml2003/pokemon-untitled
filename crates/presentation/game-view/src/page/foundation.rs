use super::super::common::{FOUNDATION_THEME, FoundationPage, FoundationPageAction};
use super::foundation_bag::foundation_bag;
use super::foundation_common::{foundation_action_button, foundation_tab};
use super::foundation_journey::foundation_journey;
use super::foundation_trainer_card::foundation_trainer_card;
use game_foundation::{GameState, ThinSliceContent};
use game_ui_kit::{
    PanelTone, TextTone, panel as ui_panel, row as ui_row, screen as ui_screen, text as ui_text,
};
use punctum_ui::{
    CrossAlign, Dimension, FlexDirection, Insets, MainAlign, UiBuildError, UiStyle, UiTree,
};

pub fn project_foundation(
    content: &ThinSliceContent,
    state: &GameState,
    page: FoundationPage,
) -> Result<UiTree<FoundationPageAction>, UiBuildError> {
    let tabs = [
        foundation_tab(
            "旅程",
            "foundation-tab-journey",
            FoundationPage::Journey,
            page,
        )?,
        foundation_tab("背包", "foundation-tab-bag", FoundationPage::Bag, page)?,
        foundation_tab(
            "训练家卡片",
            "foundation-tab-trainer-card",
            FoundationPage::TrainerCard,
            page,
        )?,
    ];
    let body = match page {
        FoundationPage::Journey => foundation_journey(content, state)?,
        FoundationPage::Bag => foundation_bag(content, state)?,
        FoundationPage::TrainerCard => foundation_trainer_card(content, state)?,
    };
    UiTree::new(ui_screen(
        &FOUNDATION_THEME,
        [
            ui_panel(
                &FOUNDATION_THEME,
                PanelTone::Header,
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(48),
                    direction: FlexDirection::Row,
                    main_align: MainAlign::SpaceBetween,
                    cross_align: CrossAlign::Center,
                    padding: Insets::symmetric(12, 8),
                    ..UiStyle::default()
                },
                [
                    ui_text(
                        &FOUNDATION_THEME,
                        TextTone::Default,
                        "旅程记录",
                        22,
                        Dimension::Fill,
                    ),
                    foundation_action_button("×", "foundation-close", FoundationPageAction::Close)?,
                ],
            ),
            ui_row(
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(38),
                    gap: 4,
                    padding: Insets::symmetric(8, 4),
                    ..UiStyle::default()
                },
                tabs,
            ),
            body,
        ],
    ))
}
