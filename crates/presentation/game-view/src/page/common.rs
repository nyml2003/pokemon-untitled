use super::super::common::FOUNDATION_THEME;
use game_assets::AssetKey;
use game_page_model::PageIntent;
use game_ui_kit::{
    ButtonOptions, PanelTone, TextTone, button_with_options as ui_button_with_options,
    image as ui_image, panel as ui_panel, text as ui_text,
};
use punctum_ui::{
    CrossAlign, Dimension, Insets, MainAlign, UiBuildError, UiColor, UiContentId, UiKey, UiNode,
    UiStyle,
};

pub(super) fn page_notice(notice: Option<&str>) -> UiNode<PageIntent> {
    match notice {
        Some(notice) => ui_panel(
            &FOUNDATION_THEME,
            PanelTone::Selected,
            UiStyle {
                width: Dimension::Fill,
                height: Dimension::Px(28),
                padding: Insets::symmetric(8, 4),
                border_radius: FOUNDATION_THEME.small_radius,
                ..UiStyle::default()
            },
            [ui_text(
                &FOUNDATION_THEME,
                TextTone::Selected,
                notice,
                13,
                Dimension::Fill,
            )],
        ),
        None => UiNode::auto().with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(0),
            ..UiStyle::default()
        }),
    }
}

pub(super) fn page_detail(
    label: impl Into<String>,
    value: impl Into<String>,
) -> UiNode<PageIntent> {
    ui_panel(
        &FOUNDATION_THEME,
        PanelTone::Panel,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(64),
            gap: 4,
            padding: Insets::symmetric(12, 8),
            border: punctum_ui::UiBorder {
                widths: Insets::all(1),
                color: UiColor::new(76, 112, 139, 255),
            },
            border_radius: FOUNDATION_THEME.small_radius,
            ..UiStyle::default()
        },
        [
            ui_text(
                &FOUNDATION_THEME,
                TextTone::Default,
                label,
                16,
                Dimension::Fill,
            ),
            ui_text(
                &FOUNDATION_THEME,
                TextTone::Muted,
                value,
                14,
                Dimension::Fill,
            ),
        ],
    )
}

pub(super) fn page_slot(
    label: impl Into<String>,
    key: impl Into<String>,
    selected: bool,
    action: Option<PageIntent>,
    width: Dimension,
    height: Dimension,
) -> Result<UiNode<PageIntent>, UiBuildError> {
    page_slot_with_image(label, key, selected, action, width, height, None)
}

pub(super) fn page_slot_with_image(
    label: impl Into<String>,
    key: impl Into<String>,
    selected: bool,
    action: Option<PageIntent>,
    width: Dimension,
    height: Dimension,
    image: Option<AssetKey>,
) -> Result<UiNode<PageIntent>, UiBuildError> {
    let label = label.into();
    let key = key.into();
    let tone = if action.is_some() {
        TextTone::Default
    } else {
        TextTone::Muted
    };
    let mut children = Vec::with_capacity(2);
    if let Some(image) = image {
        children.push(ui_image(
            UiContentId::from_resource_key(image.as_str()),
            UiStyle::fixed(72, 72),
        ));
    }
    children.push(ui_text(&FOUNDATION_THEME, tone, label, 18, Dimension::Fill));
    let node = ui_button_with_options(
        &FOUNDATION_THEME,
        UiStyle {
            width,
            height,
            main_align: MainAlign::Center,
            cross_align: CrossAlign::Center,
            padding: Insets::symmetric(10, 6),
            border: punctum_ui::UiBorder {
                widths: Insets::all(1),
                color: UiColor::new(76, 112, 139, 255),
            },
            border_radius: FOUNDATION_THEME.small_radius,
            ..UiStyle::default()
        },
        ButtonOptions::new(selected, action.is_none()),
        children,
    )
    .with_key(UiKey::new(key)?);
    Ok(match action {
        Some(action) => node.with_action(action),
        None => node,
    })
}
