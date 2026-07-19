//! Reusable, pure components for game pixel UI pages.

#![forbid(unsafe_code)]

use punctum_ui::{
    Dimension, FlexDirection, UiBorderRadius, UiColor, UiContent, UiContentId, UiKey, UiNode,
    UiPixelOffset, UiStyle,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameUiTheme {
    pub screen: UiColor,
    pub header: UiColor,
    pub panel: UiColor,
    pub selected: UiColor,
    pub selected_text: UiColor,
    pub card: UiColor,
    pub image_backdrop: UiColor,
    pub text: UiColor,
    pub muted_text: UiColor,
    pub ink: UiColor,
    pub muted_ink: UiColor,
    pub small_spacing: u32,
    pub medium_spacing: u32,
    pub large_spacing: u32,
    pub small_radius: UiBorderRadius,
    pub medium_radius: UiBorderRadius,
    pub large_radius: UiBorderRadius,
    pub body_text_size: u32,
    pub title_text_size: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelTone {
    Screen,
    Header,
    Panel,
    Selected,
    Card,
    ImageBackdrop,
}

impl PanelTone {
    const fn color(self, theme: &GameUiTheme) -> UiColor {
        match self {
            Self::Screen => theme.screen,
            Self::Header => theme.header,
            Self::Panel => theme.panel,
            Self::Selected => theme.selected,
            Self::Card => theme.card,
            Self::ImageBackdrop => theme.image_backdrop,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextTone {
    Default,
    Muted,
    Ink,
    MutedInk,
    Selected,
}

impl TextTone {
    const fn color(self, theme: &GameUiTheme) -> UiColor {
        match self {
            Self::Default => theme.text,
            Self::Muted => theme.muted_text,
            Self::Ink => theme.ink,
            Self::MutedInk => theme.muted_ink,
            Self::Selected => theme.selected_text,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpriteAppearance {
    Plain,
    Tinted(UiColor),
    Styled {
        tint: UiColor,
        pixel_offset: UiPixelOffset,
    },
}

pub fn screen<Action>(
    theme: &GameUiTheme,
    children: impl IntoIterator<Item = UiNode<Action>>,
) -> UiNode<Action> {
    panel(
        theme,
        PanelTone::Screen,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            direction: FlexDirection::Column,
            ..UiStyle::default()
        },
        children,
    )
}

pub fn panel<Action>(
    theme: &GameUiTheme,
    tone: PanelTone,
    style: UiStyle,
    children: impl IntoIterator<Item = UiNode<Action>>,
) -> UiNode<Action> {
    UiNode::auto()
        .with_style(style)
        .with_content(UiContent::Fill(tone.color(theme)))
        .with_children(children)
}

pub fn row<Action>(
    mut style: UiStyle,
    children: impl IntoIterator<Item = UiNode<Action>>,
) -> UiNode<Action> {
    style.direction = FlexDirection::Row;
    UiNode::auto().with_style(style).with_children(children)
}

pub fn column<Action>(
    mut style: UiStyle,
    children: impl IntoIterator<Item = UiNode<Action>>,
) -> UiNode<Action> {
    style.direction = FlexDirection::Column;
    UiNode::auto().with_style(style).with_children(children)
}

pub fn stack<Action>(
    mut style: UiStyle,
    children: impl IntoIterator<Item = UiNode<Action>>,
) -> UiNode<Action> {
    style.direction = FlexDirection::Stack;
    UiNode::auto().with_style(style).with_children(children)
}

pub fn text<Action>(
    theme: &GameUiTheme,
    tone: TextTone,
    content: impl Into<String>,
    font_size: u32,
    width: Dimension,
) -> UiNode<Action> {
    UiNode::auto()
        .with_style(UiStyle {
            width,
            height: Dimension::Px(font_size.saturating_add(6)),
            ..UiStyle::default()
        })
        .with_content(UiContent::Text {
            content: content.into(),
            color: tone.color(theme),
            font_size,
        })
}

pub fn image<Action>(content: UiContentId, style: UiStyle) -> UiNode<Action> {
    UiNode::auto()
        .with_style(style)
        .with_content(UiContent::Image(content))
}

pub fn sprite<Action>(
    content: UiContentId,
    style: UiStyle,
    appearance: SpriteAppearance,
) -> UiNode<Action> {
    let content = match appearance {
        SpriteAppearance::Plain => UiContent::Image(content),
        SpriteAppearance::Tinted(tint) => UiContent::ImageTinted { content, tint },
        SpriteAppearance::Styled { tint, pixel_offset } => UiContent::ImageStyled {
            content,
            tint,
            pixel_offset,
        },
    };
    UiNode::auto().with_style(style).with_content(content)
}

pub fn selectable_list_item<Action>(
    theme: &GameUiTheme,
    style: UiStyle,
    selected: bool,
    key: UiKey,
    action: Action,
    children: impl IntoIterator<Item = UiNode<Action>>,
) -> UiNode<Action> {
    panel(
        theme,
        if selected {
            PanelTone::Selected
        } else {
            PanelTone::Panel
        },
        style,
        children,
    )
    .with_key(key)
    .with_action(action)
}

#[cfg(test)]
mod tests {
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
}
