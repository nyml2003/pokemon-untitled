use super::super::common::FOUNDATION_THEME;
use super::common::{page_notice, page_slot};
use super::party::project_pause_party;
use super::pause_bag::project_pause_bag;
use super::pause_trainer_card::project_pause_trainer_card;
use super::pokedex::project_pause_pokedex;
use game_page_model::{PageIntent, PausePage, PausePageModel};
use game_ui_kit::{PanelTone, panel as ui_panel, row as ui_row};
use punctum_ui::{
    CrossAlign, Dimension, FlexDirection, Insets, MainAlign, UiBuildError, UiStyle, UiTree,
};

pub(super) fn project_page_pause(
    pause: &PausePageModel,
    notice: Option<&str>,
) -> Result<UiTree<PageIntent>, UiBuildError> {
    match pause {
        PausePageModel::Menu => project_pause_menu(notice),
        PausePageModel::Party(party) => project_pause_party(party, notice),
        PausePageModel::Bag(bag) => project_pause_bag(bag, notice),
        PausePageModel::Pokedex(pokedex) => project_pause_pokedex(pokedex, notice),
        PausePageModel::TrainerCard(card) => project_pause_trainer_card(card, notice),
    }
}

fn project_pause_menu(notice: Option<&str>) -> Result<UiTree<PageIntent>, UiBuildError> {
    let entries = [
        page_slot(
            "队",
            "page-pause-party",
            false,
            Some(PageIntent::SelectPausePage(PausePage::Party)),
            Dimension::Fill,
            Dimension::Px(150),
        )?,
        page_slot(
            "包",
            "page-pause-bag",
            false,
            Some(PageIntent::SelectPausePage(PausePage::Bag)),
            Dimension::Fill,
            Dimension::Px(150),
        )?,
        page_slot(
            "鉴",
            "page-pause-pokedex",
            false,
            Some(PageIntent::SelectPausePage(PausePage::Pokedex)),
            Dimension::Fill,
            Dimension::Px(150),
        )?,
        page_slot(
            "卡",
            "page-pause-trainer-card",
            false,
            Some(PageIntent::SelectPausePage(PausePage::TrainerCard)),
            Dimension::Fill,
            Dimension::Px(150),
        )?,
    ];
    UiTree::new(ui_panel(
        &FOUNDATION_THEME,
        PanelTone::Screen,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            direction: FlexDirection::Column,
            main_align: MainAlign::Center,
            cross_align: CrossAlign::Center,
            gap: 16,
            padding: Insets::all(32),
            ..UiStyle::default()
        },
        [
            ui_row(
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(150),
                    gap: 12,
                    ..UiStyle::default()
                },
                entries,
            ),
            page_notice(notice),
        ],
    ))
}
