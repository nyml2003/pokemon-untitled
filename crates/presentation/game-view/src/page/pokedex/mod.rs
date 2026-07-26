use super::super::assets::{move_category_icon_asset, type_icon_asset};
use super::super::common::FOUNDATION_THEME;
use super::assets::{page_pokedex_icon_asset, page_pokedex_pokemon_asset};
use super::common::{page_detail, page_notice};
use battle_session::{MoveCategory, PokemonType};
use game_assets::AssetKey;
use game_page_model::{
    NationalDexNumber, PageIntent, PokedexDetailView, PokedexMoveCategory, PokedexMoveModel,
    PokedexPageModel, PokedexSection, PokedexStatsView,
};
use game_ui::PokedexVisualState;
use game_ui_kit::{
    ButtonOptions, PanelTone, SpriteAppearance, StatChartValues, StatChartView, TextTone,
    button_with_options as ui_button_with_options, column as ui_column, image as ui_image,
    panel as ui_panel, row as ui_row, screen as ui_screen, sprite as ui_sprite, stack as ui_stack,
    stat_chart, text as ui_text,
};
use punctum_ui::{
    CrossAlign, Dimension, FlexDirection, Insets, KeyboardSingleColumnFixedHeightScrollView,
    MainAlign, UiBuildError, UiColor, UiContent, UiContentId, UiKey, UiNode, UiStyle, UiTree,
};

mod detail;
mod index;
mod moves;
mod stats;

const POKEDEX_VISIBLE_ITEMS: usize = 7;
const POKEDEX_ITEM_HEIGHT: u32 = 52;
const POKEDEX_MOVE_ITEM_HEIGHT: u32 = 48;

pub(super) fn project_pause_pokedex(
    pokedex: &PokedexPageModel,
    notice: Option<&str>,
    visual: Option<PokedexVisualState>,
    viewport: punctum_ui::UiSize,
) -> Result<UiTree<PageIntent>, UiBuildError> {
    let Some(visual) = visual else {
        return project_pause_pokedex_legacy(pokedex, notice, viewport);
    };
    project_pause_pokedex_track(pokedex, notice, visual, viewport)
}

fn project_pause_pokedex_track(
    pokedex: &PokedexPageModel,
    notice: Option<&str>,
    visual: PokedexVisualState,
    viewport: punctum_ui::UiSize,
) -> Result<UiTree<PageIntent>, UiBuildError> {
    let layers = [
        track_layer(
            PokedexSection::Index,
            visual,
            viewport,
            index::project(pokedex)?,
        ),
        track_layer(
            PokedexSection::Detail,
            visual,
            viewport,
            detail::project(pokedex),
        ),
        track_layer(
            PokedexSection::Stats,
            visual,
            viewport,
            stats::project(pokedex)?,
        ),
        track_layer(
            PokedexSection::Moves,
            visual,
            viewport,
            moves::project(pokedex)?,
        ),
    ];
    UiTree::new(ui_screen(
        &FOUNDATION_THEME,
        [
            ui_stack(
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    clip: true,
                    ..UiStyle::default()
                },
                layers,
            ),
            page_notice(notice),
        ],
    ))
}

fn track_layer(
    section: PokedexSection,
    visual: PokedexVisualState,
    viewport: punctum_ui::UiSize,
    child: UiNode<PageIntent>,
) -> UiNode<PageIntent> {
    let section_position = match section {
        PokedexSection::Index => 0,
        PokedexSection::Detail => 1000,
        PokedexSection::Stats => 2000,
        PokedexSection::Moves => 3000,
    };
    let delta = i64::from(section_position) - i64::from(visual.position);
    let offset = (i64::from(viewport.width.max(1)) * delta / 1000)
        .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
    let background_offset = scaled_offset(offset, 35);
    let structure_offset = scaled_offset(offset, 30);
    let content_offset = scaled_offset(offset, 35);
    UiNode::auto()
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            position: punctum_ui::Position::Absolute { left: 0, top: 0 },
            visual_offset: punctum_ui::UiPixelOffset::new(background_offset, 0),
            ..UiStyle::default()
        })
        .with_content(UiContent::Fill(UiColor::new(
            FOUNDATION_THEME.panel.red,
            FOUNDATION_THEME.panel.green,
            FOUNDATION_THEME.panel.blue,
            72,
        )))
        .with_children([UiNode::auto()
            .with_style(UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                position: punctum_ui::Position::Absolute { left: 0, top: 0 },
                visual_offset: punctum_ui::UiPixelOffset::new(structure_offset, 0),
                ..UiStyle::default()
            })
            .with_children([UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    position: punctum_ui::Position::Absolute { left: 0, top: 0 },
                    visual_offset: punctum_ui::UiPixelOffset::new(content_offset, 0),
                    ..UiStyle::default()
                })
                .with_children([child])])])
}

fn scaled_offset(offset: i32, percentage: i32) -> i32 {
    ((i64::from(offset) * i64::from(percentage)) / 100)
        .clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

fn pokedex_index_page(pokedex: &PokedexPageModel) -> Result<UiNode<PageIntent>, UiBuildError> {
    let selected_number = pokedex.selected.number;
    let selected_index = pokedex
        .entries
        .iter()
        .position(|entry| entry.number == selected_number)
        .unwrap_or(0);
    let mut scroll = KeyboardSingleColumnFixedHeightScrollView::new(
        pokedex.entries.len(),
        POKEDEX_VISIBLE_ITEMS,
        POKEDEX_ITEM_HEIGHT,
    )
    .with_gap(4)
    .with_overscan(2);
    scroll.select(selected_index);
    let rows = scroll
        .render_range()
        .filter_map(|index| pokedex.entries.get(index))
        .map(|entry| {
            pokedex_index_row(
                entry.number,
                entry.number == selected_number,
                entry.known,
                format!("page-pokedex-index-{}", entry.number.value()),
                Some(PageIntent::SelectPokedexEntry(entry.number)),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let name = pokedex.selected.name.as_deref().unwrap_or("???");
    let selected = pokedex_sprite(selected_number.value(), 280, 280, pokedex.selected.known);
    Ok(ui_panel(
        &FOUNDATION_THEME,
        PanelTone::Screen,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            direction: FlexDirection::Column,
            gap: 12,
            padding: Insets::all(20),
            ..UiStyle::default()
        },
        [
            ui_text(
                &FOUNDATION_THEME,
                TextTone::Default,
                "图鉴 / INDEX",
                22,
                Dimension::Fill,
            ),
            ui_row(
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    gap: 16,
                    ..UiStyle::default()
                },
                [
                    ui_panel(
                        &FOUNDATION_THEME,
                        PanelTone::Panel,
                        UiStyle {
                            width: Dimension::Ratio { units: 2, base: 5 },
                            height: Dimension::Fill,
                            direction: FlexDirection::Column,
                            gap: 4,
                            padding: Insets::all(8),
                            clip: true,
                            ..UiStyle::default()
                        },
                        [scroll.node(
                            UiStyle {
                                width: Dimension::Fill,
                                height: Dimension::Fill,
                                gap: 4,
                                ..UiStyle::default()
                            },
                            rows,
                        )],
                    ),
                    ui_panel(
                        &FOUNDATION_THEME,
                        PanelTone::ImageBackdrop,
                        UiStyle {
                            width: Dimension::Fill,
                            height: Dimension::Fill,
                            direction: FlexDirection::Column,
                            main_align: MainAlign::Center,
                            cross_align: CrossAlign::Center,
                            gap: 8,
                            clip: true,
                            ..UiStyle::default()
                        },
                        [
                            selected,
                            ui_text(
                                &FOUNDATION_THEME,
                                TextTone::Selected,
                                format!("NO.{:03}  {name}", selected_number.value()),
                                18,
                                Dimension::Fill,
                            ),
                        ],
                    ),
                ],
            ),
        ],
    ))
}

fn pokedex_detail_page(pokedex: &PokedexPageModel) -> UiNode<PageIntent> {
    let selected_number = pokedex.selected.number;
    let name = pokedex.selected.name.as_deref().unwrap_or("???");
    let type_icons = pokedex
        .selected
        .types
        .iter()
        .filter_map(|name| pokedex_type_asset(name))
        .map(|asset| {
            ui_image(
                UiContentId::from_resource_key(asset.as_str()),
                UiStyle::fixed(64, 32),
            )
        })
        .collect::<Vec<_>>();
    ui_panel(
        &FOUNDATION_THEME,
        PanelTone::Screen,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            direction: FlexDirection::Column,
            gap: 12,
            padding: Insets::all(20),
            ..UiStyle::default()
        },
        [
            ui_text(
                &FOUNDATION_THEME,
                TextTone::Default,
                "图鉴 / DETAIL",
                22,
                Dimension::Fill,
            ),
            ui_row(
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    gap: 20,
                    ..UiStyle::default()
                },
                [
                    ui_panel(
                        &FOUNDATION_THEME,
                        PanelTone::ImageBackdrop,
                        UiStyle {
                            width: Dimension::Ratio { units: 2, base: 5 },
                            height: Dimension::Fill,
                            main_align: MainAlign::Center,
                            cross_align: CrossAlign::Center,
                            clip: true,
                            ..UiStyle::default()
                        },
                        [pokedex_sprite(
                            selected_number.value(),
                            360,
                            360,
                            pokedex.selected.known,
                        )],
                    ),
                    ui_column(
                        UiStyle {
                            width: Dimension::Fill,
                            height: Dimension::Fill,
                            gap: 8,
                            main_align: MainAlign::Center,
                            ..UiStyle::default()
                        },
                        [
                            ui_text(
                                &FOUNDATION_THEME,
                                TextTone::MutedInk,
                                format!("NO.{:03}", selected_number.value()),
                                18,
                                Dimension::Fill,
                            ),
                            ui_text(&FOUNDATION_THEME, TextTone::Ink, name, 36, Dimension::Fill),
                            ui_row(
                                UiStyle {
                                    width: Dimension::Fill,
                                    height: Dimension::Px(32),
                                    gap: 8,
                                    ..UiStyle::default()
                                },
                                if type_icons.is_empty() {
                                    vec![ui_text(
                                        &FOUNDATION_THEME,
                                        TextTone::MutedInk,
                                        "属性数据未记录",
                                        15,
                                        Dimension::Fill,
                                    )]
                                } else {
                                    type_icons
                                },
                            ),
                            ui_text(
                                &FOUNDATION_THEME,
                                TextTone::MutedInk,
                                if pokedex.selected.known {
                                    "已发现 / 已记录"
                                } else {
                                    "未发现 / 仅保留轮廓"
                                },
                                16,
                                Dimension::Fill,
                            ),
                            pokedex_progress_panel(pokedex),
                        ],
                    ),
                ],
            ),
        ],
    )
}

fn pokedex_stats_page(pokedex: &PokedexPageModel) -> Result<UiNode<PageIntent>, UiBuildError> {
    let selected_number = pokedex.selected.number;
    let name = pokedex.selected.name.as_deref().unwrap_or("???");
    let values = pokedex.selected.stats.map(|stats| StatChartValues {
        hp: stats.hp,
        attack: stats.attack,
        defense: stats.defense,
        special_attack: stats.special_attack,
        special_defense: stats.special_defense,
        speed: stats.speed,
    });
    Ok(ui_panel(
        &FOUNDATION_THEME,
        PanelTone::Screen,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            direction: FlexDirection::Column,
            gap: 12,
            padding: Insets::all(20),
            ..UiStyle::default()
        },
        [
            ui_row(
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(42),
                    main_align: MainAlign::SpaceBetween,
                    cross_align: CrossAlign::Center,
                    ..UiStyle::default()
                },
                [
                    ui_text(
                        &FOUNDATION_THEME,
                        TextTone::Default,
                        format!("图鉴 / STATS  ·  NO.{:03} {name}", selected_number.value()),
                        22,
                        Dimension::Fill,
                    ),
                    pokedex_stats_toggle(pokedex.stats_view)?,
                ],
            ),
            ui_panel(
                &FOUNDATION_THEME,
                PanelTone::Panel,
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    padding: Insets::all(16),
                    clip: true,
                    ..UiStyle::default()
                },
                [stat_chart(
                    &FOUNDATION_THEME,
                    match pokedex.stats_view {
                        PokedexStatsView::Bars => StatChartView::Bars,
                        PokedexStatsView::Hexagon => StatChartView::Hexagon,
                    },
                    values,
                )],
            ),
        ],
    ))
}

fn project_pause_pokedex_legacy(
    pokedex: &PokedexPageModel,
    notice: Option<&str>,
    viewport: punctum_ui::UiSize,
) -> Result<UiTree<PageIntent>, UiBuildError> {
    let _ = viewport;
    let selected_number = pokedex.selected.number;
    let selected_index = pokedex
        .entries
        .iter()
        .position(|entry| entry.number == selected_number)
        .unwrap_or(0);
    let mut index_scroll = KeyboardSingleColumnFixedHeightScrollView::new(
        pokedex.entries.len(),
        POKEDEX_VISIBLE_ITEMS,
        POKEDEX_ITEM_HEIGHT,
    )
    .with_gap(2)
    .with_overscan(2);
    index_scroll.select(selected_index);
    let index_rows = index_scroll
        .render_range()
        .filter_map(|index| pokedex.entries.get(index))
        .map(|entry| {
            pokedex_index_row(
                entry.number,
                entry.number == selected_number,
                entry.known,
                format!("page-pokedex-index-{}", entry.number.value()),
                Some(PageIntent::SelectPokedexEntry(entry.number)),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let index_scroll = index_scroll.node(
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            gap: 2,
            ..UiStyle::default()
        },
        index_rows,
    );
    let name = pokedex.selected.name.as_deref().unwrap_or("???");
    let type_icons = pokedex
        .selected
        .types
        .iter()
        .filter_map(|name| pokedex_type_asset(name))
        .map(|asset| {
            ui_image(
                UiContentId::from_resource_key(asset.as_str()),
                // 属性 PNG 是 32x16，保持 2:1 比例，避免被 UI 拉宽。
                UiStyle::fixed(64, 32),
            )
        })
        .collect::<Vec<_>>();
    let type_summary = if pokedex.selected.types.is_empty() {
        "属性数据未记录"
    } else {
        "属性数据已收录"
    };
    let status = if pokedex.selected.known {
        "已发现 / 已记录"
    } else {
        "未发现 / 仅保留轮廓"
    };
    let stat_values = pokedex.selected.stats.map(|stats| StatChartValues {
        hp: stats.hp,
        attack: stats.attack,
        defense: stats.defense,
        special_attack: stats.special_attack,
        special_defense: stats.special_defense,
        speed: stats.speed,
    });
    let stats_content = stat_chart(
        &FOUNDATION_THEME,
        match pokedex.stats_view {
            PokedexStatsView::Bars => StatChartView::Bars,
            PokedexStatsView::Hexagon => StatChartView::Hexagon,
        },
        stat_values,
    );
    let stats_toggle = pokedex_stats_toggle(pokedex.stats_view)?;
    let detail_toggle = pokedex_detail_toggle(pokedex.detail_view)?;
    let detail_body = match pokedex.detail_view {
        PokedexDetailView::Overview => pokedex_progress_panel(pokedex),
        PokedexDetailView::Moves => pokedex_moves_panel(pokedex)?,
    };
    UiTree::new(ui_screen(
        &FOUNDATION_THEME,
        [ui_panel(
            &FOUNDATION_THEME,
            PanelTone::Screen,
            UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                direction: FlexDirection::Column,
                gap: 12,
                padding: Insets::all(20),
                ..UiStyle::default()
            },
            [
                ui_row(
                    UiStyle {
                        width: Dimension::Fill,
                        height: Dimension::Px(46),
                        main_align: MainAlign::SpaceBetween,
                        cross_align: CrossAlign::Center,
                        ..UiStyle::default()
                    },
                    [
                        ui_text(
                            &FOUNDATION_THEME,
                            TextTone::Default,
                            "图鉴 / FIELD GUIDE",
                            22,
                            Dimension::Fill,
                        ),
                        ui_text(
                            &FOUNDATION_THEME,
                            TextTone::Muted,
                            format!(
                                "{:03}  ·  {}/{}",
                                selected_number.value(),
                                pokedex.known_count,
                                pokedex.total_count
                            ),
                            16,
                            Dimension::Px(180),
                        ),
                    ],
                ),
                ui_row(
                    UiStyle {
                        width: Dimension::Fill,
                        height: Dimension::Fill,
                        gap: 16,
                        ..UiStyle::default()
                    },
                    [
                        ui_panel(
                            &FOUNDATION_THEME,
                            PanelTone::Panel,
                            UiStyle {
                                width: Dimension::Ratio { units: 1, base: 4 },
                                height: Dimension::Fill,
                                direction: FlexDirection::Column,
                                gap: 4,
                                padding: Insets::all(6),
                                clip: true,
                                ..UiStyle::default()
                            },
                            [
                                ui_text(
                                    &FOUNDATION_THEME,
                                    TextTone::Muted,
                                    "INDEX",
                                    13,
                                    Dimension::Fill,
                                ),
                                ui_column(
                                    UiStyle {
                                        width: Dimension::Fill,
                                        height: Dimension::Fill,
                                        gap: 2,
                                        ..UiStyle::default()
                                    },
                                    [index_scroll],
                                ),
                            ],
                        ),
                        ui_panel(
                            &FOUNDATION_THEME,
                            PanelTone::Card,
                            UiStyle {
                                width: Dimension::Fill,
                                height: Dimension::Fill,
                                direction: FlexDirection::Column,
                                gap: 10,
                                padding: Insets::all(12),
                                ..UiStyle::default()
                            },
                            [
                                detail_toggle,
                                if pokedex.detail_view == PokedexDetailView::Overview {
                                    ui_row(
                                        UiStyle {
                                            width: Dimension::Fill,
                                            height: Dimension::Fill,
                                            gap: 12,
                                            ..UiStyle::default()
                                        },
                                        [
                                            ui_panel(
                                                &FOUNDATION_THEME,
                                                PanelTone::ImageBackdrop,
                                                UiStyle {
                                                    width: Dimension::Ratio { units: 2, base: 5 },
                                                    height: Dimension::Px(300),
                                                    cross_align: CrossAlign::Center,
                                                    main_align: MainAlign::Center,
                                                    clip: true,
                                                    ..UiStyle::default()
                                                },
                                                [pokedex_sprite(
                                                    selected_number.value(),
                                                    280,
                                                    280,
                                                    pokedex.selected.known,
                                                )],
                                            ),
                                            ui_column(
                                                UiStyle {
                                                    width: Dimension::Fill,
                                                    height: Dimension::Fill,
                                                    gap: 0,
                                                    ..UiStyle::default()
                                                },
                                                [
                                                    ui_text(
                                                        &FOUNDATION_THEME,
                                                        TextTone::MutedInk,
                                                        format!(
                                                            "NO.{:03}",
                                                            selected_number.value()
                                                        ),
                                                        18,
                                                        Dimension::Fill,
                                                    ),
                                                    ui_text(
                                                        &FOUNDATION_THEME,
                                                        TextTone::Ink,
                                                        name,
                                                        36,
                                                        Dimension::Fill,
                                                    ),
                                                    ui_text(
                                                        &FOUNDATION_THEME,
                                                        TextTone::MutedInk,
                                                        status,
                                                        16,
                                                        Dimension::Fill,
                                                    ),
                                                    ui_row(
                                                        UiStyle {
                                                            width: Dimension::Fill,
                                                            height: Dimension::Px(32),
                                                            gap: 8,
                                                            ..UiStyle::default()
                                                        },
                                                        [
                                                            ui_row(
                                                                UiStyle {
                                                                    width: Dimension::Fill,
                                                                    height: Dimension::Px(32),
                                                                    gap: 8,
                                                                    ..UiStyle::default()
                                                                },
                                                                if type_icons.is_empty() {
                                                                    vec![ui_text(
                                                                        &FOUNDATION_THEME,
                                                                        TextTone::MutedInk,
                                                                        type_summary,
                                                                        15,
                                                                        Dimension::Fill,
                                                                    )]
                                                                } else {
                                                                    type_icons
                                                                },
                                                            ),
                                                            stats_toggle,
                                                        ],
                                                    ),
                                                    ui_text(
                                                        &FOUNDATION_THEME,
                                                        TextTone::MutedInk,
                                                        "BASE STATS",
                                                        13,
                                                        Dimension::Fill,
                                                    ),
                                                    stats_content,
                                                ],
                                            ),
                                        ],
                                    )
                                } else {
                                    UiNode::auto().with_style(UiStyle {
                                        width: Dimension::Fill,
                                        height: Dimension::Px(0),
                                        ..UiStyle::default()
                                    })
                                },
                                detail_body,
                            ],
                        ),
                    ],
                ),
                page_notice(notice),
            ],
        )],
    ))
}

fn pokedex_detail_toggle(view: PokedexDetailView) -> Result<UiNode<PageIntent>, UiBuildError> {
    let button = |label: &'static str,
                  key: &'static str,
                  target: PokedexDetailView|
     -> Result<UiNode<PageIntent>, UiBuildError> {
        Ok(ui_button_with_options(
            &FOUNDATION_THEME,
            UiStyle {
                width: Dimension::Px(104),
                height: Dimension::Px(30),
                main_align: MainAlign::Center,
                cross_align: CrossAlign::Center,
                padding: Insets::symmetric(8, 4),
                border_radius: FOUNDATION_THEME.small_radius,
                ..UiStyle::default()
            },
            ButtonOptions::new(view == target, false),
            [ui_text(
                &FOUNDATION_THEME,
                if view == target {
                    TextTone::Selected
                } else {
                    TextTone::Muted
                },
                label,
                13,
                Dimension::Fill,
            )],
        )
        .with_key(UiKey::new(key)?)
        .with_action(PageIntent::SelectPokedexDetail(target)))
    };

    Ok(ui_row(
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(34),
            gap: 4,
            ..UiStyle::default()
        },
        [
            button(
                "概要",
                "page-pokedex-detail-overview",
                PokedexDetailView::Overview,
            )?,
            button(
                "技能",
                "page-pokedex-detail-moves",
                PokedexDetailView::Moves,
            )?,
        ],
    ))
}

fn pokedex_progress_panel(pokedex: &PokedexPageModel) -> UiNode<PageIntent> {
    ui_panel(
        &FOUNDATION_THEME,
        PanelTone::Panel,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(58),
            direction: FlexDirection::Row,
            main_align: MainAlign::SpaceBetween,
            cross_align: CrossAlign::Center,
            padding: Insets::symmetric(12, 8),
            ..UiStyle::default()
        },
        [
            ui_text(
                &FOUNDATION_THEME,
                TextTone::Muted,
                "记录进度",
                15,
                Dimension::Px(90),
            ),
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(10),
                    border_radius: FOUNDATION_THEME.small_radius,
                    ..UiStyle::default()
                })
                .with_content(UiContent::Fill(UiColor::new(76, 112, 139, 255)))
                .with_children([UiNode::auto()
                    .with_style(UiStyle {
                        width: Dimension::Ratio {
                            units: pokedex.known_count as u32,
                            base: pokedex.total_count.max(1) as u32,
                        },
                        height: Dimension::Fill,
                        border_radius: FOUNDATION_THEME.small_radius,
                        ..UiStyle::default()
                    })
                    .with_content(UiContent::Fill(FOUNDATION_THEME.selected))]),
            ui_text(
                &FOUNDATION_THEME,
                TextTone::MutedInk,
                format!("{}/{}", pokedex.known_count, pokedex.total_count),
                15,
                Dimension::Px(72),
            ),
        ],
    )
}

fn pokedex_moves_panel(pokedex: &PokedexPageModel) -> Result<UiNode<PageIntent>, UiBuildError> {
    let mut scroll = KeyboardSingleColumnFixedHeightScrollView::new(
        pokedex.moves.len(),
        POKEDEX_VISIBLE_ITEMS,
        POKEDEX_MOVE_ITEM_HEIGHT,
    )
    .with_gap(4)
    .with_overscan(2);
    scroll.select(pokedex.selected_move);
    let rows = scroll
        .render_range()
        .filter_map(|index| pokedex.moves.get(index).map(|item| (index, item)))
        .map(|(index, item)| pokedex_move_row(index, item, index == scroll.selected_index()))
        .collect::<Result<Vec<_>, _>>()?;
    let list = if pokedex.moves.is_empty() {
        ui_panel(
            &FOUNDATION_THEME,
            PanelTone::Panel,
            UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                padding: Insets::all(12),
                ..UiStyle::default()
            },
            [ui_text(
                &FOUNDATION_THEME,
                TextTone::MutedInk,
                "没有可显示的技能记录",
                15,
                Dimension::Fill,
            )],
        )
    } else {
        scroll.node(
            UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                gap: 4,
                ..UiStyle::default()
            },
            rows,
        )
    };
    let selected = pokedex.moves.get(scroll.selected_index());
    let summary = selected.map_or_else(
        || page_detail("技能", "选择一项查看详情"),
        |item| page_detail(item.name.as_str(), format_move_details(item)),
    );
    Ok(ui_panel(
        &FOUNDATION_THEME,
        PanelTone::Panel,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            direction: FlexDirection::Column,
            gap: 8,
            padding: Insets::all(8),
            ..UiStyle::default()
        },
        [
            ui_text(
                &FOUNDATION_THEME,
                TextTone::MutedInk,
                format!(
                    "技能列表  {}/{}",
                    pokedex.selected_move.saturating_add(1),
                    pokedex.moves.len()
                ),
                14,
                Dimension::Fill,
            ),
            list,
            summary,
        ],
    ))
}

fn pokedex_move_row(
    index: usize,
    item: &PokedexMoveModel,
    selected: bool,
) -> Result<UiNode<PageIntent>, UiBuildError> {
    let node = ui_button_with_options(
        &FOUNDATION_THEME,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            direction: FlexDirection::Row,
            gap: 8,
            padding: Insets::symmetric(8, 4),
            border: punctum_ui::UiBorder {
                widths: Insets::all(1),
                color: UiColor::new(76, 112, 139, 255),
            },
            border_radius: FOUNDATION_THEME.small_radius,
            cross_align: CrossAlign::Center,
            ..UiStyle::default()
        },
        ButtonOptions::new(selected, false),
        [
            ui_image(
                UiContentId::from_resource_key(move_category_asset(item.category).as_str()),
                UiStyle::fixed(32, 32),
            ),
            ui_column(
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    gap: 2,
                    ..UiStyle::default()
                },
                [
                    ui_text(
                        &FOUNDATION_THEME,
                        if selected {
                            TextTone::Selected
                        } else {
                            TextTone::Default
                        },
                        format!("{:02}  {}", index.saturating_add(1), item.name),
                        15,
                        Dimension::Fill,
                    ),
                    ui_text(
                        &FOUNDATION_THEME,
                        TextTone::Muted,
                        format_move_details(item),
                        12,
                        Dimension::Fill,
                    ),
                ],
            ),
        ],
    )
    .with_key(UiKey::new(format!("page-pokedex-move-{index}"))?)
    .with_action(PageIntent::SelectPokedexMove(index));
    Ok(node)
}

fn move_category_asset(category: PokedexMoveCategory) -> AssetKey {
    let category = match category {
        PokedexMoveCategory::Physical => MoveCategory::Physical,
        PokedexMoveCategory::Special => MoveCategory::Special,
        PokedexMoveCategory::Status => MoveCategory::Status,
    };
    move_category_icon_asset(category)
}

fn format_move_details(item: &PokedexMoveModel) -> String {
    let power = item
        .power
        .map_or_else(|| String::from("威力 --"), |value| format!("威力 {value}"));
    let accuracy = item
        .accuracy
        .map_or_else(|| String::from("命中 --"), |value| format!("命中 {value}%"));
    let pp = item
        .pp
        .map_or_else(|| String::from("PP --"), |value| format!("PP {value}"));
    format!("{}  ·  {}  ·  {accuracy}  {pp}", item.move_type, power)
}

fn pokedex_index_row(
    number: NationalDexNumber,
    selected: bool,
    known: bool,
    key: impl Into<String>,
    action: Option<PageIntent>,
) -> Result<UiNode<PageIntent>, UiBuildError> {
    let key = key.into();
    let appearance = if known {
        SpriteAppearance::Plain
    } else {
        SpriteAppearance::Tinted(UiColor::new(28, 34, 45, 255))
    };
    let node = ui_button_with_options(
        &FOUNDATION_THEME,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            direction: FlexDirection::Row,
            gap: 8,
            padding: Insets::symmetric(6, 4),
            border: punctum_ui::UiBorder {
                widths: Insets::all(1),
                color: UiColor::new(76, 112, 139, 255),
            },
            border_radius: FOUNDATION_THEME.small_radius,
            cross_align: CrossAlign::Center,
            ..UiStyle::default()
        },
        ButtonOptions::new(selected, action.is_none()),
        [
            pokedex_icon_with_appearance(number.value(), 36, 36, appearance),
            ui_text(
                &FOUNDATION_THEME,
                if selected {
                    TextTone::Selected
                } else {
                    TextTone::Default
                },
                format!("{:03}", number.value()),
                13,
                Dimension::Fill,
            ),
        ],
    )
    .with_key(UiKey::new(key)?);
    Ok(match action {
        Some(action) => node.with_action(action),
        None => node,
    })
}

fn pokedex_stats_toggle(view: PokedexStatsView) -> Result<UiNode<PageIntent>, UiBuildError> {
    let label = match view {
        PokedexStatsView::Bars => "BAR",
        PokedexStatsView::Hexagon => "HEX",
    };
    Ok(ui_button_with_options(
        &FOUNDATION_THEME,
        UiStyle {
            width: Dimension::Px(72),
            height: Dimension::Px(28),
            padding: Insets::symmetric(8, 4),
            border_radius: FOUNDATION_THEME.small_radius,
            ..UiStyle::default()
        },
        ButtonOptions::new(matches!(view, PokedexStatsView::Hexagon), false),
        [ui_text(
            &FOUNDATION_THEME,
            TextTone::Selected,
            label,
            12,
            Dimension::Fill,
        )],
    )
    .with_key(UiKey::new("page-pokedex-stats-toggle")?)
    .with_action(PageIntent::TogglePokedexStatsView))
}

fn pokedex_sprite(number: u16, width: u32, height: u32, known: bool) -> UiNode<PageIntent> {
    pokedex_sprite_with_appearance(
        number,
        width,
        height,
        if known {
            SpriteAppearance::Plain
        } else {
            SpriteAppearance::Tinted(UiColor::new(28, 34, 45, 255))
        },
    )
}

fn pokedex_sprite_with_appearance(
    number: u16,
    width: u32,
    height: u32,
    appearance: SpriteAppearance,
) -> UiNode<PageIntent> {
    match page_pokedex_pokemon_asset(number) {
        Some(asset) => ui_sprite(
            UiContentId::from_resource_key(asset.as_str()),
            UiStyle::fixed(width, height),
            appearance,
        ),
        None => UiNode::auto().with_style(UiStyle::fixed(width, height)),
    }
}

fn pokedex_icon_with_appearance(
    number: u16,
    width: u32,
    height: u32,
    appearance: SpriteAppearance,
) -> UiNode<PageIntent> {
    match page_pokedex_icon_asset(number) {
        Some(asset) => ui_sprite(
            UiContentId::from_resource_key(asset.as_str()),
            UiStyle::fixed(width, height),
            appearance,
        ),
        None => UiNode::auto().with_style(UiStyle::fixed(width, height)),
    }
}

fn pokedex_type_asset(name: &str) -> Option<AssetKey> {
    let pokemon_type = match name {
        "一般" | "Normal" => PokemonType::Normal,
        "格斗" | "Fighting" => PokemonType::Fighting,
        "飞行" | "Flying" => PokemonType::Flying,
        "毒" | "Poison" => PokemonType::Poison,
        "地面" | "Ground" => PokemonType::Ground,
        "岩石" | "Rock" => PokemonType::Rock,
        "虫" | "Bug" => PokemonType::Bug,
        "幽灵" | "Ghost" => PokemonType::Ghost,
        "钢" | "Steel" => PokemonType::Steel,
        "火" | "Fire" => PokemonType::Fire,
        "水" | "Water" => PokemonType::Water,
        "草" | "Grass" => PokemonType::Grass,
        "电" | "Electric" => PokemonType::Electric,
        "超能力" | "Psychic" => PokemonType::Psychic,
        "冰" | "Ice" => PokemonType::Ice,
        "龙" | "Dragon" => PokemonType::Dragon,
        "恶" | "Dark" => PokemonType::Dark,
        _ => return None,
    };
    Some(type_icon_asset(pokemon_type))
}
