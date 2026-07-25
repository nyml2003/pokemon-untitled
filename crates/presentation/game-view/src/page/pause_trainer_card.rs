use super::super::common::FOUNDATION_THEME;
use super::common::{page_detail, page_notice, page_slot};
use game_page_model::{PageIntent, TrainerCardPageModel};
use game_ui_kit::{PanelTone, panel as ui_panel, screen as ui_screen};
use punctum_ui::{
    CrossAlign, Dimension, FlexDirection, Insets, MainAlign, UiBuildError, UiStyle, UiTree,
};

pub(super) fn project_pause_trainer_card(
    card: &TrainerCardPageModel,
    notice: Option<&str>,
) -> Result<UiTree<PageIntent>, UiBuildError> {
    UiTree::new(ui_screen(
        &FOUNDATION_THEME,
        [ui_panel(
            &FOUNDATION_THEME,
            PanelTone::Screen,
            UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                direction: FlexDirection::Column,
                main_align: MainAlign::Center,
                cross_align: CrossAlign::Center,
                gap: 12,
                padding: Insets::all(24),
                ..UiStyle::default()
            },
            [
                page_slot(
                    "卡",
                    "page-trainer-card-placeholder",
                    true,
                    None,
                    Dimension::Px(180),
                    Dimension::Px(180),
                )?,
                page_detail(
                    card.location.as_str(),
                    format!(
                        "金钱 {}    同行 {}/6    训练家 {}/{}",
                        card.money.amount(),
                        card.party_count,
                        card.defeated_trainers,
                        card.total_trainers
                    ),
                ),
                page_notice(notice),
            ],
        )],
    ))
}
