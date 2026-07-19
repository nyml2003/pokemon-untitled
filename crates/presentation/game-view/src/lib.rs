//! Pure Gen3 product-view projection.

#![forbid(unsafe_code)]

use battle_session::{
    Ability, Action, BattleCue, BattleInteraction, BattleObservation, BattleSessionSnapshot,
    MoveCategory, ObservedBattleOutcome, Participant, Pokemon, PokemonType, TypeEffectiveness,
    UsedMove,
};
use game_assets::AssetKey;
use game_data::PokedexData;
use game_ui::{BattleMenuPage, BattleUiState, CommandConsoleView, PokedexAction, WorldAnimation};
use game_ui_kit::{
    GameUiTheme, PanelTone, SpriteAppearance, TextTone, column as ui_column, image as ui_image,
    panel as ui_panel, row as ui_row, screen as ui_screen,
    selectable_list_item as ui_selectable_list_item, sprite as ui_sprite, text as ui_text,
};
use punctum_gpu::{PixelOffset, Rgba8};
use punctum_grid::{GridPos, GridRect, GridSize, Surface};
use punctum_ui::{
    CrossAlign, Dimension, FlexDirection, Insets, MainAlign, UiBuildError, UiColor, UiContent,
    UiContentId, UiId, UiKey, UiNode, UiStyle, UiTextSize, UiTree,
};
use world_application::{
    CharacterAppearanceId, Direction as WorldDirection, WorldActorObservation, WorldActorRole,
    WorldObservation,
};

pub const CANVAS_WIDTH: u32 = 32;
pub const CANVAS_HEIGHT: u32 = 24;

const SKY: Rgba8 = Rgba8::new(146, 211, 218, 255);
const SKY_DEEP: Rgba8 = Rgba8::new(102, 177, 184, 255);
const DISTANT_GRASS: Rgba8 = Rgba8::new(75, 143, 105, 255);
const GROUND: Rgba8 = Rgba8::new(54, 105, 76, 255);
const GROUND_DARK: Rgba8 = Rgba8::new(37, 78, 62, 255);
const PLATFORM: Rgba8 = Rgba8::new(174, 201, 145, 255);
const PLATFORM_SHADOW: Rgba8 = Rgba8::new(45, 82, 64, 150);
const PANEL: Rgba8 = Rgba8::new(28, 34, 45, 248);
const PANEL_EDGE: Rgba8 = Rgba8::new(218, 225, 214, 255);
const SELECTED: Rgba8 = Rgba8::new(73, 211, 168, 255);
const SELECTED_DARK: Rgba8 = Rgba8::new(29, 70, 67, 255);
const BATTLE_CARD: Rgba8 = Rgba8::new(242, 246, 239, 255);
const BATTLE_CARD_SHADOW: Rgba8 = Rgba8::new(24, 37, 45, 190);
const BATTLE_INK: Rgba8 = Rgba8::new(26, 39, 45, 255);
const BATTLE_MUTED: Rgba8 = Rgba8::new(82, 96, 98, 255);
const OPPONENT_ACCENT: Rgba8 = Rgba8::new(241, 112, 116, 255);
const PLAYER_ACCENT: Rgba8 = Rgba8::new(57, 190, 151, 255);
const ACTION_PANEL: Rgba8 = Rgba8::new(19, 25, 34, 255);
const ACTION_PANEL_ALT: Rgba8 = Rgba8::new(30, 38, 49, 255);
const ACTION_BORDER: Rgba8 = Rgba8::new(83, 98, 112, 255);
const PARTY_BG: Rgba8 = Rgba8::new(13, 18, 27, 255);
const PARTY_PANEL: Rgba8 = Rgba8::new(25, 33, 44, 255);
const PARTY_PANEL_ALT: Rgba8 = Rgba8::new(34, 44, 57, 255);
const PARTY_EDGE: Rgba8 = Rgba8::new(73, 89, 105, 255);
const HP_GOOD: Rgba8 = Rgba8::new(74, 190, 102, 255);
const HP_MID: Rgba8 = Rgba8::new(226, 177, 66, 255);
const HP_LOW: Rgba8 = Rgba8::new(224, 91, 72, 255);
const HP_TRACK_EDGE: Rgba8 = Rgba8::new(38, 46, 55, 255);
const HP_GOOD_GLOW: Rgba8 = Rgba8::new(119, 231, 142, 255);
const HP_MID_GLOW: Rgba8 = Rgba8::new(255, 214, 101, 255);
const HP_LOW_GLOW: Rgba8 = Rgba8::new(255, 133, 111, 255);
const TEXT: Rgba8 = Rgba8::new(244, 246, 239, 255);
const MUTED_TEXT: Rgba8 = Rgba8::new(182, 194, 194, 255);
const CONSOLE_ERROR: Rgba8 = Rgba8::new(255, 142, 126, 255);
const MAP_GROUND: Rgba8 = Rgba8::new(138, 187, 116, 255);
const SPEECH_BUBBLE: Rgba8 = Rgba8::new(83, 89, 96, 236);

const POKEDEX_THEME: GameUiTheme = GameUiTheme {
    screen: UiColor::new(13, 21, 29, 255),
    header: UiColor::new(21, 47, 60, 255),
    panel: UiColor::new(31, 52, 64, 255),
    selected: UiColor::new(29, 70, 67, 255),
    selected_text: UiColor::new(73, 211, 168, 255),
    card: UiColor::new(237, 242, 233, 255),
    image_backdrop: UiColor::new(201, 220, 208, 255),
    text: UiColor::new(244, 246, 239, 255),
    muted_text: UiColor::new(182, 194, 194, 255),
    ink: UiColor::new(26, 39, 45, 255),
    muted_ink: UiColor::new(82, 96, 98, 255),
    small_spacing: 8,
    medium_spacing: 16,
    large_spacing: 28,
    small_radius: punctum_ui::UiBorderRadius::all(8),
    medium_radius: punctum_ui::UiBorderRadius::all(12),
    large_radius: punctum_ui::UiBorderRadius::all(16),
    body_text_size: 18,
    title_text_size: 28,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BattleAnimation {
    #[default]
    Idle,
    Acting(Participant),
    Hit(Participant),
    Fainted(Participant),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextRole {
    Location,
    OpponentName,
    OpponentDetail,
    OpponentHp,
    PlayerName,
    PlayerDetail,
    PlayerHp,
    Action(usize),
    ActionDetail(usize),
    PageTitle,
    TeamMember(usize),
    TeamMemberHp(usize),
    TeamMemberType(usize),
    SelectedMemberName,
    SelectedMemberDetail,
    SelectedMemberHp,
    Message,
    ConsoleQuery,
    ConsoleItem(usize),
    ConsoleDiagnostic,
    Editor,
    PokedexTitle,
    PokedexEntry,
    PokedexDetail,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextLabel {
    pub role: TextRole,
    pub col: u32,
    pub row: u32,
    pub width: u32,
    pub height: u32,
    pub content: String,
    pub color: Rgba8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ViewCell {
    Empty,
    Fill(Rgba8),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewImage {
    pub bounds: GridRect,
    pub asset: AssetKey,
    pub tint: Rgba8,
    pub z_index: u16,
    pub pixel_offset: PixelOffset,
}

impl ViewImage {
    pub fn new(bounds: GridRect, asset: AssetKey, tint: Rgba8, z_index: u16) -> Self {
        Self {
            bounds,
            asset,
            tint,
            z_index,
            pixel_offset: PixelOffset::new(0, 0),
        }
    }

    pub const fn with_pixel_offset(mut self, pixel_offset: PixelOffset) -> Self {
        self.pixel_offset = pixel_offset;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewLayer {
    pub kind: LayerKind,
    pub surface: Option<Surface<ViewCell>>,
    pub images: Vec<ViewImage>,
    pub labels: Vec<TextLabel>,
}

impl ViewLayer {
    pub fn new(kind: LayerKind) -> Self {
        Self {
            kind,
            surface: None,
            images: Vec::new(),
            labels: Vec::new(),
        }
    }

    pub fn with_surface(mut self, surface: Surface<ViewCell>) -> Self {
        self.surface = Some(surface);
        self
    }

    pub fn with_images(mut self, images: Vec<ViewImage>) -> Self {
        self.images = images;
        self
    }

    pub fn with_labels(mut self, labels: Vec<TextLabel>) -> Self {
        self.labels = labels;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LayerKind {
    Map,
    Character,
    Hud,
    Console,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameView {
    layers: Vec<ViewLayer>,
}

impl GameView {
    pub fn new(layers: impl IntoIterator<Item = ViewLayer>) -> Self {
        let layers = layers.into_iter().collect::<Vec<_>>();
        assert!(layers.windows(2).all(|pair| pair[0].kind <= pair[1].kind));
        Self { layers }
    }

    pub fn layers(&self) -> &[ViewLayer] {
        &self.layers
    }

    pub fn images(&self) -> impl Iterator<Item = &ViewImage> {
        self.layers.iter().flat_map(|layer| &layer.images)
    }

    pub fn labels(&self) -> impl Iterator<Item = &TextLabel> {
        self.layers.iter().flat_map(|layer| &layer.labels)
    }
}

/// Builds the Pokedex as a responsive pixel UI tree. It deliberately does not
/// project into `GameView`: the Pokedex is a focused page, not a map surface.
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

/// Builds the battle scene as a responsive pixel UI page.
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
        return UiTree::new(with_generated_ui_ids(root));
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
        BattleMenuPage::Pokemon => {
            unreachable!("the Pokemon page returns before building the battle scene")
        }
        BattleMenuPage::Hidden => UiNode::new(UiId(9_100)),
    };

    UiTree::new(with_generated_ui_ids(panel(
        8_000,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            direction: FlexDirection::Column,
            ..UiStyle::default()
        },
        SKY.into_ui(),
        [
            UiNode::new(UiId(8_010))
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
                    UiNode::new(UiId(8_011))
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
                    UiNode::new(UiId(8_012))
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
                    UiNode::new(UiId(8_201))
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
                    UiNode::new(UiId(8_210))
                        .with_style(UiStyle {
                            width: Dimension::Px(430),
                            height: Dimension::Fill,
                            ..UiStyle::default()
                        })
                        .with_children([menu]),
                ],
            ),
        ],
    )))
}

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
    UiTree::new(with_generated_ui_ids(
        UiNode::new(UiId(8_400))
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
                    UiNode::new(UiId(8_403))
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
    ))
}

/// IDs are scoped to a single tree. Rebuilding the same tree produces the same
/// IDs, while no page author has to coordinate hand-written numeric ranges.
#[derive(Default)]
struct UiNodeIds {
    next: u32,
}

impl UiNodeIds {
    fn next(&mut self) -> UiId {
        let id = UiId(self.next);
        self.next = self
            .next
            .checked_add(1)
            .expect("a UI tree cannot contain more than u32::MAX nodes");
        id
    }
}

fn with_generated_ui_ids(root: UiNode) -> UiNode {
    fn visit(mut node: UiNode, ids: &mut UiNodeIds) -> UiNode {
        node.id = ids.next();
        node.children = node
            .children
            .into_iter()
            .map(|child| visit(child, ids))
            .collect();
        node
    }

    visit(root, &mut UiNodeIds::default())
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
        UiNode::new(UiId(9_100 + row as u32))
            .with_style(UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                direction: FlexDirection::Row,
                ..UiStyle::default()
            })
            .with_children((0..2).map(|column| {
                let index = row * 2 + column;
                battle_main_action_button(9_110 + index as u32, buttons[index], index == selected)
            }))
    });
    UiNode::new(UiId(9_010))
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            direction: FlexDirection::Column,
            border_radius: punctum_ui::UiBorderRadius::all(12),
            clip: true,
            ..UiStyle::default()
        })
        .with_content(UiContent::Fill(ACTION_PANEL_ALT.into_ui()))
        .with_children(rows)
}

fn battle_main_action_button(id: u32, content: &str, selected: bool) -> UiNode {
    UiNode::new(UiId(id))
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            border_radius: punctum_ui::UiBorderRadius::all(10),
            interactive: true,
            ..UiStyle::default()
        })
        .with_content(if selected {
            UiContent::Fill(SELECTED.into_ui())
        } else {
            UiContent::Empty
        })
        .with_children([UiNode::new(UiId(id + 100))
            .with_style(UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                ..UiStyle::default()
            })
            .with_content(UiContent::TextScaled {
                content: content.to_owned(),
                color: if selected {
                    BATTLE_INK.into_ui()
                } else {
                    TEXT.into_ui()
                },
                font_size: UiTextSize::Px(18),
            })])
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

    UiNode::new(UiId(9_100))
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            direction: FlexDirection::Row,
            gap: 10,
            ..UiStyle::default()
        })
        .with_children([
            UiNode::new(UiId(9_110))
                .with_style(UiStyle {
                    width: Dimension::Ratio { units: 3, base: 5 },
                    height: Dimension::Fill,
                    direction: FlexDirection::Column,
                    gap: 6,
                    clip: true,
                    ..UiStyle::default()
                })
                .with_children(moves.iter().enumerate().map(|(index, (name, ..))| {
                    battle_main_action_button(9_120 + index as u32, name, index == selected)
                })),
            UiNode::new(UiId(9_200))
                .with_style(UiStyle {
                    width: Dimension::Ratio { units: 2, base: 5 },
                    height: Dimension::Fill,
                    ..UiStyle::default()
                })
                .with_children(detail),
        ])
}

fn move_detail_panel(
    id: u32,
    move_type: PokemonType,
    category: MoveCategory,
    detail: &str,
) -> UiNode {
    panel(
        id,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            direction: FlexDirection::Column,
            gap: 8,
            padding: Insets::all(10),
            border_radius: punctum_ui::UiBorderRadius::all(10),
            ..UiStyle::default()
        },
        ACTION_PANEL_ALT.into_ui(),
        [
            text(
                id + 1,
                "招式详情",
                MUTED_TEXT.into_ui(),
                15,
                Dimension::Fill,
            ),
            UiNode::new(UiId(id + 2))
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(28),
                    direction: FlexDirection::Row,
                    gap: 8,
                    ..UiStyle::default()
                })
                .with_children([
                    image(
                        id + 3,
                        type_icon_asset(move_type).as_str(),
                        UiStyle::fixed(72, 28),
                    ),
                    image(
                        id + 4,
                        move_category_icon_asset(category).as_str(),
                        UiStyle::fixed(72, 28),
                    ),
                ]),
            text(id + 5, detail, TEXT.into_ui(), 17, Dimension::Fill),
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
            UiNode::new(UiId(9_510))
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
                    UiNode::new(UiId(9_600))
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
                content: UiContentId::new(pokemon_icon_asset(slot, sprite_frame).as_str())
                    .expect("team icon asset keys are non-empty"),
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
            UiNode::new(UiId(id + 4))
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
    UiNode::new(UiId(id))
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
            interactive: true,
            ..UiStyle::default()
        })
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
                content: UiContentId::new(pokemon_icon_asset(slot, sprite_frame).as_str())
                    .expect("team icon asset keys are non-empty"),
                tint: if pokemon.is_fainted() {
                    UiColor::new(112, 112, 112, 255)
                } else {
                    UiColor::new(255, 255, 255, 255)
                },
            }),
            UiNode::new(UiId(id + 2))
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

fn hp_bar(id: u32, hp: u32, max_hp: u32) -> UiNode {
    UiNode::new(UiId(id))
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(12),
            border_radius: punctum_ui::UiBorderRadius::all(6),
            ..UiStyle::default()
        })
        .with_content(UiContent::Fill(HP_TRACK_EDGE.into_ui()))
        .with_children([UiNode::new(UiId(id + 1))
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
            UiNode::new(UiId(id + 1))
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
            UiNode::new(UiId(id + 4))
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(28),
                    direction: FlexDirection::Row,
                    gap: 6,
                    ..UiStyle::default()
                })
                .with_children(types),
            UiNode::new(UiId(id + 5))
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(14),
                    border_radius: punctum_ui::UiBorderRadius::all(7),
                    ..UiStyle::default()
                })
                .with_content(UiContent::Fill(HP_TRACK_EDGE.into_ui()))
                .with_children([UiNode::new(UiId(id + 6))
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
            UiNode::new(UiId(id + 8))
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
    UiNode::new(UiId(id))
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(36),
            padding: Insets::symmetric(10, 6),
            border_radius: punctum_ui::UiBorderRadius::all(6),
            interactive: true,
            ..UiStyle::default()
        })
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
    id: u32,
    style: UiStyle,
    color: UiColor,
    children: impl IntoIterator<Item = UiNode>,
) -> UiNode {
    UiNode::new(UiId(id))
        .with_style(style)
        .with_content(UiContent::Fill(color))
        .with_children(children)
}
fn text(
    id: u32,
    content: impl Into<String>,
    color: UiColor,
    font_size: u32,
    width: Dimension,
) -> UiNode {
    UiNode::new(UiId(id))
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
fn image(id: u32, content: impl Into<String>, style: UiStyle) -> UiNode {
    UiNode::new(UiId(id))
        .with_style(style)
        .with_content(UiContent::Image(
            UiContentId::new(content).expect("static UI asset keys are non-empty"),
        ))
}

pub fn project_console(console: &CommandConsoleView) -> ViewLayer {
    const PANEL_COL: u32 = 1;
    const PANEL_ROW: u32 = 4;
    const PANEL_WIDTH: u32 = 30;
    const PANEL_HEIGHT: u32 = 16;
    const FIRST_ITEM_ROW: u32 = 8;
    const MAX_ITEMS: usize = 8;

    let panel = GridRect::new(
        GridPos::new(PANEL_COL as i32, PANEL_ROW as i32),
        GridSize::new(PANEL_WIDTH, PANEL_HEIGHT),
    );
    let mut surface = Surface::filled(GridSize::new(CANVAS_WIDTH, CANVAS_HEIGHT), ViewCell::Empty)
        .expect("the fixed console surface is valid");
    surface
        .fill_rect(panel, sprite(PANEL_EDGE))
        .expect("the fixed console panel fits the game canvas");
    surface
        .fill_rect(
            GridRect::new(
                GridPos::new((PANEL_COL + 1) as i32, (PANEL_ROW + 1) as i32),
                GridSize::new(PANEL_WIDTH - 2, PANEL_HEIGHT - 2),
            ),
            sprite(PANEL),
        )
        .expect("the fixed console body fits the game canvas");

    let mut labels = vec![label(
        TextRole::ConsoleQuery,
        3,
        6,
        26,
        1,
        &format!("> {}{}", console.query, console.preedit),
        TEXT,
    )];

    let first_visible = visible_console_start(console.items.len(), console.selected_index);
    for (visible_index, (item_index, item)) in console
        .items
        .iter()
        .enumerate()
        .skip(first_visible)
        .take(MAX_ITEMS)
        .enumerate()
    {
        let row = FIRST_ITEM_ROW + visible_index as u32;
        if console.selected_index == Some(item_index) {
            surface
                .fill_rect(
                    GridRect::new(GridPos::new(2, row as i32), GridSize::new(28, 1)),
                    sprite(SELECTED),
                )
                .expect("the fixed console selection fits the game canvas");
        }
        labels.push(label(
            TextRole::ConsoleItem(item_index),
            3,
            row,
            26,
            1,
            item,
            TEXT,
        ));
    }

    if console.items.is_empty() {
        labels.push(label(
            TextRole::ConsoleItem(0),
            3,
            FIRST_ITEM_ROW,
            26,
            1,
            "没有匹配指令",
            MUTED_TEXT,
        ));
    }
    if let Some(diagnostic) = &console.diagnostic {
        labels.push(label(
            TextRole::ConsoleDiagnostic,
            3,
            18,
            26,
            1,
            diagnostic,
            CONSOLE_ERROR,
        ));
    }
    ViewLayer::new(LayerKind::Console)
        .with_surface(surface)
        .with_labels(labels)
}

fn visible_console_start(item_count: usize, selected_index: Option<usize>) -> usize {
    selected_index
        .map_or(0, |selected| selected.saturating_add(1).saturating_sub(8))
        .min(item_count.saturating_sub(8))
}

fn prompt_data(interaction: &BattleInteraction) -> Option<(&BattleObservation, &[Action])> {
    match interaction {
        BattleInteraction::ChooseAction(prompt) => {
            Some((prompt.observation(), prompt.legal_actions()))
        }
        BattleInteraction::ChooseReplacement(prompt) => {
            Some((prompt.observation(), prompt.legal_actions()))
        }
        BattleInteraction::PlaybackLocked | BattleInteraction::Finished(_) => None,
    }
}

pub fn project_battle(
    snapshot: &BattleSessionSnapshot,
    ui: BattleUiState,
    sprites: BattleSpriteResources,
    sprite_frame: usize,
) -> GameView {
    let prompt = prompt_data(snapshot.interaction());
    let (page, selected_index, notice) = ui.view();
    let message = notice
        .map(str::to_owned)
        .unwrap_or_else(|| battle_message(snapshot));
    if page == BattleMenuPage::Pokemon
        && let Some((observation, _)) = prompt
    {
        return project_pokemon_page(observation, ui, &message, sprite_frame);
    }

    let scene = snapshot.scene();
    let own = scene.own();
    let opponent = scene.opponent();
    let mut canvas = Canvas::new(SKY);
    draw_battlefield(&mut canvas);
    let battlefield_images = battlefield_images();
    let mut images = Vec::new();
    draw_status_panel(
        &mut images,
        1,
        1,
        opponent.current_hp(),
        opponent.max_hp(),
        OPPONENT_ACCENT,
    );
    draw_status_panel(
        &mut images,
        17,
        11,
        own.current_hp(),
        own.max_hp(),
        PLAYER_ACCENT,
    );
    let actions = prompt.map_or(&[][..], |(_, actions)| actions);
    let observation = prompt.map(|(observation, _)| observation);
    let action_count = match page {
        BattleMenuPage::Main => 4,
        BattleMenuPage::Fight => {
            if actions.contains(&Action::Struggle) {
                1
            } else {
                observation.map_or(0, |observation| active_pokemon(observation).moves().len())
            }
        }
        BattleMenuPage::Pokemon | BattleMenuPage::Hidden => 0,
    };
    draw_action_panel(&mut images, page, action_count, selected_index);
    let character_images = battle_images(battle_animation(snapshot.cue()), sprites, sprite_frame);
    images.extend(type_icon_images(
        10,
        3,
        opponent.primary_type(),
        opponent.secondary_type(),
    ));
    images.extend(type_icon_images(
        26,
        13,
        own.primary_type(),
        own.secondary_type(),
    ));

    let mut labels = vec![
        label(
            TextRole::OpponentName,
            4,
            2,
            7,
            1,
            opponent.name(),
            BATTLE_INK,
        ),
        label(
            TextRole::OpponentDetail,
            4,
            3,
            6,
            1,
            &format!("Lv.{}", opponent.level()),
            BATTLE_MUTED,
        ),
        label(
            TextRole::OpponentHp,
            4,
            4,
            9,
            1,
            &format!("HP {}/{}", opponent.current_hp(), opponent.max_hp()),
            BATTLE_MUTED,
        ),
        label(TextRole::PlayerName, 20, 12, 7, 1, own.name(), BATTLE_INK),
        label(
            TextRole::PlayerDetail,
            20,
            13,
            6,
            1,
            &format!("Lv.{}", own.level()),
            BATTLE_MUTED,
        ),
        label(
            TextRole::PlayerHp,
            20,
            14,
            9,
            1,
            &format!("HP {}/{}", own.current_hp(), own.max_hp()),
            BATTLE_MUTED,
        ),
    ];
    match page {
        BattleMenuPage::Main => {
            for (index, content) in ["战斗", "宝可梦", "包包", "逃走"].into_iter().enumerate()
            {
                let col = 20 + (index as u32 % 2) * 6;
                let row = 18 + (index as u32 / 2) * 2;
                labels.push(label(
                    TextRole::Action(index),
                    col,
                    row,
                    5,
                    1,
                    content,
                    if index == selected_index {
                        BATTLE_INK
                    } else {
                        TEXT
                    },
                ));
            }
            labels.push(label(TextRole::Message, 3, 20, 13, 2, &message, MUTED_TEXT));
        }
        BattleMenuPage::Fight if actions.contains(&Action::Struggle) => {
            labels.push(label(TextRole::Action(0), 3, 18, 8, 1, "挣扎", BATTLE_INK));
            images.push(type_icon_image(23, 18, PokemonType::Normal));
            images.push(move_category_icon_image(26, 18, MoveCategory::Physical));
            labels.push(label(
                TextRole::ActionDetail(0),
                23,
                20,
                7,
                1,
                "威50 PP--",
                MUTED_TEXT,
            ));
            labels.push(label(TextRole::Message, 3, 22, 17, 1, &message, MUTED_TEXT));
        }
        BattleMenuPage::Fight => {
            let moves = observation
                .map(active_pokemon)
                .map_or(&[][..], |pokemon| pokemon.moves());
            for (index, battle_move) in moves.iter().enumerate().take(4) {
                let col = 3 + (index as u32 % 2) * 10;
                let row = 18 + (index as u32 / 2) * 2;
                labels.push(label(
                    TextRole::Action(index),
                    col,
                    row,
                    8,
                    1,
                    battle_move.name(),
                    if index == selected_index {
                        BATTLE_INK
                    } else {
                        TEXT
                    },
                ));
            }
            if let Some(battle_move) = moves.get(selected_index) {
                images.push(type_icon_image(23, 18, battle_move.move_type()));
                images.push(move_category_icon_image(26, 18, battle_move.category()));
                labels.push(label(
                    TextRole::ActionDetail(selected_index),
                    23,
                    20,
                    7,
                    1,
                    &format!(
                        "威{} PP{}/{}",
                        battle_move.power(),
                        battle_move.current_pp(),
                        battle_move.max_pp()
                    ),
                    MUTED_TEXT,
                ));
            }
            labels.push(label(TextRole::Message, 3, 22, 17, 1, &message, MUTED_TEXT));
        }
        BattleMenuPage::Hidden => {
            labels.push(label(TextRole::Message, 3, 20, 26, 2, &message, TEXT))
        }
        BattleMenuPage::Pokemon => {}
    }

    GameView::new([
        ViewLayer::new(LayerKind::Map)
            .with_surface(canvas.finish())
            .with_images(battlefield_images),
        ViewLayer::new(LayerKind::Character).with_images(character_images),
        ViewLayer::new(LayerKind::Hud)
            .with_images(images)
            .with_labels(labels),
    ])
}

fn battle_animation(cue: Option<&BattleCue>) -> BattleAnimation {
    match cue {
        Some(BattleCue::MoveUsed { participant, .. }) => BattleAnimation::Acting(*participant),
        Some(BattleCue::DamageApplied { participant, .. })
        | Some(BattleCue::Critical { participant }) => BattleAnimation::Hit(*participant),
        Some(BattleCue::Fainted { participant }) => BattleAnimation::Fainted(*participant),
        _ => BattleAnimation::Idle,
    }
}

fn battle_message(snapshot: &BattleSessionSnapshot) -> String {
    let scene = snapshot.scene();
    match snapshot.cue() {
        Some(BattleCue::TurnStarted { turn }) => format!("第 {turn} 回合"),
        Some(BattleCue::Switched { participant }) => {
            format!("{} 上场了。", combatant_name(scene, *participant))
        }
        Some(BattleCue::MoveUsed {
            participant,
            used_move,
        }) => format!(
            "{} 使用了 {}！",
            combatant_name(scene, *participant),
            used_move_name(used_move)
        ),
        Some(BattleCue::DamageApplied {
            participant,
            amount,
        }) => format!(
            "{} 受到 {} 点伤害。",
            combatant_name(scene, *participant),
            amount
        ),
        Some(BattleCue::StatusApplied {
            participant,
            status,
        }) => format!(
            "{} {}了。",
            combatant_name(scene, *participant),
            major_status_message(*status)
        ),
        Some(BattleCue::StatusFailed { .. }) => "但是失败了。".into(),
        Some(BattleCue::StatusPreventsAction {
            participant,
            status,
        }) => format!(
            "{} 因{}无法行动。",
            combatant_name(scene, *participant),
            major_status_reason(*status)
        ),
        Some(BattleCue::StatusCured {
            participant,
            status,
        }) => format!(
            "{} 从{}中恢复了。",
            combatant_name(scene, *participant),
            major_status_kind_message(*status)
        ),
        Some(BattleCue::StatStageChanged {
            participant,
            stat,
            change,
            stage: _,
        }) => format!(
            "{} 的{}{}了。",
            combatant_name(scene, *participant),
            battle_stat_message(*stat),
            if *change > 0 { "提高" } else { "降低" }
        ),
        Some(BattleCue::Healed {
            participant,
            amount,
        }) => format!(
            "{} 回复了 {} 点 HP。",
            combatant_name(scene, *participant),
            amount
        ),
        Some(BattleCue::EffectFailed { .. }) => "但是失败了。".into(),
        Some(BattleCue::ProtectionActivated { participant }) => {
            format!("{} 进入了守住状态。", combatant_name(scene, *participant))
        }
        Some(BattleCue::ProtectionFailed { participant }) => {
            format!("{} 的守住失败了。", combatant_name(scene, *participant))
        }
        Some(BattleCue::MoveBlocked { target, .. }) => {
            format!("{} 守住了攻击！", combatant_name(scene, *target))
        }
        Some(BattleCue::SubstituteCreated {
            participant,
            substitute_hp,
        }) => format!(
            "{} 制造了替身（{} HP）。",
            combatant_name(scene, *participant),
            substitute_hp
        ),
        Some(BattleCue::SubstituteBlocked { target, .. }) => {
            format!("{} 的替身挡住了招式。", combatant_name(scene, *target))
        }
        Some(BattleCue::SubstituteDamaged {
            participant,
            amount,
            remaining_hp,
        }) => format!(
            "{} 的替身受到了 {} 点伤害（剩余 {}）。",
            combatant_name(scene, *participant),
            amount,
            remaining_hp
        ),
        Some(BattleCue::SubstituteBroke { participant }) => {
            format!("{} 的替身消失了。", combatant_name(scene, *participant))
        }
        Some(BattleCue::WeatherStarted {
            weather,
            turns_remaining,
        }) => match turns_remaining {
            Some(turns) => format!("{}开始了，剩余 {turns} 回合。", weather_message(*weather)),
            None => format!("{}开始了。", weather_message(*weather)),
        },
        Some(BattleCue::WeatherUpdated {
            weather,
            turns_remaining,
        }) => format!(
            "{}，剩余 {turns_remaining} 回合。",
            weather_message(*weather)
        ),
        Some(BattleCue::WeatherEnded { weather }) => {
            format!("{}停止了。", weather_message(*weather))
        }
        Some(BattleCue::AbilityActivated {
            participant,
            ability,
        }) => format!(
            "{} 的{}发动了！",
            combatant_name(scene, *participant),
            ability_message(*ability)
        ),
        Some(BattleCue::Flinched { participant }) => {
            format!("{} 畏缩了。", combatant_name(scene, *participant))
        }
        Some(BattleCue::Missed { .. }) => "攻击没有命中。".into(),
        Some(BattleCue::Critical { .. }) => "会心一击！".into(),
        Some(BattleCue::Effectiveness { effectiveness, .. }) => {
            effectiveness_message(*effectiveness).into()
        }
        Some(BattleCue::Fainted { participant }) => {
            format!("{} 倒下了。", combatant_name(scene, *participant))
        }
        Some(BattleCue::ReplacementRequired { .. }) => "请选择下一只宝可梦".into(),
        Some(BattleCue::BattleFinished { outcome }) => outcome_message(*outcome).into(),
        None => match snapshot.interaction() {
            BattleInteraction::ChooseAction(_) => "请选择行动".into(),
            BattleInteraction::ChooseReplacement(_) => "请选择下一只宝可梦".into(),
            BattleInteraction::PlaybackLocked => String::new(),
            BattleInteraction::Finished(prompt) => outcome_message(prompt.outcome()).into(),
        },
    }
}

fn major_status_message(status: battle_session::MajorStatus) -> &'static str {
    match status {
        battle_session::MajorStatus::Burn => "烧伤",
        battle_session::MajorStatus::BadlyPoisoned { .. } => "剧毒",
        battle_session::MajorStatus::Freeze => "冰冻",
        battle_session::MajorStatus::Paralysis => "麻痹",
        battle_session::MajorStatus::Poison => "中毒",
        battle_session::MajorStatus::Sleep { .. } => "睡着",
    }
}

fn major_status_reason(status: battle_session::MajorStatus) -> &'static str {
    match status {
        battle_session::MajorStatus::Freeze => "冰冻",
        battle_session::MajorStatus::Paralysis => "麻痹",
        battle_session::MajorStatus::Sleep { .. } => "睡眠",
        battle_session::MajorStatus::BadlyPoisoned { .. }
        | battle_session::MajorStatus::Burn
        | battle_session::MajorStatus::Poison => "状态",
    }
}

fn major_status_kind_message(status: battle_session::MajorStatusKind) -> &'static str {
    match status {
        battle_session::MajorStatusKind::Burn => "烧伤",
        battle_session::MajorStatusKind::BadlyPoisoned => "剧毒",
        battle_session::MajorStatusKind::Freeze => "冰冻",
        battle_session::MajorStatusKind::Paralysis => "麻痹",
        battle_session::MajorStatusKind::Poison => "中毒",
        battle_session::MajorStatusKind::Sleep => "睡眠",
    }
}

fn battle_stat_message(stat: battle_session::BattleStat) -> &'static str {
    match stat {
        battle_session::BattleStat::Attack => "攻击",
        battle_session::BattleStat::Defense => "防御",
        battle_session::BattleStat::SpecialAttack => "特攻",
        battle_session::BattleStat::SpecialDefense => "特防",
        battle_session::BattleStat::Speed => "速度",
        battle_session::BattleStat::Accuracy => "命中率",
        battle_session::BattleStat::Evasion => "闪避率",
    }
}

fn weather_message(weather: battle_session::Weather) -> &'static str {
    match weather {
        battle_session::Weather::Hail => "冰雹",
        battle_session::Weather::Rain => "下雨",
        battle_session::Weather::Sandstorm => "沙暴",
        battle_session::Weather::Sun => "阳光强烈",
    }
}

fn ability_message(ability: Ability) -> &'static str {
    match ability {
        Ability::AirLock => "气闸",
        Ability::ArenaTrap => "沙穴",
        Ability::BattleArmor => "战斗盔甲",
        Ability::Blaze => "猛火",
        Ability::Chlorophyll => "叶绿素",
        Ability::ClearBody => "清晰之躯",
        Ability::CloudNine => "无关天气",
        Ability::CompoundEyes => "复眼",
        Ability::Drizzle => "降雨",
        Ability::Drought => "日照",
        Ability::EarlyBird => "早起",
        Ability::FlashFire => "闪火",
        Ability::Guts => "根性",
        Ability::HugePower => "大力士",
        Ability::HyperCutter => "怪力钳",
        Ability::Hustle => "活力",
        Ability::Immunity => "免疫",
        Ability::Intimidate => "威吓",
        Ability::InnerFocus => "精神力",
        Ability::KeenEye => "锐利目光",
        Ability::Insomnia => "不眠",
        Ability::Levitate => "飘浮",
        Ability::Limber => "柔软",
        Ability::LiquidOoze => "污泥浆",
        Ability::MagmaArmor => "熔岩铠甲",
        Ability::MarvelScale => "神奇鳞片",
        Ability::NaturalCure => "自然回复",
        Ability::Overgrow => "茂盛",
        Ability::Pressure => "压迫感",
        Ability::PurePower => "瑜伽之力",
        Ability::RainDish => "雨盘",
        Ability::RockHead => "坚硬脑袋",
        Ability::SandStream => "扬沙",
        Ability::SandVeil => "沙隐",
        Ability::SereneGrace => "天恩",
        Ability::ShellArmor => "硬壳盔甲",
        Ability::ShedSkin => "蜕皮",
        Ability::ShieldDust => "鳞粉",
        Ability::ShadowTag => "踩影",
        Ability::SpeedBoost => "加速",
        Ability::Synchronize => "同步",
        Ability::SwiftSwim => "悠游自如",
        Ability::Swarm => "虫之预感",
        Ability::ThickFat => "厚脂肪",
        Ability::Torrent => "激流",
        Ability::VitalSpirit => "干劲",
        Ability::VoltAbsorb => "蓄电",
        Ability::WaterAbsorb => "蓄水",
        Ability::WaterVeil => "水幕",
        Ability::WhiteSmoke => "白色烟雾",
    }
}

fn combatant_name(scene: &battle_session::BattleScene, participant: Participant) -> &str {
    match participant {
        Participant::Own => scene.own().name(),
        Participant::Opponent => scene.opponent().name(),
    }
}

fn used_move_name(used_move: &UsedMove) -> &str {
    match used_move {
        UsedMove::Move { name, .. } => name,
        UsedMove::Struggle => "挣扎",
    }
}

fn outcome_message(outcome: ObservedBattleOutcome) -> &'static str {
    match outcome {
        ObservedBattleOutcome::Winner(Participant::Own) => "你赢了！",
        ObservedBattleOutcome::Winner(Participant::Opponent) => "对手赢了。",
        ObservedBattleOutcome::Escaped(Participant::Own) => "成功逃走了！",
        ObservedBattleOutcome::Escaped(Participant::Opponent) => "对手逃走了。",
        ObservedBattleOutcome::Draw => "战斗平局。",
    }
}

fn effectiveness_message(effectiveness: TypeEffectiveness) -> &'static str {
    match effectiveness {
        TypeEffectiveness::Immune => "没有效果。",
        TypeEffectiveness::Quarter | TypeEffectiveness::Half => "效果不太好……",
        TypeEffectiveness::Normal => "命中了。",
        TypeEffectiveness::Double | TypeEffectiveness::Quadruple => "效果绝佳！",
    }
}

fn project_pokemon_page(
    observation: &BattleObservation,
    ui: BattleUiState,
    message: &str,
    sprite_frame: usize,
) -> GameView {
    let selected_index = ui.view().1;
    let selected_pokemon = &observation.own().members()[selected_index];
    let canvas = Canvas::new(PARTY_BG);

    let mut labels = vec![
        label(TextRole::PageTitle, 3, 1, 26, 1, "选择宝可梦", TEXT),
        label(
            TextRole::SelectedMemberName,
            2,
            13,
            9,
            1,
            selected_pokemon.name(),
            TEXT,
        ),
        label(
            TextRole::SelectedMemberDetail,
            2,
            14,
            9,
            1,
            &format!(
                "Lv.{}{}",
                selected_pokemon.level(),
                if selected_index == observation.own().active_slot().index() {
                    "  出战"
                } else {
                    ""
                }
            ),
            MUTED_TEXT,
        ),
        label(
            TextRole::SelectedMemberHp,
            2,
            18,
            9,
            1,
            &if selected_pokemon.is_fainted() {
                "无法战斗".into()
            } else {
                format!(
                    "HP {}/{}",
                    selected_pokemon.current_hp(),
                    selected_pokemon.max_hp()
                )
            },
            if selected_pokemon.is_fainted() {
                HP_LOW
            } else {
                TEXT
            },
        ),
    ];
    let mut images = vec![
        rounded_image(1, 0, 30, 3, PARTY_PANEL_ALT, 0),
        rounded_image(1, 4, 11, 17, PARTY_PANEL, 0),
        rounded_image(1, 21, 30, 3, PARTY_PANEL, 0),
    ];
    images.push(pokemon_icon_image(
        GridRect::new(GridPos::new(3, 5), GridSize::new(7, 7)),
        selected_index,
        selected_pokemon.is_fainted(),
        sprite_frame,
    ));
    images.extend(type_icon_images(
        3,
        16,
        selected_pokemon.primary_type(),
        selected_pokemon.secondary_type(),
    ));
    draw_hp_bar(
        &mut images,
        2,
        20,
        9,
        selected_pokemon.current_hp(),
        selected_pokemon.max_hp(),
    );

    for (index, pokemon) in observation.own().members().iter().enumerate() {
        let row = 4 + index as u32 * 3;
        let selected = index == selected_index;
        draw_team_card(&mut images, 13, row, selected, pokemon);
        images.push(pokemon_icon_image(
            GridRect::new(GridPos::new(14, row as i32), GridSize::new(3, 3)),
            index,
            pokemon.is_fainted(),
            sprite_frame,
        ));
        let active = index == observation.own().active_slot().index();
        labels.push(label(
            TextRole::TeamMember(index),
            18,
            row,
            8,
            1,
            pokemon.name(),
            if pokemon.is_fainted() {
                MUTED_TEXT
            } else if selected {
                TEXT
            } else {
                MUTED_TEXT
            },
        ));
        labels.push(label(
            TextRole::TeamMemberType(index),
            26,
            row,
            4,
            1,
            &if active {
                "出战".into()
            } else {
                format!("Lv.{}", pokemon.level())
            },
            if active { PLAYER_ACCENT } else { MUTED_TEXT },
        ));
        labels.push(label(
            TextRole::TeamMemberHp(index),
            18,
            row + 1,
            11,
            1,
            &if pokemon.is_fainted() {
                "无法战斗".into()
            } else {
                format!("{}/{}", pokemon.current_hp(), pokemon.max_hp())
            },
            if pokemon.is_fainted() {
                HP_LOW
            } else {
                MUTED_TEXT
            },
        ));
    }
    labels.push(label(TextRole::Message, 3, 22, 27, 1, message, MUTED_TEXT));
    GameView::new([
        ViewLayer::new(LayerKind::Map),
        ViewLayer::new(LayerKind::Character),
        ViewLayer::new(LayerKind::Hud)
            .with_surface(canvas.finish())
            .with_images(images)
            .with_labels(labels),
    ])
}

pub fn project_world(observation: &WorldObservation) -> GameView {
    project_world_animated(observation, WorldAnimation::Stand, 0)
}

pub fn project_world_animated(
    observation: &WorldObservation,
    animation: WorldAnimation,
    sprite_frame: usize,
) -> GameView {
    project_world_presented(observation, animation, sprite_frame, PixelOffset::new(0, 0))
}

pub fn project_world_presented(
    observation: &WorldObservation,
    animation: WorldAnimation,
    sprite_frame: usize,
    pixel_offset: PixelOffset,
) -> GameView {
    let mut actors = world_actor_images(
        observation,
        animation,
        sprite_frame,
        pixel_offset,
        PixelOffset::new(0, 0),
    );
    let speech = world_speech_overlay(observation, GridPos::new(0, 0), PixelOffset::new(0, 0));
    actors.extend(speech.images);
    GameView::new([
        ViewLayer::new(LayerKind::Map).with_surface(Canvas::new(MAP_GROUND).finish()),
        ViewLayer::new(LayerKind::Character)
            .with_images(actors)
            .with_labels(speech.labels),
        ViewLayer::new(LayerKind::Hud),
    ])
}

pub fn compose_world(
    map: ViewLayer,
    camera: GridPos,
    observation: &WorldObservation,
    animation: WorldAnimation,
    sprite_frame: usize,
    npc_pixel_offset: PixelOffset,
    console: Option<&CommandConsoleView>,
) -> GameView {
    assert_eq!(map.kind, LayerKind::Map);
    let viewport_size = map
        .surface
        .as_ref()
        .expect("a composed map layer has a grid surface")
        .size();
    let mut actors = world_actor_images(
        observation,
        animation,
        sprite_frame,
        PixelOffset::new(0, 0),
        npc_pixel_offset,
    );
    for actor in &mut actors {
        actor.bounds.origin.col -= camera.col * 2;
        actor.bounds.origin.row -= camera.row * 2;
    }
    actors.retain(|actor| actor.bounds.clip_to(viewport_size) == Some(actor.bounds));
    let speech = world_speech_overlay(observation, camera, npc_pixel_offset);
    actors.extend(speech.images);
    let mut layers = vec![
        map,
        ViewLayer::new(LayerKind::Character)
            .with_images(actors)
            .with_labels(speech.labels),
        ViewLayer::new(LayerKind::Hud),
    ];
    if let Some(console) = console {
        layers.push(project_console(console));
    }
    GameView::new(layers)
}

struct WorldSpeechOverlay {
    images: Vec<ViewImage>,
    labels: Vec<TextLabel>,
}

fn world_speech_overlay(
    observation: &WorldObservation,
    camera: GridPos,
    pixel_offset: PixelOffset,
) -> WorldSpeechOverlay {
    let mut images = Vec::new();
    let mut labels = Vec::new();
    for actor in observation.actors() {
        let Some(speech) = actor.speech() else {
            continue;
        };
        let center = i32::from(actor.position().x()) * 2 - camera.col * 2 + 1;
        let row = i32::from(actor.position().y()) * 2 - camera.row * 2 - 2;
        if row < 0 || center < 0 || center >= CANVAS_WIDTH as i32 {
            continue;
        }
        let content = speech_text(speech.as_str());
        let width = (content.chars().count() as u32 * 2 + 2).clamp(10, 18);
        let max_col = CANVAS_WIDTH.saturating_sub(width) as i32;
        let col = (center - width as i32 / 2).clamp(0, max_col) as u32;
        let row = row as u32;
        images.push(
            rounded_image(col, row, width, 2, SPEECH_BUBBLE, 100).with_pixel_offset(pixel_offset),
        );
        labels.push(label(
            TextRole::Message,
            col + 1,
            row,
            width.saturating_sub(2),
            2,
            content,
            TEXT,
        ));
    }
    WorldSpeechOverlay { images, labels }
}

fn speech_text(text: &str) -> &str {
    match text {
        "text:guide_hello" => "前方的小路很安全。",
        "text:ranger_welcome" => "森林里要注意脚下。",
        "text:collector_found" => "我刚找到一个好东西。",
        "text:hello_there" => "你好。",
        _ => "……",
    }
}

pub fn with_console(mut layers: Vec<ViewLayer>, console: Option<&CommandConsoleView>) -> GameView {
    if let Some(console) = console {
        layers.push(project_console(console));
    }
    GameView::new(layers)
}

fn world_actor_images(
    observation: &WorldObservation,
    animation: WorldAnimation,
    sprite_frame: usize,
    player_pixel_offset: PixelOffset,
    npc_pixel_offset: PixelOffset,
) -> Vec<ViewImage> {
    let mut actors = observation.actors().to_vec();
    actors.sort_by(|left, right| {
        (left.position().y(), left.position().x(), left.id().as_str()).cmp(&(
            right.position().y(),
            right.position().x(),
            right.id().as_str(),
        ))
    });
    actors
        .iter()
        .enumerate()
        .map(|(index, actor)| {
            let (animation, frame, pixel_offset) = match actor.role() {
                WorldActorRole::Player => (animation, sprite_frame, player_pixel_offset),
                WorldActorRole::Npc => (WorldAnimation::Stand, 0, npc_pixel_offset),
            };
            world_actor_image(actor, animation, frame, pixel_offset, 20 + index as u16)
        })
        .collect()
}

fn world_actor_image(
    actor: &WorldActorObservation,
    animation: WorldAnimation,
    sprite_frame: usize,
    pixel_offset: PixelOffset,
    z_index: u16,
) -> ViewImage {
    let position = actor.position();
    ViewImage::new(
        GridRect::new(
            GridPos::new(i32::from(position.x()) * 2, i32::from(position.y()) * 2),
            GridSize::new(2, 2),
        ),
        world_character_asset(actor.appearance(), actor.facing(), animation, sprite_frame),
        Rgba8::new(255, 255, 255, 255),
        z_index,
    )
    .with_pixel_offset(pixel_offset)
}

pub fn world_character_asset(
    appearance: &CharacterAppearanceId,
    direction: WorldDirection,
    animation: WorldAnimation,
    sprite_frame: usize,
) -> AssetKey {
    let direction_index = match direction {
        WorldDirection::Down => 0,
        WorldDirection::Left => 1,
        WorldDirection::Right => 2,
        WorldDirection::Up => 3,
    };
    let frame_offset = match animation {
        WorldAnimation::Stand => 0,
        WorldAnimation::Walk => match sprite_frame % 4 {
            0 => 1,
            1 | 3 => 0,
            _ => 2,
        },
        WorldAnimation::Run => match sprite_frame % 4 {
            0 => 4,
            1 | 3 => 3,
            _ => 5,
        },
        WorldAnimation::RunStopping => 3,
    };
    AssetKey::new(format!(
        "character/{}/{direction_index}/{frame_offset}",
        appearance.as_str()
    ))
    .expect("character asset keys are non-empty")
}

fn active_pokemon(observation: &BattleObservation) -> &Pokemon {
    &observation.own().members()[observation.own().active_slot().index()]
}

fn rounded_image(
    col: u32,
    row: u32,
    width: u32,
    height: u32,
    tint: Rgba8,
    z_index: u16,
) -> ViewImage {
    shape_image(col, row, width, height, rounded_ui_asset(), tint, z_index)
}

fn pill_image(col: u32, row: u32, width: u32, height: u32, tint: Rgba8, z_index: u16) -> ViewImage {
    shape_image(col, row, width, height, pill_ui_asset(), tint, z_index)
}

#[allow(clippy::too_many_arguments)]
fn shape_image(
    col: u32,
    row: u32,
    width: u32,
    height: u32,
    asset: AssetKey,
    tint: Rgba8,
    z_index: u16,
) -> ViewImage {
    ViewImage::new(
        GridRect::new(
            GridPos::new(col as i32, row as i32),
            GridSize::new(width, height),
        ),
        asset,
        tint,
        z_index,
    )
}

fn draw_battlefield(canvas: &mut Canvas) {
    canvas.fill(0, 0, CANVAS_WIDTH, 7, SKY);
    canvas.fill(0, 7, CANVAS_WIDTH, 2, SKY_DEEP);
    canvas.fill(0, 9, CANVAS_WIDTH, 3, DISTANT_GRASS);
    canvas.fill(0, 12, CANVAS_WIDTH, 5, GROUND);
    canvas.fill(0, 16, CANVAS_WIDTH, 1, GROUND_DARK);
    for col in [1, 5, 12, 16, 28] {
        canvas.fill(col, 10, 3, 1, GROUND);
    }
}

fn battlefield_images() -> Vec<ViewImage> {
    vec![
        pill_image(20, 8, 11, 3, PLATFORM_SHADOW, 0),
        pill_image(20, 7, 10, 3, PLATFORM, 1),
        pill_image(1, 14, 14, 3, PLATFORM_SHADOW, 0),
        pill_image(2, 13, 13, 3, PLATFORM, 1),
    ]
}

fn draw_status_panel(
    images: &mut Vec<ViewImage>,
    col: u32,
    row: u32,
    hp: u32,
    max_hp: u32,
    accent: Rgba8,
) {
    images.push(
        rounded_image(col, row, 14, 5, BATTLE_CARD_SHADOW, 0)
            .with_pixel_offset(PixelOffset::new(3, 3)),
    );
    images.push(rounded_image(col, row, 14, 5, BATTLE_CARD, 1));
    images.push(pill_image(col + 1, row + 1, 1, 3, accent, 2));
    draw_hp_bar(images, col + 2, row + 4, 10, hp, max_hp);
}

fn draw_action_panel(
    images: &mut Vec<ViewImage>,
    page: BattleMenuPage,
    action_count: usize,
    selected: usize,
) {
    images.push(rounded_image(0, 17, CANVAS_WIDTH, 7, ACTION_BORDER, 0));
    images.push(rounded_image(1, 18, CANVAS_WIDTH - 2, 5, ACTION_PANEL, 1));
    match page {
        BattleMenuPage::Main => {
            images.push(rounded_image(18, 18, 13, 5, ACTION_PANEL_ALT, 2));
            images.push(pill_image(17, 19, 1, 3, ACTION_BORDER, 2));
            for index in 0..action_count.min(4) {
                if index == selected {
                    let col = 19 + (index as u32 % 2) * 6;
                    let row = 18 + (index as u32 / 2) * 2;
                    images.push(pill_image(col, row.saturating_sub(1), 6, 3, SELECTED, 3));
                }
            }
        }
        BattleMenuPage::Fight => {
            images.push(rounded_image(22, 18, 9, 5, ACTION_PANEL_ALT, 2));
            images.push(pill_image(21, 19, 1, 3, ACTION_BORDER, 2));
            for index in 0..action_count.min(4) {
                if index == selected {
                    let col = 2 + (index as u32 % 2) * 10;
                    let row = 18 + (index as u32 / 2) * 2;
                    images.push(pill_image(col, row.saturating_sub(1), 10, 3, SELECTED, 3));
                }
            }
        }
        BattleMenuPage::Pokemon | BattleMenuPage::Hidden => {}
    }
}

fn draw_team_card(
    images: &mut Vec<ViewImage>,
    col: u32,
    row: u32,
    selected: bool,
    pokemon: &Pokemon,
) {
    images.push(rounded_image(
        col,
        row,
        18,
        3,
        if selected {
            SELECTED_DARK
        } else {
            PARTY_PANEL_ALT
        },
        0,
    ));
    images.push(rounded_image(
        col + 1,
        row + 1,
        1,
        1,
        if pokemon.is_fainted() {
            HP_LOW
        } else if selected {
            SELECTED
        } else {
            PARTY_EDGE
        },
        2,
    ));
    draw_hp_bar(
        images,
        col + 5,
        row + 2,
        11,
        pokemon.current_hp(),
        pokemon.max_hp(),
    );
}

fn pokemon_icon_image(
    bounds: GridRect,
    slot: usize,
    fainted: bool,
    sprite_frame: usize,
) -> ViewImage {
    ViewImage::new(
        bounds,
        pokemon_icon_asset(slot, sprite_frame),
        if fainted {
            Rgba8::new(112, 112, 112, 255)
        } else {
            Rgba8::new(255, 255, 255, 255)
        },
        10,
    )
}

fn draw_hp_bar(images: &mut Vec<ViewImage>, col: u32, row: u32, width: u32, hp: u32, max_hp: u32) {
    if width == 0 {
        return;
    }
    images.push(pill_image(col, row, width, 1, HP_TRACK_EDGE, 3));
    let filled = hp.saturating_mul(width).checked_div(max_hp).unwrap_or(0);
    let (color, glow) = if hp.saturating_mul(4) <= max_hp {
        (HP_LOW, HP_LOW_GLOW)
    } else if hp.saturating_mul(2) <= max_hp {
        (HP_MID, HP_MID_GLOW)
    } else {
        (HP_GOOD, HP_GOOD_GLOW)
    };
    if filled > 0 {
        let filled = filled.min(width);
        images.push(pill_image(col, row, filled, 1, color, 4));
        images.push(pill_image(col + filled - 1, row, 1, 1, glow, 5));
    }
}

fn type_icon_images(
    col: u32,
    row: u32,
    primary: PokemonType,
    secondary: Option<PokemonType>,
) -> Vec<ViewImage> {
    let mut images = vec![type_icon_image(col, row, primary)];
    if let Some(secondary) = secondary {
        images.push(type_icon_image(col + 2, row, secondary));
    }
    images
}

fn type_icon_image(col: u32, row: u32, pokemon_type: PokemonType) -> ViewImage {
    ViewImage::new(
        GridRect::new(GridPos::new(col as i32, row as i32), GridSize::new(2, 1)),
        type_icon_asset(pokemon_type),
        Rgba8::new(255, 255, 255, 255),
        20,
    )
}

fn pokedex_type(id: u16) -> Option<PokemonType> {
    Some(match id {
        1 => PokemonType::Normal,
        2 => PokemonType::Fighting,
        3 => PokemonType::Flying,
        4 => PokemonType::Poison,
        5 => PokemonType::Ground,
        6 => PokemonType::Rock,
        7 => PokemonType::Bug,
        8 => PokemonType::Ghost,
        9 => PokemonType::Steel,
        10 => PokemonType::Fire,
        11 => PokemonType::Water,
        12 => PokemonType::Grass,
        13 => PokemonType::Electric,
        14 => PokemonType::Psychic,
        15 => PokemonType::Ice,
        16 => PokemonType::Dragon,
        17 => PokemonType::Dark,
        _ => return None,
    })
}

pub fn type_icon_asset(pokemon_type: PokemonType) -> AssetKey {
    let name = match pokemon_type {
        PokemonType::Normal => "normal",
        PokemonType::Fighting => "fighting",
        PokemonType::Flying => "flying",
        PokemonType::Poison => "poison",
        PokemonType::Ground => "ground",
        PokemonType::Rock => "rock",
        PokemonType::Bug => "bug",
        PokemonType::Ghost => "ghost",
        PokemonType::Steel => "steel",
        PokemonType::Fire => "fire",
        PokemonType::Water => "water",
        PokemonType::Grass => "grass",
        PokemonType::Electric => "electric",
        PokemonType::Psychic => "psychic",
        PokemonType::Ice => "ice",
        PokemonType::Dragon => "dragon",
        PokemonType::Dark => "dark",
    };
    AssetKey::new(format!("ui/battle/type/{name}")).expect("type icon asset keys are non-empty")
}

fn move_category_icon_image(col: u32, row: u32, category: MoveCategory) -> ViewImage {
    ViewImage::new(
        GridRect::new(GridPos::new(col as i32, row as i32), GridSize::new(2, 1)),
        move_category_icon_asset(category),
        Rgba8::new(255, 255, 255, 255),
        20,
    )
}

pub fn move_category_icon_asset(category: MoveCategory) -> AssetKey {
    let name = match category {
        MoveCategory::Physical => "physical",
        MoveCategory::Special => "special",
        MoveCategory::Status => "status",
    };
    AssetKey::new(format!("ui/battle/move-category/{name}"))
        .expect("move category asset keys are non-empty")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BattleSpriteResources {
    own: [AssetKey; 2],
    opponent: [AssetKey; 2],
}

impl BattleSpriteResources {
    pub fn for_slots(own_slot: usize, opponent_slot: usize) -> Self {
        Self {
            own: [
                player_back_asset(own_slot, 0),
                player_back_asset(own_slot, 1),
            ],
            opponent: [
                opponent_front_asset(opponent_slot, 0),
                opponent_front_asset(opponent_slot, 1),
            ],
        }
    }
}

pub fn player_back_asset(slot: usize, frame: usize) -> AssetKey {
    AssetKey::new(format!("battle/player/{slot}/back/{}", frame % 2))
        .expect("player sprite asset keys are non-empty")
}

pub fn opponent_front_asset(slot: usize, frame: usize) -> AssetKey {
    AssetKey::new(format!("battle/opponent/{slot}/front/{}", frame % 2))
        .expect("opponent sprite asset keys are non-empty")
}

pub fn pokemon_icon_asset(slot: usize, frame: usize) -> AssetKey {
    AssetKey::new(format!("battle/team/{slot}/icon/{}", frame % 2))
        .expect("team icon asset keys are non-empty")
}

pub fn rounded_ui_asset() -> AssetKey {
    AssetKey::new("ui/rounded-rect").expect("the rounded UI asset key is non-empty")
}

pub fn pill_ui_asset() -> AssetKey {
    AssetKey::new("ui/pill").expect("the pill UI asset key is non-empty")
}

fn battle_images(
    animation: BattleAnimation,
    sprites: BattleSpriteResources,
    sprite_frame: usize,
) -> Vec<ViewImage> {
    let player_origin = if animation == BattleAnimation::Acting(Participant::Own) {
        GridPos::new(6, 9)
    } else {
        GridPos::new(5, 10)
    };
    let opponent_origin = if animation == BattleAnimation::Acting(Participant::Opponent) {
        GridPos::new(21, 5)
    } else {
        GridPos::new(22, 4)
    };

    vec![
        ViewImage::new(
            GridRect::new(player_origin, GridSize::new(8, 8)),
            sprites.own[sprite_frame % 2].clone(),
            creature_tint(animation, Participant::Own),
            10,
        ),
        ViewImage::new(
            GridRect::new(opponent_origin, GridSize::new(8, 8)),
            sprites.opponent[sprite_frame % 2].clone(),
            creature_tint(animation, Participant::Opponent),
            10,
        ),
    ]
}

fn creature_tint(animation: BattleAnimation, participant: Participant) -> Rgba8 {
    match animation {
        BattleAnimation::Hit(target) if target == participant => Rgba8::new(255, 112, 112, 255),
        BattleAnimation::Fainted(target) if target == participant => Rgba8::new(112, 112, 112, 255),
        _ => Rgba8::new(255, 255, 255, 255),
    }
}

fn label(
    role: TextRole,
    col: u32,
    row: u32,
    width: u32,
    height: u32,
    content: &str,
    color: Rgba8,
) -> TextLabel {
    TextLabel {
        role,
        col,
        row,
        width,
        height,
        content: content.into(),
        color,
    }
}

struct Canvas {
    cells: Vec<ViewCell>,
}

impl Canvas {
    fn new(color: Rgba8) -> Self {
        Self {
            cells: vec![sprite(color); (CANVAS_WIDTH * CANVAS_HEIGHT) as usize],
        }
    }

    fn set(&mut self, col: u32, row: u32, color: Rgba8) {
        if col < CANVAS_WIDTH && row < CANVAS_HEIGHT {
            self.cells[(row * CANVAS_WIDTH + col) as usize] = sprite(color);
        }
    }

    fn fill(&mut self, col: u32, row: u32, width: u32, height: u32, color: Rgba8) {
        for y in row..row.saturating_add(height).min(CANVAS_HEIGHT) {
            for x in col..col.saturating_add(width).min(CANVAS_WIDTH) {
                self.set(x, y, color);
            }
        }
    }

    fn finish(self) -> Surface<ViewCell> {
        Surface::from_cells(GridSize::new(CANVAS_WIDTH, CANVAS_HEIGHT), self.cells)
            .expect("the fixed battle canvas dimensions are valid")
    }
}

const fn sprite(tint: Rgba8) -> ViewCell {
    ViewCell::Fill(tint)
}

#[cfg(test)]
mod tests {
    use battle_application::{
        Accuracy, BattleApplication, BattleStats, Move, MoveCategory, MoveId, Pokemon, PokemonId,
        PokemonType, TEAM_SIZE, Team,
    };
    use battle_session::{
        Action, BattleCoordinator, BattleObservation, BattleSession, BattleSessionSnapshot,
        OpponentPolicy,
    };
    use game_data::PokedexData;
    use map_project::{
        AtomicTileId, CompositeTile, CompositeTileId, MapActor, MapActorId, MapDirection,
        MapProject, MapProjectId, TilePosition,
    };
    use punctum_grid::{GridPos, GridSize};
    use punctum_input::{KeyEvent, KeyPhase, LogicalKey, Modifiers, NamedKey, PhysicalKeyCode};
    use world_application::{CharacterAppearanceId, Direction, Position, WorldApplication};

    use game_ui::{
        BattleMenuPage, BattleUiOutcome, BattleUiState, CommandConsoleView, PokedexAction,
        WorldAnimation,
    };

    use super::{
        BattleSpriteResources, LayerKind, TextRole, ViewCell, ViewLayer, compose_world,
        move_category_icon_asset, pill_ui_asset, pokemon_icon_asset, project_battle,
        project_battle_ui, project_console, project_console_ui, project_pokedex, project_world,
        rounded_ui_asset, type_icon_asset, world_character_asset,
    };

    #[test]
    fn pokedex_projects_its_selected_canonical_front() {
        let data = PokedexData::embedded_gen3().unwrap();
        let tree = project_pokedex(&data, 0).unwrap();
        for viewport in [
            punctum_ui::UiSize::new(960, 720),
            punctum_ui::UiSize::new(640, 480),
            punctum_ui::UiSize::new(320, 240),
        ] {
            assert!(tree.resolve(viewport).is_ok());
        }
        let frame = tree.resolve(punctum_ui::UiSize::new(960, 720)).unwrap();
        assert!(frame.commands().iter().any(|command| matches!(command,
                punctum_ui::UiDrawCommand::Text { content, .. } if content == "妙蛙种子")));
        assert!(
            frame.commands().iter().any(|command| matches!(command,
                punctum_ui::UiDrawCommand::Image { content, .. } if content.as_str() == "pokedex/1"))
        );
        assert!(frame.commands().iter().any(|command| matches!(
            command,
            punctum_ui::UiDrawCommand::Image { content, border_radius, .. }
                if content.as_str() == "pokedex/1" && *border_radius == punctum_ui::UiBorderRadius::all(12)
        )));
        assert!(frame.commands().iter().any(|command| matches!(
            command,
            punctum_ui::UiDrawCommand::Fill { color, border_radius, .. }
                if *color == super::UiColor::new(237, 242, 233, 255)
                    && *border_radius == punctum_ui::UiBorderRadius::all(16)
        )));
        assert!(
            frame.commands().iter().any(|command| matches!(command,
                punctum_ui::UiDrawCommand::Image { content, .. } if content.as_str() == type_icon_asset(PokemonType::Grass).as_str()))
        );
        assert!(frame.hit_regions().len() == 5);
        assert_eq!(tree.root().id, punctum_ui::UiId(0));
        assert_eq!(frame.action_hits().len(), 5);
        assert_eq!(
            frame.action_hits()[0].action,
            PokedexAction::SelectEntry { index: 0 }
        );
        assert_eq!(
            frame.action_hits()[0]
                .key
                .as_ref()
                .map(punctum_ui::UiKey::as_str),
            Some("pokedex-entry-1")
        );
    }

    #[test]
    fn battle_pixel_ui_uses_flex_and_keeps_move_metadata_visible() {
        let snapshot = battle_fixture();
        let sprites = BattleSpriteResources::for_slots(0, 0);
        let main = project_battle_ui(&snapshot, BattleUiState::default(), sprites.clone(), 0)
            .unwrap()
            .resolve(punctum_ui::UiSize::new(1000, 720))
            .unwrap();
        assert_eq!(main.hit_regions().len(), 4);
        assert!(
            main.hit_regions()
                .iter()
                .all(|region| region.bounds.width > 0 && region.bounds.height > 0)
        );
        assert!(main.commands().iter().any(|command| matches!(
            command,
            punctum_ui::UiDrawCommand::Text { content, .. } if content == "战斗"
        )));

        let mut ui = BattleUiState::default();
        handle_battle_key(&mut ui, &key(NamedKey::Enter), snapshot.interaction());
        let fight = project_battle_ui(&snapshot, ui, sprites, 0)
            .unwrap()
            .resolve(punctum_ui::UiSize::new(1000, 720))
            .unwrap();
        assert!(fight.commands().iter().any(|command| matches!(
            command,
            punctum_ui::UiDrawCommand::Text { content, .. } if content == "威40 PP35/35"
        )));
        let images = fight
            .commands()
            .iter()
            .filter_map(|command| match command {
                punctum_ui::UiDrawCommand::Image { content, .. } => Some(content.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(images.contains(&type_icon_asset(PokemonType::Grass).as_str()));
        assert!(images.contains(&move_category_icon_asset(MoveCategory::Special).as_str()));
    }

    #[test]
    fn pokemon_selection_pixel_ui_has_a_detail_panel_and_all_team_members() {
        let snapshot = battle_fixture();
        let mut ui = BattleUiState::default();
        handle_battle_key(&mut ui, &key(NamedKey::ArrowRight), snapshot.interaction());
        handle_battle_key(&mut ui, &key(NamedKey::Enter), snapshot.interaction());

        let frame = project_battle_ui(&snapshot, ui, BattleSpriteResources::for_slots(0, 0), 1)
            .unwrap()
            .resolve(punctum_ui::UiSize::new(1000, 720))
            .unwrap();
        assert!(frame.commands().iter().any(|command| matches!(
            command,
            punctum_ui::UiDrawCommand::Text { content, .. } if content == "选择宝可梦"
        )));
        assert_eq!(frame.hit_regions().len(), TEAM_SIZE);
        let images = frame
            .commands()
            .iter()
            .filter_map(|command| match command {
                punctum_ui::UiDrawCommand::Image { content, .. } => Some(content.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        for slot in 0..TEAM_SIZE {
            assert!(images.contains(&pokemon_icon_asset(slot, 1).as_str()));
        }
        assert!(images.contains(&type_icon_asset(PokemonType::Poison).as_str()));
    }

    #[test]
    fn console_pixel_ui_preserves_the_legacy_overlay_without_an_opaque_root() {
        let console = CommandConsoleView {
            query: "gi".to_owned(),
            preedit: "t".to_owned(),
            items: vec!["give potion".to_owned(), "goto town".to_owned()],
            selected_index: Some(1),
            diagnostic: Some("invalid target".to_owned()),
        };
        let frame = project_console_ui(&console)
            .unwrap()
            .resolve(punctum_ui::UiSize::new(1000, 720))
            .unwrap();
        assert!(!frame.commands().iter().any(|command| matches!(
            command,
            punctum_ui::UiDrawCommand::Fill {
                bounds,
                color,
                ..
            } if *bounds == punctum_ui::UiRect::new(0, 0, 1000, 720)
                && color.alpha == 255
        )));
        let actual_labels = frame
            .commands()
            .iter()
            .filter_map(|command| match command {
                punctum_ui::UiDrawCommand::Text { content, .. } => Some(content.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            actual_labels,
            ["> git", "give potion", "goto town", "invalid target"]
        );
    }

    fn key(name: NamedKey) -> KeyEvent {
        KeyEvent {
            physical: Some(PhysicalKeyCode::Unidentified),
            logical: LogicalKey::Named(name),
            modifiers: Modifiers::default(),
            phase: KeyPhase::Press,
        }
    }

    fn handle_battle_key(
        ui: &mut BattleUiState,
        key: &KeyEvent,
        interaction: &battle_session::BattleInteraction,
    ) -> BattleUiOutcome {
        let (next, outcome) = (*ui).handle_key(key, interaction);
        *ui = next;
        outcome
    }

    struct FirstActionPolicy;

    impl OpponentPolicy for FirstActionPolicy {
        fn choose_action(
            &self,
            _observation: &BattleObservation,
            legal_actions: &[Action],
        ) -> Option<Action> {
            legal_actions.first().copied()
        }
    }

    fn battle_session_with_move(
        current_pp: u8,
        accuracy: Accuracy,
    ) -> BattleSession<FirstActionPolicy> {
        battle_session_with_team_state(current_pp, accuracy, false)
    }

    fn battle_session_with_team_state(
        current_pp: u8,
        accuracy: Accuracy,
        fainted_own_bench: bool,
    ) -> BattleSession<FirstActionPolicy> {
        fn team(
            prefix: &str,
            move_type: PokemonType,
            current_pp: u8,
            accuracy: Accuracy,
            fainted_bench: bool,
        ) -> Team {
            let members = (0..TEAM_SIZE)
                .map(|index| {
                    let battle_move = Move::new(
                        MoveId::new(format!("{prefix}-move-{index}")).unwrap(),
                        if index == 0 { "撞击" } else { "电光一闪" },
                        move_type,
                        40,
                        accuracy,
                        35,
                        current_pp,
                        0,
                    )
                    .unwrap();
                    let alternate_move = Move::new(
                        MoveId::new(format!("{prefix}-alternate-{index}")).unwrap(),
                        "飞叶快刀",
                        move_type,
                        55,
                        accuracy,
                        25,
                        current_pp.min(25),
                        0,
                    )
                    .unwrap();
                    let current_hp = if fainted_bench && index == 1 {
                        0
                    } else {
                        80 + index as u32
                    };
                    Pokemon::new(
                        PokemonId::new(format!("{prefix}-{index}")).unwrap(),
                        format!("{prefix}{index}"),
                        24,
                        move_type,
                        (index == 0).then_some(PokemonType::Poison),
                        80 + index as u32,
                        current_hp,
                        BattleStats::new(50, 50, 50, 50, 50).unwrap(),
                        vec![battle_move, alternate_move],
                    )
                    .unwrap()
                })
                .collect();
            Team::new(members).unwrap()
        }

        let application = BattleApplication::new(
            team(
                "己方",
                PokemonType::Grass,
                current_pp,
                accuracy,
                fainted_own_bench,
            ),
            team("对手", PokemonType::Fire, current_pp, accuracy, false),
            42,
        )
        .unwrap();
        BattleSession::new(BattleCoordinator::new(application, FirstActionPolicy))
    }

    fn battle_session_fixture() -> BattleSession<FirstActionPolicy> {
        battle_session_with_move(35, Accuracy::AlwaysHit)
    }

    fn battle_fixture() -> BattleSessionSnapshot {
        battle_session_fixture().snapshot()
    }

    #[test]
    fn main_menu_routes_to_fight_pokemon_bag_and_run() {
        let snapshot = battle_fixture();
        let interaction = snapshot.interaction();
        let mut ui = BattleUiState::default();

        assert_eq!(
            handle_battle_key(&mut ui, &key(NamedKey::Enter), interaction),
            BattleUiOutcome::Updated
        );
        assert_eq!(ui.view().0, BattleMenuPage::Fight);
        assert_eq!(
            handle_battle_key(&mut ui, &key(NamedKey::Enter), interaction),
            BattleUiOutcome::Submit(Action::UseMove(battle_session::MoveSlot::new(0).unwrap()))
        );

        ui = BattleUiState::default();
        assert_eq!(
            handle_battle_key(&mut ui, &key(NamedKey::ArrowRight), interaction),
            BattleUiOutcome::Updated
        );
        assert_eq!(
            handle_battle_key(&mut ui, &key(NamedKey::Enter), interaction),
            BattleUiOutcome::Updated
        );
        assert_eq!(ui.view().0, BattleMenuPage::Pokemon);
        assert_eq!(
            handle_battle_key(&mut ui, &key(NamedKey::Enter), interaction),
            BattleUiOutcome::Updated
        );
        handle_battle_key(&mut ui, &key(NamedKey::ArrowRight), interaction);
        assert_eq!(
            handle_battle_key(&mut ui, &key(NamedKey::Enter), interaction),
            BattleUiOutcome::Submit(Action::Switch(battle_session::TeamSlot::new(1).unwrap()))
        );

        ui = BattleUiState::default();
        handle_battle_key(&mut ui, &key(NamedKey::ArrowRight), interaction);
        handle_battle_key(&mut ui, &key(NamedKey::ArrowRight), interaction);
        assert_eq!(
            handle_battle_key(&mut ui, &key(NamedKey::Enter), interaction),
            BattleUiOutcome::Updated
        );
        assert_eq!(ui.view().0, BattleMenuPage::Main);

        ui = BattleUiState::default();
        handle_battle_key(&mut ui, &key(NamedKey::ArrowLeft), interaction);
        assert_eq!(ui.view().1, 3);
        assert_eq!(
            handle_battle_key(&mut ui, &key(NamedKey::Enter), interaction),
            BattleUiOutcome::Submit(Action::Run)
        );
    }

    #[test]
    fn battle_projection_shows_status_and_move_details() {
        let snapshot = battle_fixture();
        let mut ui = BattleUiState::default();
        let main = project_battle(&snapshot, ui, BattleSpriteResources::for_slots(0, 0), 0);
        let commands = main
            .labels()
            .filter(|label| matches!(label.role, TextRole::Action(_)))
            .map(|label| label.content.as_str())
            .collect::<Vec<_>>();
        assert_eq!(commands, ["战斗", "宝可梦", "包包", "逃走"]);
        assert!(
            main.labels()
                .any(|label| { label.role == TextRole::PlayerDetail && label.content == "Lv.24" })
        );
        assert!(
            main.images()
                .any(|image| image.asset == type_icon_asset(PokemonType::Grass))
        );
        assert!(
            main.images()
                .any(|image| image.asset == type_icon_asset(PokemonType::Poison))
        );
        assert!(
            main.labels()
                .any(|label| { label.role == TextRole::PlayerHp && label.content == "HP 80/80" })
        );

        handle_battle_key(&mut ui, &key(NamedKey::Enter), snapshot.interaction());
        let fight = project_battle(&snapshot, ui, BattleSpriteResources::for_slots(0, 0), 0);
        assert!(fight.labels().any(|label| {
            label.role == TextRole::ActionDetail(0) && label.content == "威40 PP35/35"
        }));
        assert!(fight.labels().any(|label| {
            label.role == TextRole::Action(1)
                && label.content == "飞叶快刀"
                && label.color == super::TEXT
        }));
        assert!(fight.images().any(|image| {
            image.asset == type_icon_asset(PokemonType::Grass)
                && image.bounds.origin == GridPos::new(23, 18)
        }));
        assert!(fight.images().any(|image| {
            image.asset == move_category_icon_asset(MoveCategory::Special)
                && image.bounds.origin == GridPos::new(26, 18)
        }));
    }

    #[test]
    fn pokemon_selection_uses_animated_team_icons() {
        let snapshot = battle_fixture();
        let mut ui = BattleUiState::default();
        handle_battle_key(&mut ui, &key(NamedKey::ArrowRight), snapshot.interaction());
        handle_battle_key(&mut ui, &key(NamedKey::Enter), snapshot.interaction());

        let view = project_battle(&snapshot, ui, BattleSpriteResources::for_slots(0, 0), 0);

        assert!(view.labels().any(|label| {
            label.role == TextRole::PageTitle && label.content == "选择宝可梦"
        }));
        assert!(view.labels().any(|label| {
            label.role == TextRole::SelectedMemberDetail && label.content == "Lv.24  出战"
        }));
        assert_eq!(
            view.labels()
                .filter(|label| matches!(label.role, TextRole::TeamMember(_)))
                .count(),
            TEAM_SIZE
        );
        assert_eq!(
            view.images()
                .filter(|image| image.asset.as_str().starts_with("battle/team/"))
                .count(),
            TEAM_SIZE + 1
        );
        assert!(view.images().any(|image| {
            image.asset == rounded_ui_asset()
                && image.bounds.origin == GridPos::new(1, 4)
                && image.bounds.size == GridSize::new(11, 17)
        }));
        assert!(view.images().any(|image| {
            image.asset == rounded_ui_asset()
                && image.bounds.origin == GridPos::new(13, 4)
                && image.bounds.size == GridSize::new(18, 3)
        }));
        for slot in 0..TEAM_SIZE {
            assert!(
                view.images()
                    .any(|image| image.asset == super::pokemon_icon_asset(slot, 0))
            );
        }
        assert!(view.images().any(|image| {
            image.asset == type_icon_asset(PokemonType::Poison)
                && image.bounds.origin == GridPos::new(5, 16)
        }));

        let animated = project_battle(&snapshot, ui, BattleSpriteResources::for_slots(0, 0), 1);
        assert!(
            animated
                .images()
                .any(|image| image.asset == super::pokemon_icon_asset(0, 1))
        );
        assert!(view.images().any(|image| {
            image.asset == super::pokemon_icon_asset(0, 0)
                && image.bounds.origin == GridPos::new(3, 5)
                && image.bounds.size == GridSize::new(7, 7)
        }));
        assert!(view.images().any(|image| {
            image.asset == super::pokemon_icon_asset(0, 0)
                && image.bounds.origin == GridPos::new(14, 4)
                && image.bounds.size == GridSize::new(3, 3)
        }));
    }

    #[test]
    fn selected_fainted_pokemon_uses_the_unavailable_status() {
        let snapshot = battle_session_with_team_state(35, Accuracy::AlwaysHit, true).snapshot();
        let mut ui = BattleUiState::default();
        handle_battle_key(&mut ui, &key(NamedKey::ArrowRight), snapshot.interaction());
        handle_battle_key(&mut ui, &key(NamedKey::Enter), snapshot.interaction());
        handle_battle_key(&mut ui, &key(NamedKey::ArrowRight), snapshot.interaction());

        let view = project_battle(&snapshot, ui, BattleSpriteResources::for_slots(0, 0), 0);
        assert!(view.labels().any(|label| {
            label.role == TextRole::SelectedMemberHp
                && label.content == "无法战斗"
                && label.color == super::HP_LOW
        }));
    }

    #[test]
    fn zero_width_hp_bar_draws_nothing() {
        let mut images = Vec::new();
        super::draw_hp_bar(&mut images, 0, 0, 0, 1, 1);
        assert!(images.is_empty());
    }

    #[test]
    fn product_assets_use_stable_semantic_keys() {
        assert_eq!(rounded_ui_asset().as_str(), "ui/rounded-rect");
        assert_eq!(pill_ui_asset().as_str(), "ui/pill");
        let types = [
            PokemonType::Normal,
            PokemonType::Fighting,
            PokemonType::Flying,
            PokemonType::Poison,
            PokemonType::Ground,
            PokemonType::Rock,
            PokemonType::Bug,
            PokemonType::Ghost,
            PokemonType::Steel,
            PokemonType::Fire,
            PokemonType::Water,
            PokemonType::Grass,
            PokemonType::Electric,
            PokemonType::Psychic,
            PokemonType::Ice,
            PokemonType::Dragon,
            PokemonType::Dark,
        ];
        for pokemon_type in types {
            assert!(
                type_icon_asset(pokemon_type)
                    .as_str()
                    .starts_with("ui/battle/type/")
            );
        }
        for category in [
            MoveCategory::Physical,
            MoveCategory::Special,
            MoveCategory::Status,
        ] {
            assert!(
                move_category_icon_asset(category)
                    .as_str()
                    .starts_with("ui/battle/move-category/")
            );
        }
        for direction in [
            Direction::Down,
            Direction::Left,
            Direction::Right,
            Direction::Up,
        ] {
            let appearance = CharacterAppearanceId::new("red").unwrap();
            for animation in [
                WorldAnimation::Stand,
                WorldAnimation::Walk,
                WorldAnimation::Run,
                WorldAnimation::RunStopping,
            ] {
                for frame in 0..4 {
                    assert!(
                        world_character_asset(&appearance, direction, animation, frame)
                            .as_str()
                            .starts_with("character/")
                    );
                }
            }
        }
    }

    #[test]
    fn message_and_tint_tables_cover_every_semantic_variant() {
        use battle_session::{ObservedBattleOutcome, Participant, TypeEffectiveness, UsedMove};

        for outcome in [
            ObservedBattleOutcome::Winner(Participant::Own),
            ObservedBattleOutcome::Winner(Participant::Opponent),
            ObservedBattleOutcome::Escaped(Participant::Own),
            ObservedBattleOutcome::Escaped(Participant::Opponent),
            ObservedBattleOutcome::Draw,
        ] {
            assert!(!super::outcome_message(outcome).is_empty());
        }
        for effectiveness in [
            TypeEffectiveness::Immune,
            TypeEffectiveness::Quarter,
            TypeEffectiveness::Half,
            TypeEffectiveness::Normal,
            TypeEffectiveness::Double,
            TypeEffectiveness::Quadruple,
        ] {
            assert!(!super::effectiveness_message(effectiveness).is_empty());
        }
        assert_eq!(super::used_move_name(&UsedMove::Struggle), "挣扎");
        let used = UsedMove::Move {
            id: MoveId::new("move").unwrap(),
            name: "招式".into(),
        };
        assert_eq!(super::used_move_name(&used), "招式");
        for animation in [
            super::BattleAnimation::Idle,
            super::BattleAnimation::Acting(Participant::Own),
            super::BattleAnimation::Acting(Participant::Opponent),
            super::BattleAnimation::Hit(Participant::Own),
            super::BattleAnimation::Fainted(Participant::Opponent),
        ] {
            for participant in [Participant::Own, Participant::Opponent] {
                assert_eq!(super::creature_tint(animation, participant).alpha, 255);
            }
        }
    }

    #[test]
    fn a_complete_battle_story_projects_every_reachable_cue() {
        let mut battle = battle_session_fixture();
        let mut ui = BattleUiState::default();
        let mut projected = 0;
        for _ in 0..2_000 {
            let snapshot = battle.snapshot();
            ui = ui.synced(snapshot.interaction());
            let view = project_battle(
                &snapshot,
                ui,
                BattleSpriteResources::for_slots(0, 0),
                projected,
            );
            assert_eq!(view.layers()[0].kind, LayerKind::Map);
            projected += 1;
            if battle.is_finished() {
                break;
            }
            if battle.has_pending_playback() {
                let (next, advanced) = battle.advance();
                battle = next;
                assert!(advanced);
            } else {
                let action = battle.legal_actions()[0];
                let (next, result) = battle.submit(action);
                battle = next;
                result.unwrap();
            }
        }
        assert!(battle.is_finished());
        assert!(projected > 20);
    }

    #[test]
    fn exhausted_moves_and_misses_have_complete_battle_views() {
        let struggle = battle_session_with_move(0, Accuracy::AlwaysHit).snapshot();
        let mut ui = BattleUiState::default();
        handle_battle_key(&mut ui, &key(NamedKey::Enter), struggle.interaction());
        let view = project_battle(&struggle, ui, BattleSpriteResources::for_slots(0, 0), 0);
        assert!(view.labels().any(|label| label.content == "挣扎"));

        let mut battle = battle_session_with_move(35, Accuracy::percent(1).unwrap());
        let action = battle.legal_actions()[0];
        let (next, result) = battle.submit(action);
        battle = next;
        result.unwrap();
        let mut saw_miss = false;
        while battle.has_pending_playback() {
            (battle, _) = battle.advance();
            let snapshot = battle.snapshot();
            if matches!(
                snapshot.cue(),
                Some(battle_session::BattleCue::Missed { .. })
            ) {
                let view = project_battle(
                    &snapshot,
                    BattleUiState::default(),
                    BattleSpriteResources::for_slots(0, 0),
                    0,
                );
                assert!(view.labels().any(|label| label.content == "攻击没有命中。"));
                saw_miss = true;
            }
        }
        assert!(saw_miss);
    }

    #[test]
    fn world_projection_uses_stable_character_keys_and_explicit_layers() {
        let world = WorldApplication::demo().unwrap();
        let observation = world.observe();
        let appearance = observation.actors()[0].appearance();
        let view = project_world(&observation);

        assert_eq!(world.observe().player(), Position::new(3, 6));
        assert_eq!(
            view.layers()
                .iter()
                .map(|layer| layer.kind)
                .collect::<Vec<_>>(),
            [LayerKind::Map, LayerKind::Character, LayerKind::Hud]
        );
        assert_eq!(
            view.layers()[0].surface.as_ref().unwrap().size(),
            GridSize::new(32, 24)
        );
        assert_eq!(view.images().count(), 1);
        assert_eq!(
            view.images().next().unwrap().asset,
            world_character_asset(appearance, Direction::Down, WorldAnimation::Stand, 0)
        );
        assert_eq!(
            world_character_asset(appearance, Direction::Up, WorldAnimation::Run, 2).as_str(),
            "character/red/3/5"
        );
        assert_eq!(
            world_character_asset(appearance, Direction::Up, WorldAnimation::RunStopping, 99)
                .as_str(),
            "character/red/3/3"
        );
    }

    #[test]
    fn world_camera_motion_does_not_apply_the_map_offset_to_the_character() {
        let world = WorldApplication::demo().unwrap();
        let map = ViewLayer::new(LayerKind::Map).with_surface(
            punctum_grid::Surface::filled(GridSize::new(32, 24), ViewCell::Empty).unwrap(),
        );
        let view = compose_world(
            map,
            GridPos::new(-5, 0),
            &world.observe(),
            WorldAnimation::Walk,
            1,
            punctum_gpu::PixelOffset::new(0, 0),
            None,
        );

        assert_eq!(
            view.layers()[1].images[0].pixel_offset,
            punctum_gpu::PixelOffset::new(0, 0)
        );
        let map = ViewLayer::new(LayerKind::Map).with_surface(
            punctum_grid::Surface::filled(GridSize::new(32, 24), ViewCell::Empty).unwrap(),
        );
        let with_console = compose_world(
            map,
            GridPos::new(-5, 0),
            &world.observe(),
            WorldAnimation::Stand,
            0,
            punctum_gpu::PixelOffset::new(0, 0),
            Some(&CommandConsoleView::default()),
        );
        assert_eq!(with_console.layers().len(), 4);
    }

    #[test]
    fn world_projection_adds_a_speech_label_for_an_actor_speaking() {
        let material = CompositeTile::new(
            CompositeTileId::new("ground").unwrap(),
            vec![AtomicTileId::new("tile-0001").unwrap()],
        );
        let mut project =
            MapProject::blank(MapProjectId::new("speech").unwrap(), 6, 4, Some(material));
        project.player_spawn = TilePosition::new(0, 0);
        project.actors.push(MapActor::new(
            MapActorId::new("guide").unwrap(),
            TilePosition::new(2, 1),
            MapDirection::Left,
            CharacterAppearanceId::new("dppt/000").unwrap(),
        ));
        let mut continuations = std::collections::BTreeMap::new();
        continuations.insert(
            narrative_cps::ContinuationId::new(0),
            narrative_cps::CpsNode::Say {
                text: narrative_cps::TextId::new("text:hello_there").unwrap(),
                next: narrative_cps::ContinuationId::new(1),
            },
        );
        continuations.insert(
            narrative_cps::ContinuationId::new(1),
            narrative_cps::CpsNode::End,
        );
        let script = narrative_cps::ScriptProgram::with_actor(
            narrative_cps::ScriptId::new("script:guide").unwrap(),
            Some(narrative_cps::ActorId::new("actor:guide").unwrap()),
            narrative_cps::ContinuationId::new(0),
            continuations,
        )
        .unwrap();
        let observation = WorldApplication::from_map_project_with_scripts(&project, [script])
            .unwrap()
            .advance_npcs()
            .observe();
        let view = project_world(&observation);
        assert!(
            view.labels()
                .any(|label| label.role == TextRole::Message && label.content == "你好。")
        );
        let speech_background = view.layers()[1]
            .images
            .iter()
            .find(|image| image.asset == rounded_ui_asset())
            .unwrap();
        assert_eq!(speech_background.tint, super::SPEECH_BUBBLE);
        assert_eq!(speech_background.bounds.size, GridSize::new(10, 2));
    }

    #[test]
    fn command_console_is_an_explicit_top_layer() {
        let layer = project_console(&CommandConsoleView {
            query: "move".into(),
            preedit: "中".into(),
            items: vec!["/battle/move/one use".into(), "/battle/move/two use".into()],
            selected_index: Some(1),
            diagnostic: Some("action rejected".into()),
        });

        assert_eq!(layer.kind, LayerKind::Console);
        assert!(
            layer.labels.iter().any(|label| {
                label.role == TextRole::ConsoleQuery && label.content == "> move中"
            })
        );
        assert!(layer.labels.iter().any(|label| {
            label.role == TextRole::ConsoleItem(1) && label.content == "/battle/move/two use"
        }));
        assert!(
            layer
                .labels
                .iter()
                .any(|label| label.role == TextRole::ConsoleDiagnostic)
        );
        assert!(layer.surface.unwrap().get(GridPos::new(2, 9)).is_ok());

        let empty = project_console(&CommandConsoleView::default());
        assert!(
            empty
                .labels
                .iter()
                .any(|label| label.content == "没有匹配指令")
        );
        let items = (0..12).map(|index| format!("item-{index}")).collect();
        let scrolled = project_console(&CommandConsoleView {
            items,
            selected_index: Some(11),
            ..CommandConsoleView::default()
        });
        assert_eq!(
            scrolled
                .labels
                .iter()
                .filter(|label| matches!(label.role, TextRole::ConsoleItem(_)))
                .count(),
            8
        );
        assert_eq!(super::visible_console_start(12, Some(11)), 4);
        assert_eq!(super::visible_console_start(3, None), 0);

        let world = project_world(&WorldApplication::demo().unwrap().observe());
        assert_eq!(
            super::with_console(world.layers().to_vec(), None)
                .layers()
                .len(),
            3
        );
        assert_eq!(
            super::with_console(
                world.layers().to_vec(),
                Some(&CommandConsoleView::default())
            )
            .layers()
            .last()
            .unwrap()
            .kind,
            LayerKind::Console
        );
    }
}
