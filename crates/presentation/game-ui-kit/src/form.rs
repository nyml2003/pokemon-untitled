use crate::{ButtonOptions, GameUiTheme, TextTone, button_with_options, column, modal, row, text};
use punctum_ui::{
    CrossAlign, Dimension, Insets, MainAlign, UiBorder, UiContent, UiKey, UiNode, UiPixelOffset,
    UiStyle,
};

/// A single selectable value rendered by checkbox, radio, or select controls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormOption<Action> {
    pub key: UiKey,
    pub label: String,
    pub selected: bool,
    pub focused: bool,
    pub action: Action,
}

/// Projects a dimmed modal form surface with a centered, clipped content card.
pub fn form_shell<Action>(
    theme: &GameUiTheme,
    scroll_y: u32,
    children: impl IntoIterator<Item = UiNode<Action>>,
) -> UiNode<Action> {
    UiNode::auto()
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            main_align: MainAlign::Center,
            cross_align: CrossAlign::Center,
            padding: Insets::all(theme.large_spacing),
            ..UiStyle::default()
        })
        .with_content(UiContent::Fill(theme.modal_scrim))
        .with_children([modal(
            theme,
            UiStyle {
                width: Dimension::Ratio { units: 3, base: 4 },
                height: Dimension::Ratio { units: 7, base: 8 },
                direction: punctum_ui::FlexDirection::Column,
                clip: true,
                border: UiBorder {
                    widths: Insets::all(1),
                    color: theme.modal_border,
                },
                border_radius: theme.large_radius,
                ..UiStyle::default()
            },
            [column(
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Auto,
                    padding: Insets::all(theme.large_spacing.saturating_mul(2)),
                    gap: theme.large_spacing,
                    visual_offset: UiPixelOffset::new(
                        0,
                        i32::try_from(scroll_y).map_or(i32::MIN.saturating_add(1), |value| -value),
                    ),
                    ..UiStyle::default()
                },
                children,
            )],
        )])
}

/// Groups related form controls without introducing an additional panel surface.
pub fn form_section<Action>(
    theme: &GameUiTheme,
    children: impl IntoIterator<Item = UiNode<Action>>,
) -> UiNode<Action> {
    column(
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Auto,
            gap: theme.small_spacing,
            ..UiStyle::default()
        },
        children,
    )
}

/// Renders a non-focusable field label with optional supporting text.
pub fn label<Action>(
    theme: &GameUiTheme,
    title: impl Into<String>,
    supporting: Option<impl Into<String>>,
) -> UiNode<Action> {
    let mut children = vec![text(
        theme,
        TextTone::Muted,
        title,
        theme.body_text_size,
        Dimension::Fill,
    )];
    if let Some(supporting) = supporting {
        children.push(text(
            theme,
            TextTone::MutedInk,
            supporting,
            theme.body_text_size.saturating_sub(2).max(1),
            Dimension::Fill,
        ));
    }
    column(
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Auto,
            gap: 1,
            ..UiStyle::default()
        },
        children,
    )
}

/// Builds a focusable icon or command button with a stable key.
pub fn icon_button<Action>(
    theme: &GameUiTheme,
    key: UiKey,
    glyph: impl Into<String>,
    selected: bool,
    action: Action,
) -> UiNode<Action> {
    button_with_options(
        theme,
        UiStyle {
            width: Dimension::Px(32),
            height: Dimension::Px(32),
            main_align: MainAlign::Center,
            cross_align: CrossAlign::Center,
            border_radius: theme.small_radius,
            ..UiStyle::default()
        },
        ButtonOptions::new(selected, false),
        [text(
            theme,
            if selected {
                TextTone::Selected
            } else {
                TextTone::Default
            },
            glyph,
            theme.body_text_size,
            Dimension::Px(24),
        )],
    )
    .with_key(key)
    .with_action(action)
}

/// Projects a value field. Text mutation remains in the owning page model.
pub fn text_input<Action>(
    theme: &GameUiTheme,
    key: UiKey,
    value: impl Into<String>,
    focused: bool,
    action: Action,
) -> UiNode<Action> {
    input_surface(theme, key, value, focused, action)
}

/// Projects a numeric value field. Numeric parsing remains in the owning page model.
pub fn number_input<Action>(
    theme: &GameUiTheme,
    key: UiKey,
    value: impl Into<String>,
    focused: bool,
    action: Action,
) -> UiNode<Action> {
    input_surface(theme, key, value, focused, action)
}

/// Places the paired minimum and maximum numeric controls on one stable row.
pub fn range_input<Action>(
    theme: &GameUiTheme,
    minimum: UiNode<Action>,
    maximum: UiNode<Action>,
) -> UiNode<Action> {
    row(
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(30),
            gap: theme.small_spacing,
            cross_align: CrossAlign::Center,
            ..UiStyle::default()
        },
        [
            text(
                theme,
                TextTone::Muted,
                "最小",
                theme.body_text_size.saturating_sub(1).max(1),
                Dimension::Px(28),
            ),
            minimum,
            text(
                theme,
                TextTone::Muted,
                "~",
                theme.body_text_size,
                Dimension::Px(12),
            ),
            text(
                theme,
                TextTone::Muted,
                "最大",
                theme.body_text_size.saturating_sub(1).max(1),
                Dimension::Px(28),
            ),
            maximum,
        ],
    )
}

/// Projects a multi-select group whose actions are supplied by the caller.
pub fn checkbox_group<Action>(
    theme: &GameUiTheme,
    options: impl IntoIterator<Item = FormOption<Action>>,
) -> UiNode<Action> {
    choice_group(theme, options, false)
}

/// Projects a mutually-exclusive choice group whose actions are supplied by the caller.
pub fn radio_group<Action>(
    theme: &GameUiTheme,
    options: impl IntoIterator<Item = FormOption<Action>>,
) -> UiNode<Action> {
    choice_group(theme, options, true)
}

/// Projects a select trigger and only materializes its option rows while opened.
pub fn select<Action>(
    theme: &GameUiTheme,
    trigger_key: UiKey,
    value: impl Into<String>,
    focused: bool,
    trigger_action: Action,
    opened_options: Option<impl IntoIterator<Item = FormOption<Action>>>,
) -> UiNode<Action> {
    let mut children = vec![input_surface(
        theme,
        trigger_key,
        format!("{} v", value.into()),
        focused,
        trigger_action,
    )];
    if let Some(options) = opened_options {
        children.push(column(
            UiStyle {
                width: Dimension::Fill,
                height: Dimension::Auto,
                gap: 2,
                padding: Insets::all(theme.small_spacing),
                ..UiStyle::default()
            },
            options
                .into_iter()
                .map(|option| choice_option(theme, option, false, Dimension::Fill)),
        ));
    }
    column(
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Auto,
            gap: 2,
            ..UiStyle::default()
        },
        children,
    )
}

/// Projects a binary state as a focusable button.
pub fn toggle<Action>(
    theme: &GameUiTheme,
    key: UiKey,
    label: impl Into<String>,
    enabled: bool,
    focused: bool,
    action: Action,
) -> UiNode<Action> {
    let marker = if enabled { "[x]" } else { "[ ]" };
    button_with_options(
        theme,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(28),
            padding: Insets::symmetric(theme.small_spacing, 3),
            cross_align: CrossAlign::Center,
            border_radius: theme.small_radius,
            ..UiStyle::default()
        },
        ButtonOptions::new(enabled || focused, false),
        [text(
            theme,
            if enabled || focused {
                TextTone::Selected
            } else {
                TextTone::Default
            },
            format!("{marker} {}", label.into()),
            theme.body_text_size,
            Dimension::Fill,
        )],
    )
    .with_key(key)
    .with_action(action)
}

/// Builds the compact filter entry with its query command and active-condition summary.
pub fn filter_summary<Action>(
    theme: &GameUiTheme,
    icon: UiNode<Action>,
    summary: impl Into<String>,
) -> UiNode<Action> {
    let summary = summary.into();
    let mut children = vec![icon];
    if !summary.is_empty() {
        children.push(text(
            theme,
            TextTone::Muted,
            summary,
            theme.body_text_size.saturating_sub(1).max(1),
            Dimension::Auto,
        ));
    }
    row(
        UiStyle {
            width: Dimension::Auto,
            height: Dimension::Px(34),
            gap: theme.small_spacing,
            cross_align: CrossAlign::Center,
            ..UiStyle::default()
        },
        children,
    )
}

fn input_surface<Action>(
    theme: &GameUiTheme,
    key: UiKey,
    value: impl Into<String>,
    focused: bool,
    action: Action,
) -> UiNode<Action> {
    button_with_options(
        theme,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(28),
            padding: Insets::symmetric(theme.small_spacing, 3),
            cross_align: CrossAlign::Center,
            border_radius: theme.small_radius,
            ..UiStyle::default()
        },
        ButtonOptions::new(focused, false),
        [text(
            theme,
            if focused {
                TextTone::Selected
            } else {
                TextTone::Default
            },
            value,
            theme.body_text_size,
            Dimension::Fill,
        )],
    )
    .with_key(key)
    .with_action(action)
}

fn choice_group<Action>(
    theme: &GameUiTheme,
    options: impl IntoIterator<Item = FormOption<Action>>,
    radio: bool,
) -> UiNode<Action> {
    let options = options.into_iter().collect::<Vec<_>>();
    let mut rows = Vec::new();
    let mut current_row = Vec::new();
    for option in options {
        current_row.push(choice_option(theme, option, radio, Dimension::Auto));
        if current_row.len() == 4 {
            rows.push(form_choice_row(theme, std::mem::take(&mut current_row)));
        }
    }
    if !current_row.is_empty() {
        rows.push(form_choice_row(theme, current_row));
    }
    column(
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Auto,
            gap: theme.small_spacing,
            ..UiStyle::default()
        },
        rows,
    )
}

fn form_choice_row<Action>(theme: &GameUiTheme, children: Vec<UiNode<Action>>) -> UiNode<Action> {
    row(
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(28),
            gap: theme.small_spacing,
            cross_align: CrossAlign::Center,
            ..UiStyle::default()
        },
        children,
    )
}

fn choice_option<Action>(
    theme: &GameUiTheme,
    option: FormOption<Action>,
    radio: bool,
    width: Dimension,
) -> UiNode<Action> {
    let marker = match (radio, option.selected) {
        (true, true) => "(*)",
        (true, false) => "( )",
        (false, true) => "[x]",
        (false, false) => "[ ]",
    };
    button_with_options(
        theme,
        UiStyle {
            width,
            height: Dimension::Px(26),
            padding: Insets::symmetric(theme.small_spacing, 2),
            main_align: MainAlign::Center,
            cross_align: CrossAlign::Center,
            border_radius: theme.small_radius,
            ..UiStyle::default()
        },
        ButtonOptions::new(option.selected || option.focused, false),
        [text(
            theme,
            if option.selected || option.focused {
                TextTone::Selected
            } else {
                TextTone::Default
            },
            format!("{marker} {}", option.label),
            theme.body_text_size.saturating_sub(1).max(1),
            Dimension::Auto,
        )],
    )
    .with_key(option.key)
    .with_action(option.action)
}
