//! Reusable, pure components for game pixel UI pages.

#![forbid(unsafe_code)]

use punctum_ui::{
    CrossAlign, Dimension, FlexDirection, UiBorderRadius, UiButtonStyle, UiColor, UiContent,
    UiContentId, UiKey, UiNode, UiPixelOffset, UiStyle,
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
    pub button: GameButtonTheme,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameButtonTheme {
    pub hover_color: UiColor,
    pub pressed_color: UiColor,
    pub disabled_color: UiColor,
    pub focus_color: UiColor,
    pub ripple_color: UiColor,
    pub focus_width: u32,
    pub ripple_duration_ms: u32,
}

impl GameButtonTheme {
    const fn style(self, selected: bool, disabled: bool) -> UiButtonStyle {
        UiButtonStyle {
            selected,
            disabled,
            hover_color: self.hover_color,
            pressed_color: self.pressed_color,
            disabled_color: self.disabled_color,
            focus_color: self.focus_color,
            ripple_color: self.ripple_color,
            focus_width: self.focus_width,
            ripple_duration_ms: self.ripple_duration_ms,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ButtonOptions {
    pub selected: bool,
    pub disabled: bool,
}

impl ButtonOptions {
    pub const fn new(selected: bool, disabled: bool) -> Self {
        Self { selected, disabled }
    }
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StatChartValues {
    pub hp: u16,
    pub attack: u16,
    pub defense: u16,
    pub special_attack: u16,
    pub special_defense: u16,
    pub speed: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatChartView {
    Bars,
    Hexagon,
}

/// Builds the shared six-stat component used by pages that show base stats.
pub fn stat_chart<Action>(
    theme: &GameUiTheme,
    view: StatChartView,
    values: Option<StatChartValues>,
) -> UiNode<Action> {
    match view {
        StatChartView::Bars => stat_bars(theme, values),
        StatChartView::Hexagon => stat_hexagon(theme, values),
    }
}

fn stat_bars<Action>(theme: &GameUiTheme, values: Option<StatChartValues>) -> UiNode<Action> {
    let rows = values.map_or_else(
        || vec![text(theme, TextTone::MutedInk, "--", 14, Dimension::Fill)],
        |values| {
            [
                ("HP", values.hp),
                ("ATK", values.attack),
                ("DEF", values.defense),
                ("SPA", values.special_attack),
                ("SPD", values.special_defense),
                ("SPE", values.speed),
            ]
            .into_iter()
            .map(|(label, value)| stat_bar(theme, label, value))
            .collect()
        },
    );
    column(
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            gap: 4,
            ..UiStyle::default()
        },
        rows,
    )
}

fn stat_bar<Action>(theme: &GameUiTheme, label: &str, value: u16) -> UiNode<Action> {
    let value = value.min(256);
    row(
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(18),
            gap: 6,
            cross_align: CrossAlign::Center,
            ..UiStyle::default()
        },
        [
            text(theme, TextTone::MutedInk, label, 12, Dimension::Px(32)),
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(8),
                    border_radius: theme.small_radius,
                    ..UiStyle::default()
                })
                .with_content(UiContent::Fill(theme.panel))
                .with_children([UiNode::auto()
                    .with_style(UiStyle {
                        width: Dimension::Ratio {
                            units: u32::from(value),
                            base: 256,
                        },
                        height: Dimension::Fill,
                        border_radius: theme.small_radius,
                        ..UiStyle::default()
                    })
                    .with_content(UiContent::Fill(theme.selected))]),
            text(
                theme,
                TextTone::MutedInk,
                value.to_string(),
                12,
                Dimension::Px(32),
            ),
        ],
    )
}

fn stat_hexagon<Action>(theme: &GameUiTheme, values: Option<StatChartValues>) -> UiNode<Action> {
    let values = values.unwrap_or(StatChartValues {
        hp: 0,
        attack: 0,
        defense: 0,
        special_attack: 0,
        special_defense: 0,
        speed: 0,
    });
    UiNode::auto()
        .with_content(UiContent::RadarChart {
            values: [
                values.hp,
                values.attack,
                values.defense,
                values.special_attack,
                values.special_defense,
                values.speed,
            ],
            max: 256,
            rings: 5,
            grid_color: theme.panel,
            axis_color: theme.muted_ink,
            fill_color: UiColor::new(
                theme.selected.red,
                theme.selected.green,
                theme.selected.blue,
                96,
            ),
            edge_color: theme.selected,
            point_color: theme.selected_text,
            label_color: theme.muted_ink,
            labels: [
                String::from("HP"),
                String::from("ATK"),
                String::from("DEF"),
                String::from("SPA"),
                String::from("SPD"),
                String::from("SPE"),
            ],
            label_font_size: 11,
        })
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            clip: true,
            ..UiStyle::default()
        })
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
    button_surface(theme, style, ButtonOptions::new(selected, false), children)
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
    button_with_options(theme, style, ButtonOptions::new(selected, false), children)
}

pub fn button_with_options<Action>(
    theme: &GameUiTheme,
    style: UiStyle,
    options: ButtonOptions,
    children: impl IntoIterator<Item = UiNode<Action>>,
) -> UiNode<Action> {
    button_surface(theme, style, options, children)
}

fn button_surface<Action>(
    theme: &GameUiTheme,
    mut style: UiStyle,
    options: ButtonOptions,
    children: impl IntoIterator<Item = UiNode<Action>>,
) -> UiNode<Action> {
    style.clip = true;
    panel(
        theme,
        if options.selected {
            PanelTone::Selected
        } else {
            PanelTone::Panel
        },
        style,
        children,
    )
    .with_button(theme.button.style(options.selected, options.disabled))
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
