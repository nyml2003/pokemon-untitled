use super::*;
use game_ui_kit::{
    PanelTone, StatChartValues, StatChartView, column as ui_column, panel as ui_panel,
    row as ui_row, stat_chart,
};
use punctum_ui::{CrossAlign, Dimension, FlexDirection, Insets, MainAlign, UiNode, UiStyle};

pub(super) fn project_content(pokedex: &PokedexPageModel) -> UiNode<PageIntent> {
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
                [types],
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
        [stat_chart(&POKEDEX_THEME, StatChartView::Hexagon, stats)],
    );
    let facts = ui_panel(
        &POKEDEX_THEME,
        PanelTone::Panel,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(58),
            padding: Insets::symmetric(12, 8),
            gap: 6,
            ..UiStyle::default()
        },
        [
            compact_text_node(
                selected.genus.as_deref().unwrap_or("分类 --"),
                TextTone::Muted,
                13,
            ),
            compact_text_node(
                selected.height_decimeters.map_or_else(
                    || String::from("身高 --"),
                    |value| format!("身高 {}.{}m", value / 10, value % 10),
                ),
                TextTone::Muted,
                13,
            ),
            compact_text_node(
                selected.weight_hectograms.map_or_else(
                    || String::from("体重 --"),
                    |value| format!("体重 {}.{}kg", value / 10, value % 10),
                ),
                TextTone::Muted,
                13,
            ),
            compact_text_node(
                selected
                    .abilities
                    .iter()
                    .map(|ability| ability.name.as_str())
                    .collect::<Vec<_>>()
                    .join(" / "),
                TextTone::Default,
                13,
            ),
        ],
    );
    ui_column(
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            direction: FlexDirection::Column,
            gap: 14,
            main_align: MainAlign::Center,
            ..UiStyle::default()
        },
        [identity, facts, chart],
    )
}
