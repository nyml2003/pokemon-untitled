//! 战斗界面、旧图鉴界面和命令控制台 UI 投影。

use super::{
    active_pokemon, battle_animation, battle_message, creature_tint, prompt_data,
    visible_console_start,
};
use super::{assets::*, common::*};
use battle_session::{
    Action, BattleObservation, BattleSessionSnapshot, MoveCategory, Participant, Pokemon,
    PokemonType,
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
    CrossAlign, Dimension, FlexDirection, Insets, MainAlign, UiBuildError, UiColor, UiContent,
    UiContentId, UiKey, UiNode, UiStyle, UiTree,
};
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
                UiStyle::fixed(88, 30),
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
        BattleMenuPage::Fight if actions.contains(&Action::Struggle) => battle_move_menu(
            selected,
            [(
                "挣扎".to_owned(),
                PokemonType::Normal,
                MoveCategory::Physical,
                "威50 PP--".to_owned(),
            )],
        ),
        BattleMenuPage::Fight => battle_move_menu(
            selected,
            observation
                .map(active_pokemon)
                .map_or(&[][..], |pokemon| pokemon.moves())
                .iter()
                .take(4)
                .map(|battle_move| {
                    (
                        battle_move.name().to_owned(),
                        battle_move.move_type(),
                        battle_move.category(),
                        format!(
                            "威{} PP{}/{}",
                            battle_move.power(),
                            battle_move.current_pp(),
                            battle_move.max_pp()
                        ),
                    )
                }),
        ),
        BattleMenuPage::Pokemon => UiNode::auto(),
        BattleMenuPage::Hidden => UiNode::auto(),
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
                                opponent.current_hp(),
                                opponent.max_hp(),
                                OPPONENT_ACCENT.into_ui(),
                                opponent.primary_type(),
                                opponent.secondary_type(),
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
                                own.current_hp(),
                                own.max_hp(),
                                PLAYER_ACCENT.into_ui(),
                                own.primary_type(),
                                own.secondary_type(),
                            ),
                        ]),
                ]),
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
                battle_main_action_button(9_110 + index as u32, buttons[index], index == selected)
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

fn battle_main_action_button(_id: u32, content: &str, selected: bool) -> UiNode {
    ui_button(
        &BATTLE_THEME,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            border_radius: BATTLE_THEME.medium_radius,
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

fn battle_move_menu(
    selected: usize,
    moves: impl IntoIterator<Item = (String, PokemonType, MoveCategory, String)>,
) -> UiNode {
    let moves = moves.into_iter().collect::<Vec<_>>();
    let selected = selected.min(moves.len().saturating_sub(1));
    let detail = moves.get(selected).map(|(_, move_type, category, detail)| {
        move_detail_panel(9_300, *move_type, *category, detail)
    });

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
                moves.iter().enumerate().map(|(index, (name, ..))| {
                    battle_main_action_button(9_120 + index as u32, name, index == selected)
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

fn move_detail_panel(
    _id: u32,
    move_type: PokemonType,
    category: MoveCategory,
    detail: &str,
) -> UiNode {
    ui_modal(
        &BATTLE_THEME,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            direction: FlexDirection::Column,
            gap: BATTLE_THEME.small_spacing,
            padding: Insets::all(BATTLE_THEME.medium_spacing),
            border_radius: BATTLE_THEME.medium_radius,
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
            ui_row(
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(28),
                    gap: BATTLE_THEME.small_spacing,
                    ..UiStyle::default()
                },
                [
                    ui_image(
                        UiContentId::from_resource_key(type_icon_asset(move_type).as_str()),
                        UiStyle::fixed(72, 28),
                    ),
                    ui_image(
                        UiContentId::from_resource_key(move_category_icon_asset(category).as_str()),
                        UiStyle::fixed(72, 28),
                    ),
                ],
            ),
            ui_text(&BATTLE_THEME, TextTone::Ink, detail, 17, Dimension::Fill),
        ],
    )
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
    pokemon: &Pokemon,
    active_slot: usize,
    sprite_frame: usize,
) -> UiNode {
    let mut types = vec![image(
        id + 5,
        type_icon_asset(pokemon.primary_type()).as_str(),
        UiStyle::fixed(72, 28),
    )];
    if let Some(secondary) = pokemon.secondary_type() {
        types.push(image(
            id + 6,
            type_icon_asset(secondary).as_str(),
            UiStyle::fixed(72, 28),
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
            hp_bar(id + 8, pokemon.current_hp(), pokemon.max_hp()),
            text(
                id + 12,
                if pokemon.is_fainted() {
                    "无法战斗".to_owned()
                } else {
                    format!("HP {}/{}", pokemon.current_hp(), pokemon.max_hp())
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
    pokemon: &Pokemon,
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
                    hp_bar(id + 4, pokemon.current_hp(), pokemon.max_hp()),
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

fn hp_bar(_id: u32, hp: u32, max_hp: u32) -> UiNode {
    UiNode::auto()
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(12),
            border_radius: punctum_ui::UiBorderRadius::all(6),
            ..UiStyle::default()
        })
        .with_content(UiContent::Fill(HP_TRACK_EDGE.into_ui()))
        .with_children([UiNode::auto()
            .with_style(UiStyle {
                width: Dimension::Ratio {
                    units: hp,
                    base: max_hp.max(1),
                },
                height: Dimension::Fill,
                border_radius: punctum_ui::UiBorderRadius::all(6),
                ..UiStyle::default()
            })
            .with_content(UiContent::Fill(hp_color(hp, max_hp).into_ui()))])
}

#[allow(clippy::too_many_arguments)]
fn battle_status_panel(
    id: u32,
    name: &str,
    level: u8,
    hp: u32,
    max_hp: u32,
    accent: UiColor,
    primary: PokemonType,
    secondary: Option<PokemonType>,
) -> UiNode {
    let mut types = vec![image(
        id + 30,
        type_icon_asset(primary).as_str(),
        UiStyle::fixed(72, 28),
    )];
    if let Some(secondary) = secondary {
        types.push(image(
            id + 31,
            type_icon_asset(secondary).as_str(),
            UiStyle::fixed(72, 28),
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
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(14),
                    border_radius: punctum_ui::UiBorderRadius::all(7),
                    ..UiStyle::default()
                })
                .with_content(UiContent::Fill(HP_TRACK_EDGE.into_ui()))
                .with_children([UiNode::auto()
                    .with_style(UiStyle {
                        width: Dimension::Ratio {
                            units: hp,
                            base: max_hp.max(1),
                        },
                        height: Dimension::Fill,
                        border_radius: punctum_ui::UiBorderRadius::all(7),
                        ..UiStyle::default()
                    })
                    .with_content(UiContent::Fill(hp_color(hp, max_hp).into_ui()))]),
            text(
                id + 7,
                format!("HP {hp}/{max_hp}"),
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

fn hp_color(hp: u32, max_hp: u32) -> Rgba8 {
    match hp.saturating_mul(100) / max_hp.max(1) {
        0..=20 => HP_LOW,
        21..=50 => HP_MID,
        _ => HP_GOOD,
    }
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
