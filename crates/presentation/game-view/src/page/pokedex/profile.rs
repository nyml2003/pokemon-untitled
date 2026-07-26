use super::*;
use game_ui_kit::{
    ButtonOptions, PanelTone, StatChartValues, button_with_options as ui_button_with_options,
    column as ui_column, panel as ui_panel, row as ui_row, stat_chart,
};
use punctum_ui::{
    CrossAlign, Dimension, FlexDirection, Insets, MainAlign, UiBuildError, UiKey, UiNode, UiStyle,
};

pub(super) fn project(
    pokedex: &PokedexPageModel,
    _wheel_position: i32,
    hide_transition_icons: bool,
) -> Result<UiNode<PageIntent>, UiBuildError> {
    let selected = &pokedex.selected;
    let stats = selected.stats.map(|stats| StatChartValues {
        hp: stats.hp,
        attack: stats.attack,
        defense: stats.defense,
        special_attack: stats.special_attack,
        special_defense: stats.special_defense,
        speed: stats.speed,
    });
    let types = type_icons(&selected.types);
    let types = ui_row(
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(32),
            gap: 8,
            cross_align: CrossAlign::Center,
            ..UiStyle::default()
        },
        types,
    );
    let toggle = ui_button_with_options(
        &POKEDEX_THEME,
        UiStyle {
            width: Dimension::Px(30),
            height: Dimension::Px(28),
            main_align: MainAlign::Center,
            cross_align: CrossAlign::Center,
            padding: Insets::symmetric(3, 2),
            border_radius: POKEDEX_THEME.small_radius,
            ..UiStyle::default()
        },
        ButtonOptions::new(
            matches!(pokedex.stats_view, PokedexStatsView::Hexagon),
            false,
        ),
        [text_node(
            match pokedex.stats_view {
                PokedexStatsView::Bars => "[=]",
                PokedexStatsView::Hexagon => "[o]",
            },
            TextTone::Selected,
            12,
        )],
    )
    .with_key(UiKey::new("page-pokedex-stats-toggle")?)
    .with_action(PageIntent::TogglePokedexStatsView);
    let identity = ui_column(
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(86),
            gap: 2,
            main_align: MainAlign::Center,
            ..UiStyle::default()
        },
        [
            text_node(number_text(selected.number), TextTone::Muted, 14),
            text_node(
                name_or_unknown(selected.name.as_deref()),
                TextTone::Default,
                30,
            ),
            ui_row(
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(32),
                    gap: 8,
                    cross_align: CrossAlign::Center,
                    ..UiStyle::default()
                },
                [types, toggle],
            ),
        ],
    );
    let chart = ui_panel(
        &POKEDEX_THEME,
        PanelTone::Panel,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            padding: Insets::all(20),
            clip: true,
            ..UiStyle::default()
        },
        [stat_chart(
            &POKEDEX_THEME,
            selected_stats_view(pokedex.stats_view),
            stats,
        )],
    );
    Ok(ui_row(
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            padding: Insets::all(28),
            gap: 28,
            ..UiStyle::default()
        },
        [
            rail::project(pokedex, hide_transition_icons),
            ui_column(
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    direction: FlexDirection::Column,
                    gap: 14,
                    main_align: MainAlign::Center,
                    ..UiStyle::default()
                },
                [identity, chart],
            ),
        ],
    ))
}
