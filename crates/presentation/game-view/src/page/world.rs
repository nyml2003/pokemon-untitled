use super::super::common::FOUNDATION_THEME;
use super::assets::{page_world_player_asset, world_tree_image};
use super::common::page_notice;
use game_page_model::{PageIntent, WorldPageModel};
use game_ui_kit::{
    PanelTone, column as ui_column, image as ui_image, panel as ui_panel, row as ui_row,
};
use punctum_ui::{CrossAlign, Dimension, MainAlign, UiBuildError, UiContentId, UiStyle, UiTree};

pub(super) fn project_page_world(
    _: &WorldPageModel,
    notice: Option<&str>,
) -> Result<UiTree<PageIntent>, UiBuildError> {
    UiTree::new(ui_column(
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            gap: 0,
            ..UiStyle::default()
        },
        [
            ui_panel(
                &FOUNDATION_THEME,
                PanelTone::Header,
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(260),
                    ..UiStyle::default()
                },
                [],
            ),
            ui_panel(
                &FOUNDATION_THEME,
                PanelTone::Card,
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(32),
                    ..UiStyle::default()
                },
                [],
            ),
            ui_panel(
                &FOUNDATION_THEME,
                PanelTone::Panel,
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    ..UiStyle::default()
                },
                [ui_row(
                    UiStyle {
                        width: Dimension::Fill,
                        height: Dimension::Fill,
                        main_align: MainAlign::Center,
                        cross_align: CrossAlign::Center,
                        ..UiStyle::default()
                    },
                    [
                        world_tree_image(),
                        ui_image(
                            UiContentId::from_resource_key(page_world_player_asset().as_str()),
                            UiStyle::fixed(96, 96),
                        ),
                    ],
                )],
            ),
            page_notice(notice),
        ],
    ))
}
