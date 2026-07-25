use super::super::common::FOUNDATION_THEME;
use super::common::{page_detail, page_notice, page_slot};
use game_page_model::{PageIntent, SaveConfirmPageModel, SaveUnavailableReason};
use game_ui_kit::{PanelTone, panel as ui_panel, screen as ui_screen};
use punctum_ui::{CrossAlign, Dimension, Insets, MainAlign, UiBuildError, UiStyle, UiTree};

pub(super) fn project_page_save_confirm(
    save: &SaveConfirmPageModel,
    notice: Option<&str>,
) -> Result<UiTree<PageIntent>, UiBuildError> {
    let message = match save.unavailable_reason {
        None => "当前位置安全，确认后写入当前存档。",
        Some(SaveUnavailableReason::BattleActive) => "战斗进行中，暂时不能保存。",
    };
    UiTree::new(ui_screen(
        &FOUNDATION_THEME,
        [ui_panel(
            &FOUNDATION_THEME,
            PanelTone::Screen,
            UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                gap: 16,
                padding: Insets::all(24),
                main_align: MainAlign::Center,
                cross_align: CrossAlign::Center,
                ..UiStyle::default()
            },
            [
                page_slot(
                    "存",
                    "page-save-state",
                    save.available,
                    None,
                    Dimension::Px(160),
                    Dimension::Px(120),
                )?,
                page_detail("存档", message),
                page_slot(
                    if save.available { "存" } else { "x" },
                    "page-save-confirm",
                    false,
                    save.available.then_some(PageIntent::ConfirmSave),
                    Dimension::Px(120),
                    Dimension::Px(64),
                )?,
                page_notice(notice),
            ],
        )],
    ))
}
