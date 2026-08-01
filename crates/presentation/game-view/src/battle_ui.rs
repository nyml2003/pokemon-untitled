//! 战斗界面、旧图鉴界面和命令控制台 UI 投影。

use super::{
    active_pokemon, battle_animation, battle_message, creature_tint, prompt_data,
    visible_console_start,
};
use super::{assets::*, common::*};
use battle_session::{
    Action, BattleObservation, BattleSessionSnapshot, BattleUnit, DamageProjection, HitPoints,
    HitPointsPhase, MoveCategory, Participant, PokemonType, TypeEffectiveness, Weather,
    WeatherState,
};
use game_data::PokedexData;
use game_ui::{BattleMenuPage, BattleUiState, CommandConsoleView, PokedexAction};
use game_ui_kit::{
    PanelTone, SpriteAppearance, TextTone, button as ui_button, column as ui_column,
    image as ui_image, modal as ui_modal, panel as ui_panel, row as ui_row, screen as ui_screen,
    selectable_list_item as ui_selectable_list_item, sprite as ui_sprite, text as ui_text,
};
use punctum_gpu::Rgba8;
use punctum_ui::{
    CrossAlign, Dimension, FlexDirection, Insets, MainAlign, Position, UiBuildError, UiColor,
    UiContent, UiContentId, UiKey, UiNode, UiStyle, UiTree,
};

/// 属性图标显示尺寸：真实资源 32x16（2:1）放大两倍。
const TYPE_ICON_SIZE: (u32, u32) = (56, 28);
/// 招式分类图标显示尺寸：真实资源 32x14（16:7）放大两倍。
const CATEGORY_ICON_SIZE: (u32, u32) = (64, 28);
/// 构建响应式像素 UI 图鉴树。
/// 图鉴是独立页面而非地图表面，因此不会投影为 `GameView`。
pub fn project_pokedex(
    pokedex: &PokedexData,
    selected_index: usize,
) -> Result<UiTree<PokedexAction>, UiBuildError> {
    let entries = pokedex.entries();
    let selected_index = selected_index.min(entries.len().saturating_sub(1));
    let entry = &entries[selected_index];
    let first = selected_index
        .saturating_sub(2)
        .min(entries.len().saturating_sub(5));
    let mut list_children = Vec::new();
    for (row, candidate) in entries.iter().skip(first).take(5).enumerate() {
        let selected = first + row == selected_index;
        list_children.push(ui_selectable_list_item(
            &POKEDEX_THEME,
            UiStyle {
                width: Dimension::Fill,
                height: Dimension::Px(52),
                padding: Insets::symmetric(14, 10),
                border_radius: POKEDEX_THEME.small_radius,
                ..UiStyle::default()
            },
            selected,
            UiKey::new(format!("pokedex-entry-{}", candidate.national_dex))?,
            PokedexAction::SelectEntry { index: first + row },
            [ui_text(
                &POKEDEX_THEME,
                if selected {
                    TextTone::Selected
                } else {
                    TextTone::Default
                },
                format!(
                    "{:03}  {}",
                    candidate.national_dex, candidate.localized_name
                ),
                19,
                Dimension::Fill,
            )],
        ));
    }
    let mut type_children = Vec::new();
    for kind in &entry.types {
        if let Some(pokemon_type) = pokedex_type(kind.id.0) {
            type_children.push(ui_image(
                UiContentId::new(type_icon_asset(pokemon_type).as_str())?,
                UiStyle::fixed(60, 30),
            ));
        } else {
            type_children.push(ui_text(
                &POKEDEX_THEME,
                TextTone::Ink,
                kind.name.clone(),
                16,
                Dimension::Px(90),
            ));
        }
    }
    UiTree::new(ui_screen(
        &POKEDEX_THEME,
        [
            ui_panel(
                &POKEDEX_THEME,
                PanelTone::Header,
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(76),
                    direction: FlexDirection::Row,
                    main_align: MainAlign::SpaceBetween,
                    cross_align: CrossAlign::Center,
                    padding: Insets::symmetric(32, 18),
                    border_radius: punctum_ui::UiBorderRadius {
                        top_left: 0,
                        top_right: 0,
                        bottom_right: 14,
                        bottom_left: 14,
                    },
                    ..UiStyle::default()
                },
                [
                    ui_text(
                        &POKEDEX_THEME,
                        TextTone::Default,
                        "宝可梦图鉴",
                        POKEDEX_THEME.title_text_size,
                        Dimension::Px(300),
                    ),
                    ui_text(
                        &POKEDEX_THEME,
                        TextTone::Muted,
                        format!("{}/{}", selected_index + 1, entries.len()),
                        18,
                        Dimension::Px(120),
                    ),
                ],
            ),
            ui_row(
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    gap: 20,
                    padding: Insets::all(24),
                    ..UiStyle::default()
                },
                [
                    ui_panel(
                        &POKEDEX_THEME,
                        PanelTone::Panel,
                        UiStyle {
                            width: Dimension::Px(300),
                            height: Dimension::Fill,
                            gap: 10,
                            padding: Insets::all(12),
                            border_radius: POKEDEX_THEME.medium_radius,
                            clip: true,
                            ..UiStyle::default()
                        },
                        list_children,
                    ),
                    ui_panel(
                        &POKEDEX_THEME,
                        PanelTone::Card,
                        UiStyle {
                            width: Dimension::Fill,
                            height: Dimension::Fill,
                            gap: POKEDEX_THEME.medium_spacing,
                            padding: Insets::all(28),
                            border_radius: POKEDEX_THEME.large_radius,
                            ..UiStyle::default()
                        },
                        [
                            ui_row(
                                UiStyle {
                                    width: Dimension::Fill,
                                    height: Dimension::Fill,
                                    gap: 28,
                                    ..UiStyle::default()
                                },
                                [
                                    ui_panel(
                                        &POKEDEX_THEME,
                                        PanelTone::ImageBackdrop,
                                        UiStyle {
                                            width: Dimension::Px(280),
                                            height: Dimension::Px(280),
                                            border_radius: POKEDEX_THEME.medium_radius,
                                            clip: true,
                                            ..UiStyle::default()
                                        },
                                        [ui_sprite(
                                            UiContentId::new(format!(
                                                "pokedex/{}",
                                                entry.national_dex
                                            ))?,
                                            UiStyle {
                                                width: Dimension::Fill,
                                                height: Dimension::Fill,
                                                border_radius: POKEDEX_THEME.medium_radius,
                                                ..UiStyle::default()
                                            },
                                            SpriteAppearance::Plain,
                                        )],
                                    ),
                                    ui_column(
                                        UiStyle {
                                            width: Dimension::Fill,
                                            height: Dimension::Fill,
                                            direction: FlexDirection::Column,
                                            gap: 12,
                                            ..UiStyle::default()
                                        },
                                        [
                                            ui_text(
                                                &POKEDEX_THEME,
                                                TextTone::Ink,
                                                format!("No.{:03}", entry.national_dex),
                                                22,
                                                Dimension::Fill,
                                            ),
                                            ui_text(
                                                &POKEDEX_THEME,
                                                TextTone::Ink,
                                                entry.localized_name.clone(),
                                                34,
                                                Dimension::Fill,
                                            ),
                                            ui_text(
                                                &POKEDEX_THEME,
                                                TextTone::MutedInk,
                                                entry.english_name.clone(),
                                                19,
                                                Dimension::Fill,
                                            ),
                                            ui_row(
                                                UiStyle {
                                                    width: Dimension::Fill,
                                                    height: Dimension::Px(36),
                                                    gap: POKEDEX_THEME.small_spacing,
                                                    ..UiStyle::default()
                                                },
                                                type_children,
                                            ),
                                        ],
                                    ),
                                ],
                            ),
                            ui_panel(
                                &POKEDEX_THEME,
                                PanelTone::Panel,
                                UiStyle {
                                    width: Dimension::Fill,
                                    height: Dimension::Px(96),
                                    gap: 10,
                                    padding: Insets::all(16),
                                    border_radius: POKEDEX_THEME.small_radius,
                                    ..UiStyle::default()
                                },
                                [
                                    ui_text(
                                        &POKEDEX_THEME,
                                        TextTone::Default,
                                        format!(
                                            "HP {:>3}    ATK {:>3}    DEF {:>3}",
                                            entry.base_stats.hp,
                                            entry.base_stats.attack,
                                            entry.base_stats.defense
                                        ),
                                        POKEDEX_THEME.body_text_size,
                                        Dimension::Fill,
                                    ),
                                    ui_text(
                                        &POKEDEX_THEME,
                                        TextTone::Default,
                                        format!(
                                            "SPA {:>3}    SPD {:>3}    SPE {:>3}",
                                            entry.base_stats.special_attack,
                                            entry.base_stats.special_defense,
                                            entry.base_stats.speed
                                        ),
                                        POKEDEX_THEME.body_text_size,
                                        Dimension::Fill,
                                    ),
                                ],
                            ),
                        ],
                    ),
                ],
            ),
        ],
    ))
}

/// 构建响应式像素 UI 战斗页面。
pub fn project_battle_ui(
    snapshot: &BattleSessionSnapshot,
    ui: BattleUiState,
    sprites: BattleSpriteResources,
    sprite_frame: usize,
) -> Result<UiTree, UiBuildError> {
    let scene = snapshot.scene();
    let own = scene.own();
    let opponent = scene.opponent();
    let (page, selected, notice) = ui.view();
    let message = notice
        .map(str::to_owned)
        .unwrap_or_else(|| battle_message(snapshot));
    let animation = battle_animation(snapshot.cue());
    let prompt = prompt_data(snapshot.interaction());
    let actions = prompt.map_or(&[][..], |(_, actions)| actions);
    let observation = prompt.map(|(observation, _)| observation);

    if page == BattleMenuPage::Pokemon {
        let root = observation.map_or_else(
            || battle_unavailable_page(&message),
            |observation| battle_pokemon_page_ui(observation, selected, &message, sprite_frame),
        );
        return UiTree::new(root);
    }

    let menu = match page {
        BattleMenuPage::Main => battle_main_actions_flex(selected),
        BattleMenuPage::Fight if actions.contains(&Action::Struggle) => {
            battle_move_menu(selected, [BattleMoveMenuItem::struggle()])
        }
        BattleMenuPage::Fight => {
            let items = battle_move_items(observation);
            battle_move_menu(selected, items)
        }
        BattleMenuPage::Pokemon => UiNode::auto(),
        BattleMenuPage::Hidden => UiNode::auto(),
    };
    // 战斗菜单选中招式时，在对方血条上标出命中后可能剩余的 HP 区间。
    let opponent_preview = if page == BattleMenuPage::Fight && !actions.contains(&Action::Struggle)
    {
        observation
            .and_then(|observation| observation.own().active_move_projections().get(selected))
            .filter(|projection| projection.min_percent() > 0 || projection.max_percent() > 0)
            .map(|projection| (projection.min_percent(), projection.max_percent()))
    } else {
        None
    };

    UiTree::new(panel(
        8_000,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            direction: FlexDirection::Column,
            ..UiStyle::default()
        },
        SKY.into_ui(),
        [
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    direction: FlexDirection::Column,
                    padding: Insets::all(20),
                    gap: 18,
                    ..UiStyle::default()
                })
                .with_content(UiContent::Fill(DISTANT_GRASS.into_ui()))
                .with_children([
                    UiNode::auto()
                        .with_style(UiStyle {
                            width: Dimension::Fill,
                            height: Dimension::Fill,
                            direction: FlexDirection::Row,
                            main_align: MainAlign::SpaceBetween,
                            cross_align: CrossAlign::Center,
                            ..UiStyle::default()
                        })
                        .with_children([
                            battle_status_panel(
                                8_100,
                                opponent.name(),
                                opponent.level(),
                                if opponent_preview.is_some() {
                                    opponent.hp().lock()
                                } else {
                                    opponent.hp()
                                },
                                sprite_frame,
                                OPPONENT_ACCENT.into_ui(),
                                opponent.primary_type(),
                                opponent.secondary_type(),
                                opponent_preview,
                            ),
                            image(
                                8_110,
                                sprites.opponent[sprite_frame % 2].as_str(),
                                UiStyle {
                                    width: Dimension::Px(220),
                                    height: Dimension::Px(220),
                                    ..UiStyle::default()
                                },
                            )
                            .with_content(UiContent::ImageTinted {
                                content: UiContentId::new(
                                    sprites.opponent[sprite_frame % 2].as_str(),
                                )?,
                                tint: creature_tint(animation, Participant::Opponent).into_ui(),
                            }),
                        ]),
                    UiNode::auto()
                        .with_style(UiStyle {
                            width: Dimension::Fill,
                            height: Dimension::Fill,
                            direction: FlexDirection::Row,
                            main_align: MainAlign::SpaceBetween,
                            cross_align: CrossAlign::Center,
                            ..UiStyle::default()
                        })
                        .with_children([
                            image(
                                8_120,
                                sprites.own[sprite_frame % 2].as_str(),
                                UiStyle {
                                    width: Dimension::Px(220),
                                    height: Dimension::Px(220),
                                    ..UiStyle::default()
                                },
                            )
                            .with_content(UiContent::ImageTinted {
                                content: UiContentId::new(sprites.own[sprite_frame % 2].as_str())?,
                                tint: creature_tint(animation, Participant::Own).into_ui(),
                            }),
                            battle_status_panel(
                                8_300,
                                own.name(),
                                own.level(),
                                own.hp(),
                                sprite_frame,
                                PLAYER_ACCENT.into_ui(),
                                own.primary_type(),
                                own.secondary_type(),
                                None,
                            ),
                        ]),
                ]),
            weather_overlay_node(snapshot.scene().weather(), sprite_frame)
                .unwrap_or_else(UiNode::auto),
            panel(
                8_200,
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(220),
                    direction: FlexDirection::Row,
                    padding: Insets::all(14),
                    border: punctum_ui::UiBorder {
                        widths: Insets::all(2),
                        color: ACTION_BORDER.into_ui(),
                    },
                    border_radius: punctum_ui::UiBorderRadius::all(16),
                    ..UiStyle::default()
                },
                ACTION_PANEL.into_ui(),
                [
                    UiNode::auto()
                        .with_style(UiStyle {
                            width: Dimension::Fill,
                            height: Dimension::Fill,
                            direction: FlexDirection::Column,
                            padding: Insets::all(12),
                            ..UiStyle::default()
                        })
                        .with_children([text(
                            8_202,
                            message,
                            MUTED_TEXT.into_ui(),
                            19,
                            Dimension::Fill,
                        )]),
                    UiNode::auto()
                        .with_style(UiStyle {
                            width: Dimension::Px(430),
                            height: Dimension::Fill,
                            ..UiStyle::default()
                        })
                        .with_children([menu]),
                ],
            ),
        ],
    ))
}

/// 将命令控制台投影为独立的响应式像素 UI 树。
/// 该树最多展示八个条目，并保留当前选中项的可见性。
pub fn project_console_ui(console: &CommandConsoleView) -> Result<UiTree, UiBuildError> {
    let first = visible_console_start(console.items.len(), console.selected_index);
    let mut rows = console
        .items
        .iter()
        .enumerate()
        .skip(first)
        .take(8)
        .map(|(index, item)| {
            console_item(
                8_500 + index as u32,
                item,
                console.selected_index == Some(index),
            )
        })
        .collect::<Vec<_>>();
    if rows.is_empty() {
        rows.push(text(
            8_500,
            "没有匹配指令",
            MUTED_TEXT.into_ui(),
            18,
            Dimension::Fill,
        ));
    }
    if let Some(diagnostic) = &console.diagnostic {
        rows.push(text(
            8_590,
            diagnostic.clone(),
            CONSOLE_ERROR.into_ui(),
            17,
            Dimension::Fill,
        ));
    }
    UiTree::new(
        UiNode::auto()
            .with_style(UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                direction: FlexDirection::Column,
                main_align: MainAlign::Center,
                cross_align: CrossAlign::Center,
                padding: Insets::all(36),
                ..UiStyle::default()
            })
            .with_children([panel(
                8_401,
                UiStyle {
                    width: Dimension::Px(880),
                    height: Dimension::Px(510),
                    direction: FlexDirection::Column,
                    gap: 12,
                    padding: Insets::all(24),
                    border: punctum_ui::UiBorder {
                        widths: Insets::all(2),
                        color: PANEL_EDGE.into_ui(),
                    },
                    border_radius: punctum_ui::UiBorderRadius::all(18),
                    ..UiStyle::default()
                },
                PANEL.into_ui(),
                [
                    text(
                        8_402,
                        format!("> {}{}", console.query, console.preedit),
                        TEXT.into_ui(),
                        21,
                        Dimension::Fill,
                    ),
                    UiNode::auto()
                        .with_style(UiStyle {
                            width: Dimension::Fill,
                            height: Dimension::Fill,
                            direction: FlexDirection::Column,
                            gap: 6,
                            clip: true,
                            ..UiStyle::default()
                        })
                        .with_children(rows),
                ],
            )]),
    )
}

trait UiColorExt {
    fn into_ui(self) -> UiColor;
}
impl UiColorExt for Rgba8 {
    fn into_ui(self) -> UiColor {
        UiColor::new(self.red, self.green, self.blue, self.alpha)
    }
}

fn battle_main_actions_flex(selected: usize) -> UiNode {
    let buttons = ["战斗", "宝可梦", "包包", "逃走"];
    let rows = (0_usize..2).map(|row| {
        ui_row(
            UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                ..UiStyle::default()
            },
            (0..2).map(|column| {
                let index = row * 2 + column;
                battle_main_action_button(
                    9_110 + index as u32,
                    buttons[index],
                    index == selected,
                    TypeEffectiveness::Normal,
                )
            }),
        )
    });
    ui_panel(
        &BATTLE_THEME,
        PanelTone::Panel,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            border_radius: BATTLE_THEME.large_radius,
            clip: true,
            ..UiStyle::default()
        },
        rows,
    )
}

fn battle_main_action_button(
    _id: u32,
    content: &str,
    selected: bool,
    effectiveness: TypeEffectiveness,
) -> UiNode {
    ui_button(
        &BATTLE_THEME,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            border_radius: BATTLE_THEME.medium_radius,
            shadow: effectiveness_annotation(effectiveness).1,
            ..UiStyle::default()
        },
        selected,
        [ui_text(
            &BATTLE_THEME,
            if selected {
                TextTone::Selected
            } else {
                TextTone::Default
            },
            content,
            BATTLE_THEME.body_text_size,
            Dimension::Fill,
        )],
    )
    .with_action(())
}

/// 招式菜单中一个招式的展示数据，含伤害预测与相性标注。
struct BattleMoveMenuItem {
    name: String,
    move_type: PokemonType,
    category: MoveCategory,
    power_detail: String,
    effectiveness: TypeEffectiveness,
    min_percent: u8,
    max_percent: u8,
}

impl BattleMoveMenuItem {
    fn struggle() -> Self {
        Self {
            name: "挣扎".to_owned(),
            move_type: PokemonType::Normal,
            category: MoveCategory::Physical,
            power_detail: "威50 PP--".to_owned(),
            effectiveness: TypeEffectiveness::Normal,
            min_percent: 0,
            max_percent: 0,
        }
    }
}

/// 汇总出战宝可梦前四个招式的展示数据，与观察到的伤害预测对齐。
fn battle_move_items(observation: Option<&BattleObservation>) -> Vec<BattleMoveMenuItem> {
    let Some(observation) = observation else {
        return Vec::new();
    };
    let projections = observation.own().active_move_projections();
    active_pokemon(observation)
        .moves()
        .iter()
        .take(4)
        .enumerate()
        .map(|(index, battle_move)| {
            let projection = projections.get(index);
            BattleMoveMenuItem {
                name: battle_move.name().to_owned(),
                move_type: battle_move
                    .move_types()
                    .first()
                    .copied()
                    .unwrap_or(PokemonType::Normal),
                category: battle_move.category(),
                power_detail: format!(
                    "威{} PP{}/{}",
                    battle_move.power(),
                    battle_move.current_pp(),
                    battle_move.max_pp()
                ),
                effectiveness: projection
                    .map_or(TypeEffectiveness::Normal, DamageProjection::effectiveness),
                min_percent: projection.map_or(0, DamageProjection::min_percent),
                max_percent: projection.map_or(0, DamageProjection::max_percent),
            }
        })
        .collect()
}

fn battle_move_menu(
    selected: usize,
    moves: impl IntoIterator<Item = BattleMoveMenuItem>,
) -> UiNode {
    let moves = moves.into_iter().collect::<Vec<_>>();
    let selected = selected.min(moves.len().saturating_sub(1));
    let detail = moves
        .get(selected)
        .map(|item| move_detail_panel(9_300, item));

    ui_row(
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            gap: BATTLE_THEME.medium_spacing,
            ..UiStyle::default()
        },
        [
            ui_column(
                UiStyle {
                    width: Dimension::Ratio { units: 3, base: 5 },
                    height: Dimension::Fill,
                    gap: BATTLE_THEME.small_spacing,
                    clip: true,
                    ..UiStyle::default()
                },
                moves.iter().enumerate().map(|(index, item)| {
                    battle_main_action_button(
                        9_120 + index as u32,
                        &item.name,
                        index == selected,
                        item.effectiveness,
                    )
                }),
            ),
            ui_column(
                UiStyle {
                    width: Dimension::Ratio { units: 2, base: 5 },
                    height: Dimension::Fill,
                    ..UiStyle::default()
                },
                detail,
            ),
        ],
    )
}

fn move_detail_panel(_id: u32, item: &BattleMoveMenuItem) -> UiNode {
    let mut rows = Vec::new();
    rows.push(ui_row(
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(28),
            gap: BATTLE_THEME.small_spacing,
            ..UiStyle::default()
        },
        [
            ui_image(
                UiContentId::from_resource_key(type_icon_asset(item.move_type).as_str()),
                UiStyle::fixed(TYPE_ICON_SIZE.0, TYPE_ICON_SIZE.1),
            ),
            ui_image(
                UiContentId::from_resource_key(move_category_icon_asset(item.category).as_str()),
                UiStyle::fixed(CATEGORY_ICON_SIZE.0, CATEGORY_ICON_SIZE.1),
            ),
        ],
    ));
    rows.push(ui_text(
        &BATTLE_THEME,
        TextTone::Ink,
        item.power_detail.clone(),
        17,
        Dimension::Fill,
    ));
    let (tone, shadow) = effectiveness_annotation(item.effectiveness);
    if let Some(label) = effectiveness_label(item.effectiveness) {
        rows.push(ui_text(
            &BATTLE_THEME,
            tone,
            label.to_owned(),
            16,
            Dimension::Fill,
        ));
    }
    if let Some(prediction) = damage_prediction(item) {
        rows.push(ui_text(
            &BATTLE_THEME,
            TextTone::MutedInk,
            prediction,
            15,
            Dimension::Fill,
        ));
    }
    ui_modal(
        &BATTLE_THEME,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            direction: FlexDirection::Column,
            gap: BATTLE_THEME.small_spacing,
            padding: Insets::all(BATTLE_THEME.medium_spacing),
            border_radius: BATTLE_THEME.medium_radius,
            shadow,
            ..UiStyle::default()
        },
        [
            ui_text(
                &BATTLE_THEME,
                TextTone::MutedInk,
                "招式详情",
                15,
                Dimension::Fill,
            ),
            ui_column(
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    gap: BATTLE_THEME.small_spacing,
                    ..UiStyle::default()
                },
                rows,
            ),
        ],
    )
}

/// 返回效果拔群/效果不济/无效的展示文案。
fn effectiveness_label(effectiveness: TypeEffectiveness) -> Option<&'static str> {
    match effectiveness {
        TypeEffectiveness::Quadruple | TypeEffectiveness::Double => Some("效果拔群"),
        TypeEffectiveness::Quarter | TypeEffectiveness::Half => Some("效果不济"),
        TypeEffectiveness::Immune => Some("没有效果"),
        TypeEffectiveness::Normal => None,
    }
}

/// 返回效果标注使用的文字色调与阴影样式。
fn effectiveness_annotation(effectiveness: TypeEffectiveness) -> (TextTone, punctum_ui::UiShadow) {
    let shadow_color = match effectiveness {
        TypeEffectiveness::Quadruple | TypeEffectiveness::Double => Rgba8::new(232, 64, 64, 120),
        TypeEffectiveness::Quarter | TypeEffectiveness::Half => Rgba8::new(86, 148, 232, 110),
        TypeEffectiveness::Immune => Rgba8::new(96, 96, 96, 110),
        TypeEffectiveness::Normal => Rgba8::new(0, 0, 0, 0),
    };
    let tone = match effectiveness {
        TypeEffectiveness::Quadruple | TypeEffectiveness::Double => TextTone::Selected,
        TypeEffectiveness::Quarter | TypeEffectiveness::Half => TextTone::MutedInk,
        TypeEffectiveness::Immune => TextTone::Muted,
        TypeEffectiveness::Normal => TextTone::Ink,
    };
    (
        tone,
        punctum_ui::UiShadow::new(
            shadow_color.into_ui(),
            0,
            0,
            if effectiveness == TypeEffectiveness::Normal {
                0
            } else {
                6
            },
        ),
    )
}

/// 返回伤害预测文案；状态类、零威力或无效招式返回 `None`。
fn damage_prediction(item: &BattleMoveMenuItem) -> Option<String> {
    match item.effectiveness {
        TypeEffectiveness::Immune => None,
        _ if item.min_percent == 0 && item.max_percent == 0 => Some("无伤害".to_owned()),
        _ => Some(format!("伤害约 {}-{}%", item.min_percent, item.max_percent)),
    }
}

fn battle_unavailable_page(message: &str) -> UiNode {
    panel(
        9_400,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            direction: FlexDirection::Column,
            main_align: MainAlign::Center,
            cross_align: CrossAlign::Center,
            padding: Insets::all(32),
            ..UiStyle::default()
        },
        PARTY_BG.into_ui(),
        [text(9_401, message, TEXT.into_ui(), 22, Dimension::Fill)],
    )
}

fn battle_pokemon_page_ui(
    observation: &BattleObservation,
    selected: usize,
    message: &str,
    sprite_frame: usize,
) -> UiNode {
    let members = observation.own().members();
    let selected = selected.min(members.len().saturating_sub(1));
    let selected_pokemon = &members[selected];
    let active_slot = observation.own().active_slot().index();

    panel(
        9_500,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            direction: FlexDirection::Column,
            padding: Insets::all(24),
            gap: 16,
            ..UiStyle::default()
        },
        PARTY_BG.into_ui(),
        [
            panel(
                9_501,
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(54),
                    direction: FlexDirection::Row,
                    main_align: MainAlign::SpaceBetween,
                    cross_align: CrossAlign::Center,
                    padding: Insets::symmetric(18, 10),
                    border_radius: punctum_ui::UiBorderRadius::all(12),
                    ..UiStyle::default()
                },
                PARTY_PANEL_ALT.into_ui(),
                [
                    text(9_502, "选择宝可梦", TEXT.into_ui(), 25, Dimension::Px(240)),
                    text(9_503, message, MUTED_TEXT.into_ui(), 16, Dimension::Fill),
                ],
            ),
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    direction: FlexDirection::Row,
                    gap: 16,
                    ..UiStyle::default()
                })
                .with_children([
                    selected_team_member_panel(
                        9_520,
                        selected,
                        selected_pokemon,
                        active_slot,
                        sprite_frame,
                    ),
                    UiNode::auto()
                        .with_style(UiStyle {
                            width: Dimension::Ratio { units: 3, base: 5 },
                            height: Dimension::Fill,
                            direction: FlexDirection::Column,
                            gap: 8,
                            clip: true,
                            ..UiStyle::default()
                        })
                        .with_children(members.iter().enumerate().map(|(index, pokemon)| {
                            team_member_card(
                                9_610 + index as u32 * 20,
                                index,
                                pokemon,
                                index == selected,
                                index == active_slot,
                                sprite_frame,
                            )
                        })),
                ]),
        ],
    )
}

fn selected_team_member_panel(
    id: u32,
    slot: usize,
    pokemon: &BattleUnit,
    active_slot: usize,
    sprite_frame: usize,
) -> UiNode {
    let mut types = vec![image(
        id + 5,
        type_icon_asset(
            pokemon
                .types()
                .first()
                .copied()
                .unwrap_or(PokemonType::Normal),
        )
        .as_str(),
        UiStyle::fixed(TYPE_ICON_SIZE.0, TYPE_ICON_SIZE.1),
    )];
    if let Some(secondary) = pokemon.types().get(1).copied() {
        types.push(image(
            id + 6,
            type_icon_asset(secondary).as_str(),
            UiStyle::fixed(TYPE_ICON_SIZE.0, TYPE_ICON_SIZE.1),
        ));
    }
    panel(
        id,
        UiStyle {
            width: Dimension::Ratio { units: 2, base: 5 },
            height: Dimension::Fill,
            direction: FlexDirection::Column,
            gap: 12,
            padding: Insets::all(20),
            border: punctum_ui::UiBorder {
                widths: Insets::all(2),
                color: PARTY_EDGE.into_ui(),
            },
            border_radius: punctum_ui::UiBorderRadius::all(14),
            ..UiStyle::default()
        },
        PARTY_PANEL.into_ui(),
        [
            image(
                id + 1,
                pokemon_icon_asset(slot, sprite_frame).as_str(),
                UiStyle::fixed(190, 190),
            )
            .with_content(UiContent::ImageTinted {
                content: UiContentId::from_resource_key(
                    pokemon_icon_asset(slot, sprite_frame).as_str(),
                ),
                tint: if pokemon.is_fainted() {
                    UiColor::new(112, 112, 112, 255)
                } else {
                    UiColor::new(255, 255, 255, 255)
                },
            }),
            text(id + 2, pokemon.name(), TEXT.into_ui(), 24, Dimension::Fill),
            text(
                id + 3,
                format!(
                    "Lv.{}{}",
                    pokemon.level(),
                    if slot == active_slot { "  出战" } else { "" }
                ),
                if slot == active_slot {
                    PLAYER_ACCENT.into_ui()
                } else {
                    MUTED_TEXT.into_ui()
                },
                17,
                Dimension::Fill,
            ),
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(28),
                    direction: FlexDirection::Row,
                    gap: 8,
                    ..UiStyle::default()
                })
                .with_children(types),
            hp_bar(id + 8, pokemon.state.hp(), sprite_frame),
            text(
                id + 12,
                if pokemon.is_fainted() {
                    "无法战斗".to_owned()
                } else {
                    format!(
                        "HP {}/{}（{}%）",
                        pokemon.current_hp(),
                        pokemon.max_hp(),
                        hp_percent(pokemon.current_hp(), pokemon.max_hp())
                    )
                },
                if pokemon.is_fainted() {
                    HP_LOW.into_ui()
                } else {
                    TEXT.into_ui()
                },
                17,
                Dimension::Fill,
            ),
        ],
    )
}

fn team_member_card(
    id: u32,
    slot: usize,
    pokemon: &BattleUnit,
    selected: bool,
    active: bool,
    sprite_frame: usize,
) -> UiNode {
    UiNode::auto()
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            direction: FlexDirection::Row,
            cross_align: CrossAlign::Center,
            gap: 12,
            padding: Insets::all(10),
            border: punctum_ui::UiBorder {
                widths: Insets::all(1),
                color: if selected { SELECTED } else { PARTY_EDGE }.into_ui(),
            },
            border_radius: punctum_ui::UiBorderRadius::all(10),
            ..UiStyle::default()
        })
        .with_action(())
        .with_content(UiContent::Fill(
            if selected {
                PARTY_PANEL_ALT
            } else {
                PARTY_PANEL
            }
            .into_ui(),
        ))
        .with_children([
            image(
                id + 1,
                pokemon_icon_asset(slot, sprite_frame).as_str(),
                UiStyle::fixed(54, 54),
            )
            .with_content(UiContent::ImageTinted {
                content: UiContentId::from_resource_key(
                    pokemon_icon_asset(slot, sprite_frame).as_str(),
                ),
                tint: if pokemon.is_fainted() {
                    UiColor::new(112, 112, 112, 255)
                } else {
                    UiColor::new(255, 255, 255, 255)
                },
            }),
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    direction: FlexDirection::Column,
                    main_align: MainAlign::Center,
                    gap: 4,
                    ..UiStyle::default()
                })
                .with_children([
                    text(
                        id + 3,
                        pokemon.name(),
                        if pokemon.is_fainted() {
                            MUTED_TEXT.into_ui()
                        } else {
                            TEXT.into_ui()
                        },
                        18,
                        Dimension::Fill,
                    ),
                    hp_bar(id + 4, pokemon.state.hp(), sprite_frame),
                ]),
            text(
                id + 9,
                if pokemon.is_fainted() {
                    "无法战斗".to_owned()
                } else if active {
                    "出战".to_owned()
                } else {
                    format!("Lv.{}", pokemon.level())
                },
                if pokemon.is_fainted() {
                    HP_LOW.into_ui()
                } else if active {
                    PLAYER_ACCENT.into_ui()
                } else {
                    MUTED_TEXT.into_ui()
                },
                16,
                Dimension::Px(82),
            ),
        ])
}

fn hp_bar(_id: u32, hp: HitPoints, frame: usize) -> UiNode {
    hp_track(hp, frame, None)
}

/// 渲染带律动特效与可选伤害预览的血条。
///
/// 双方血条常态律动：高光随阶段呼吸（高血慢、黄血中速、红血急促），
/// 红血时填充与边框随快节奏闪烁，倒下显示灰色；被锁定（选中招式瞄准）
/// 时高光叠加琥珀警示色。
/// `preview` 是选中招式对该宝可梦的最小/最大伤害百分比：
/// 命中后至少扣除 `min`（必扣带），随机 85-100 的差值形成浮动带，
/// 两段用不同透明度区分。
fn hp_track(hp: HitPoints, frame: usize, preview: Option<(u8, u8)>) -> UiNode {
    let max_hp = hp.max().max(1);
    let current = hp.current().min(max_hp);
    let fast_pulse = frame.is_multiple_of(2);
    let fill = hp_fill_color(hp, fast_pulse);
    let mut children = vec![
        UiNode::auto()
            .with_style(UiStyle {
                width: Dimension::Ratio {
                    units: current,
                    base: max_hp,
                },
                height: Dimension::Fill,
                border_radius: punctum_ui::UiBorderRadius::all(6),
                ..UiStyle::default()
            })
            .with_content(UiContent::Fill(fill.into_ui())),
    ];
    if let Some(gloss) = hp_gloss_color(hp, frame) {
        children.push(
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Ratio {
                        units: current.saturating_sub(4),
                        base: max_hp,
                    },
                    height: Dimension::Px(3),
                    margin: Insets {
                        top: 2,
                        left: 2,
                        right: 0,
                        bottom: 0,
                    },
                    border_radius: punctum_ui::UiBorderRadius::all(2),
                    ..UiStyle::default()
                })
                .with_content(UiContent::Fill(gloss.into_ui())),
        );
    }
    if let Some((min_percent, max_percent)) = preview {
        let damage_max = u32::from(max_percent) * max_hp / 100;
        let damage_min = u32::from(min_percent) * max_hp / 100;
        let float_start = current.saturating_sub(damage_max);
        let base_start = current.saturating_sub(damage_min);
        if base_start < current {
            children.push(
                UiNode::auto()
                    .with_style(UiStyle {
                        width: Dimension::Fill,
                        height: Dimension::Fill,
                        direction: FlexDirection::Row,
                        ..UiStyle::default()
                    })
                    .with_children([
                        UiNode::auto().with_style(UiStyle {
                            width: Dimension::Ratio {
                                units: float_start,
                                base: max_hp,
                            },
                            height: Dimension::Fill,
                            ..UiStyle::default()
                        }),
                        UiNode::auto()
                            .with_style(UiStyle {
                                width: Dimension::Ratio {
                                    units: base_start - float_start,
                                    base: max_hp,
                                },
                                height: Dimension::Fill,
                                border_radius: punctum_ui::UiBorderRadius::all(4),
                                ..UiStyle::default()
                            })
                            .with_content(UiContent::Fill(HP_PREVIEW_BAND.into_ui())),
                        UiNode::auto()
                            .with_style(UiStyle {
                                width: Dimension::Ratio {
                                    units: current - base_start,
                                    base: max_hp,
                                },
                                height: Dimension::Fill,
                                border_radius: punctum_ui::UiBorderRadius::all(4),
                                ..UiStyle::default()
                            })
                            .with_content(UiContent::Fill(HP_PREVIEW_BASE.into_ui())),
                    ]),
            );
        }
    }
    UiNode::auto()
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(14),
            direction: FlexDirection::Stack,
            padding: Insets::all(1),
            border: punctum_ui::UiBorder {
                widths: Insets::all(1),
                color: hp_border_color(hp, fast_pulse).into_ui(),
            },
            border_radius: punctum_ui::UiBorderRadius::all(7),
            ..UiStyle::default()
        })
        .with_content(UiContent::Fill(HP_TRACK_EDGE.into_ui()))
        .with_children(children)
}

/// 满血与高血使用绿色，黄血脉动使用黄色，红血按快节奏在红与亮红之间切换。
fn hp_fill_color(hp: HitPoints, fast_pulse: bool) -> Rgba8 {
    match hp.phase() {
        HitPointsPhase::Zero => Rgba8::new(96, 108, 116, 255),
        HitPointsPhase::Low if fast_pulse => HP_LOW_GLOW,
        HitPointsPhase::Low => HP_LOW,
        HitPointsPhase::Mid => HP_MID,
        HitPointsPhase::Full | HitPointsPhase::High => HP_GOOD,
    }
}

/// 高光条常态呼吸：高血慢、黄血中速、红血急促，全部阶段随帧周期亮暗；
/// 锁定时高光替换为琥珀警示色。
fn hp_gloss_color(hp: HitPoints, frame: usize) -> Option<Rgba8> {
    if hp.phase() == HitPointsPhase::Zero {
        return None;
    }
    let breathing = match hp.phase() {
        HitPointsPhase::Full | HitPointsPhase::High => frame.is_multiple_of(6),
        HitPointsPhase::Mid => frame.is_multiple_of(3),
        HitPointsPhase::Low => frame.is_multiple_of(2),
        HitPointsPhase::Zero => false,
    };
    let color = if hp.is_locked() {
        HP_PREVIEW_BAND
    } else {
        HP_GLOSS
    };
    breathing.then_some(color)
}

/// 边框色：红血随快节奏闪红，锁定时闪琥珀，其余保持轨道色。
fn hp_border_color(hp: HitPoints, fast_pulse: bool) -> Rgba8 {
    if hp.is_locked() {
        return if fast_pulse {
            HP_PREVIEW_BAND
        } else {
            HP_TRACK_EDGE
        };
    }
    match hp.phase() {
        HitPointsPhase::Low if fast_pulse => HP_LOW_GLOW,
        HitPointsPhase::Low => HP_LOW,
        _ => HP_TRACK_EDGE,
    }
}

/// 根据天气生成全屏覆盖节点；无天气时返回 `None`。
fn weather_overlay_node(weather: Option<WeatherState>, frame: usize) -> Option<UiNode> {
    let (pattern, color) = weather_effect(weather?.weather())?;
    Some(
        UiNode::auto()
            .with_style(UiStyle {
                position: Position::Absolute { left: 0, top: 0 },
                width: Dimension::Fill,
                height: Dimension::Fill,
                ..UiStyle::default()
            })
            .with_content(UiContent::Weather {
                pattern,
                frame: frame as u32,
                color: color.into_ui(),
            }),
    )
}

/// 把领域天气映射为天气图案码与覆盖颜色。
fn weather_effect(weather: Weather) -> Option<(u32, Rgba8)> {
    match weather {
        Weather::Rain => Some((0, Rgba8::new(120, 160, 220, 90))),
        Weather::Sandstorm => Some((1, Rgba8::new(205, 180, 110, 110))),
        Weather::Sun => Some((2, Rgba8::new(255, 220, 140, 80))),
        Weather::Hail => Some((3, Rgba8::new(180, 210, 235, 90))),
    }
}

#[allow(clippy::too_many_arguments)]
fn battle_status_panel(
    id: u32,
    name: &str,
    level: u8,
    hp: HitPoints,
    frame: usize,
    accent: UiColor,
    primary: PokemonType,
    secondary: Option<PokemonType>,
    preview: Option<(u8, u8)>,
) -> UiNode {
    let mut types = vec![image(
        id + 30,
        type_icon_asset(primary).as_str(),
        UiStyle::fixed(TYPE_ICON_SIZE.0, TYPE_ICON_SIZE.1),
    )];
    if let Some(secondary) = secondary {
        types.push(image(
            id + 31,
            type_icon_asset(secondary).as_str(),
            UiStyle::fixed(TYPE_ICON_SIZE.0, TYPE_ICON_SIZE.1),
        ));
    }
    panel(
        id,
        UiStyle {
            width: Dimension::Px(300),
            height: Dimension::Px(160),
            direction: FlexDirection::Column,
            gap: 8,
            padding: Insets::all(16),
            border_radius: punctum_ui::UiBorderRadius::all(14),
            ..UiStyle::default()
        },
        BATTLE_CARD.into_ui(),
        [
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(30),
                    direction: FlexDirection::Row,
                    main_align: MainAlign::SpaceBetween,
                    ..UiStyle::default()
                })
                .with_children([
                    text(id + 2, name, BATTLE_INK.into_ui(), 21, Dimension::Fill),
                    text(
                        id + 3,
                        format!("Lv.{level}"),
                        BATTLE_MUTED.into_ui(),
                        16,
                        Dimension::Px(64),
                    ),
                ]),
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(28),
                    direction: FlexDirection::Row,
                    gap: 6,
                    ..UiStyle::default()
                })
                .with_children(types),
            hp_track(hp, frame, preview),
            text(
                id + 7,
                format!("HP {}/{}（{}%）", hp.current(), hp.max(), hp.percent()),
                BATTLE_MUTED.into_ui(),
                15,
                Dimension::Fill,
            ),
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Px(12),
                    height: Dimension::Px(12),
                    border_radius: punctum_ui::UiBorderRadius::all(6),
                    ..UiStyle::default()
                })
                .with_content(UiContent::Fill(accent)),
        ],
    )
}

fn console_item(id: u32, content: &str, selected: bool) -> UiNode {
    UiNode::auto()
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(36),
            padding: Insets::symmetric(10, 6),
            border_radius: punctum_ui::UiBorderRadius::all(6),
            ..UiStyle::default()
        })
        .with_action(())
        .with_content(UiContent::Fill(
            if selected {
                SELECTED_DARK
            } else {
                ACTION_PANEL_ALT
            }
            .into_ui(),
        ))
        .with_children([text(
            id + 100,
            content,
            if selected { SELECTED } else { TEXT }.into_ui(),
            17,
            Dimension::Fill,
        )])
}
/// 当前 HP 占最大 HP 的整数百分比。
fn hp_percent(hp: u32, max_hp: u32) -> u32 {
    (u64::from(hp) * 100 / u64::from(max_hp.max(1))) as u32
}

fn panel(
    _id: u32,
    style: UiStyle,
    color: UiColor,
    children: impl IntoIterator<Item = UiNode>,
) -> UiNode {
    UiNode::auto()
        .with_style(style)
        .with_content(UiContent::Fill(color))
        .with_children(children)
}
fn text(
    _id: u32,
    content: impl Into<String>,
    color: UiColor,
    font_size: u32,
    width: Dimension,
) -> UiNode {
    UiNode::auto()
        .with_style(UiStyle {
            width,
            height: Dimension::Px(font_size.saturating_add(6)),
            ..UiStyle::default()
        })
        .with_content(UiContent::Text {
            content: content.into(),
            color,
            font_size,
        })
}
fn image(_id: u32, content: impl Into<String>, style: UiStyle) -> UiNode {
    UiNode::auto()
        .with_style(style)
        .with_content(UiContent::Image(UiContentId::from_resource_key(content)))
}
