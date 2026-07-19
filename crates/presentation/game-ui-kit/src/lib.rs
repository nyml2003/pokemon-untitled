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

/// A purely visual selectable surface. Pages attach their own business action
/// when interaction semantics are available.
pub fn button<Action>(
    theme: &GameUiTheme,
    style: UiStyle,
    selected: bool,
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
}

/// A visual row that owns the shared background of a tab control.
pub fn tab_bar<Action>(
    theme: &GameUiTheme,
    mut style: UiStyle,
    children: impl IntoIterator<Item = UiNode<Action>>,
) -> UiNode<Action> {
    style.direction = FlexDirection::Row;
    panel(theme, PanelTone::Panel, style, children)
}

/// A visual dialog surface. Its visibility and dismissal stay with the page.
pub fn modal<Action>(
    theme: &GameUiTheme,
    style: UiStyle,
    children: impl IntoIterator<Item = UiNode<Action>>,
) -> UiNode<Action> {
    panel(theme, PanelTone::Card, style, children)
}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
