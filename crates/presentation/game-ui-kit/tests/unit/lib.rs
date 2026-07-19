use punctum_ui::{UiBorderRadius, UiDrawCommand, UiSize, UiTree};

use super::*;

const THEME: GameUiTheme = GameUiTheme {
    screen: UiColor::new(1, 2, 3, 255),
    header: UiColor::new(4, 5, 6, 255),
    panel: UiColor::new(7, 8, 9, 255),
    selected: UiColor::new(10, 11, 12, 255),
    selected_text: UiColor::new(31, 32, 33, 255),
    card: UiColor::new(13, 14, 15, 255),
    image_backdrop: UiColor::new(16, 17, 18, 255),
    text: UiColor::new(19, 20, 21, 255),
    muted_text: UiColor::new(22, 23, 24, 255),
    ink: UiColor::new(25, 26, 27, 255),
    muted_ink: UiColor::new(28, 29, 30, 255),
    small_spacing: 4,
    medium_spacing: 8,
    large_spacing: 12,
    small_radius: UiBorderRadius::all(2),
    medium_radius: UiBorderRadius::all(4),
    large_radius: UiBorderRadius::all(6),
    body_text_size: 12,
    title_text_size: 18,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TestAction {
    Select,
}

#[test]
fn selectable_item_keeps_its_action_and_uses_the_selected_token() {
    let tree = UiTree::new(screen(
        &THEME,
        [selectable_list_item(
            &THEME,
            UiStyle {
                width: Dimension::Px(40),
                height: Dimension::Px(20),
                border_radius: UiBorderRadius::all(4),
                ..UiStyle::default()
            },
            true,
            UiKey::new("entry").unwrap(),
            TestAction::Select,
            [text(
                &THEME,
                TextTone::Selected,
                "Entry",
                12,
                Dimension::Fill,
            )],
        )],
    ))
    .unwrap();
    let frame = tree.resolve(UiSize::new(40, 20)).unwrap();

    assert_eq!(frame.hit_action(1, 1), Some(&TestAction::Select));
    assert!(frame.commands().iter().any(|command| matches!(
        command,
        UiDrawCommand::Fill { color, .. } if *color == THEME.selected
    )));
}

#[test]
fn sprite_selects_the_requested_image_content_variant() {
    let tree = UiTree::<()>::new(sprite(
        UiContentId::new("sprite/test").unwrap(),
        UiStyle::fixed(16, 16),
        SpriteAppearance::Styled {
            tint: THEME.text,
            pixel_offset: UiPixelOffset::new(2, -1),
        },
    ))
    .unwrap();
    let frame = tree.resolve(UiSize::new(16, 16)).unwrap();

    assert!(matches!(
        frame.commands()[0],
        UiDrawCommand::Image { pixel_offset, .. } if pixel_offset == UiPixelOffset::new(2, -1)
    ));
}

#[test]
fn button_tab_bar_and_modal_use_their_theme_surfaces() {
    let tree = UiTree::<()>::new(column(
        UiStyle {
            width: Dimension::Px(60),
            height: Dimension::Px(60),
            ..UiStyle::default()
        },
        [
            button(&THEME, UiStyle::fixed(60, 20), true, std::iter::empty()),
            tab_bar(&THEME, UiStyle::fixed(60, 20), std::iter::empty()),
            modal(&THEME, UiStyle::fixed(60, 20), std::iter::empty()),
        ],
    ))
    .unwrap();
    let frame = tree.resolve(UiSize::new(60, 60)).unwrap();
    let colors = frame
        .commands()
        .iter()
        .filter_map(|command| match command {
            UiDrawCommand::Fill { color, .. } => Some(*color),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(colors.contains(&THEME.selected));
    assert!(colors.contains(&THEME.panel));
    assert!(colors.contains(&THEME.card));
}
