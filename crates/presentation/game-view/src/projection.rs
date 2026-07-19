//! 纯 Gen3 产品视图投影。

use std::{error::Error, fmt};

use battle_session::{
    Ability, Action, BattleCue, BattleInteraction, BattleObservation, BattleSessionSnapshot,
    MoveCategory, ObservedBattleOutcome, Participant, Pokemon, PokemonType, TypeEffectiveness,
    UsedMove,
};
use game_assets::AssetKey;
use game_data::PokedexData;
use game_foundation::{Direction as FoundationDirection, GameState, ThinSliceContent};
use game_ui::{BattleMenuPage, BattleUiState, CommandConsoleView, PokedexAction, WorldAnimation};
use game_ui_kit::{
    GameUiTheme, PanelTone, SpriteAppearance, TextTone, button as ui_button, column as ui_column,
    image as ui_image, modal as ui_modal, panel as ui_panel, row as ui_row, screen as ui_screen,
    selectable_list_item as ui_selectable_list_item, sprite as ui_sprite, text as ui_text,
};
use punctum_gpu::{PixelOffset, Rgba8};
use punctum_grid::{GridPos, GridRect, GridSize, Surface, SurfaceError};
use punctum_ui::{
    CrossAlign, Dimension, FlexDirection, Insets, MainAlign, UiBuildError, UiColor, UiContent,
    UiContentId, UiKey, UiNode, UiStyle, UiTree,
};
use world_application::{
    CharacterAppearanceId, Direction as WorldDirection, WorldActorObservation, WorldActorRole,
    WorldObservation,
};

pub const CANVAS_WIDTH: u32 = 32;
pub const CANVAS_HEIGHT: u32 = 24;
const SPEECH_BUBBLE_HEIGHT: u32 = 2;

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

const BATTLE_THEME: GameUiTheme = GameUiTheme {
    screen: UiColor::new(146, 211, 218, 255),
    header: UiColor::new(19, 25, 34, 255),
    panel: UiColor::new(30, 38, 49, 255),
    selected: UiColor::new(73, 211, 168, 255),
    selected_text: UiColor::new(26, 39, 45, 255),
    card: UiColor::new(242, 246, 239, 255),
    image_backdrop: UiColor::new(75, 143, 105, 255),
    text: UiColor::new(244, 246, 239, 255),
    muted_text: UiColor::new(182, 194, 194, 255),
    ink: UiColor::new(26, 39, 45, 255),
    muted_ink: UiColor::new(82, 96, 98, 255),
    small_spacing: 6,
    medium_spacing: 10,
    large_spacing: 16,
    small_radius: punctum_ui::UiBorderRadius::all(6),
    medium_radius: punctum_ui::UiBorderRadius::all(10),
    large_radius: punctum_ui::UiBorderRadius::all(12),
    body_text_size: 18,
    title_text_size: 24,
};

const FOUNDATION_THEME: GameUiTheme = GameUiTheme {
    screen: UiColor::new(14, 22, 32, 255),
    header: UiColor::new(26, 68, 79, 255),
    panel: UiColor::new(28, 44, 56, 255),
    selected: UiColor::new(56, 151, 123, 255),
    selected_text: UiColor::new(20, 31, 36, 255),
    card: UiColor::new(222, 234, 222, 255),
    image_backdrop: UiColor::new(132, 181, 153, 255),
    text: UiColor::new(245, 248, 240, 255),
    muted_text: UiColor::new(183, 203, 199, 255),
    ink: UiColor::new(22, 40, 45, 255),
    muted_ink: UiColor::new(72, 97, 96, 255),
    small_spacing: 8,
    medium_spacing: 16,
    large_spacing: 24,
    small_radius: punctum_ui::UiBorderRadius::all(6),
    medium_radius: punctum_ui::UiBorderRadius::all(8),
    large_radius: punctum_ui::UiBorderRadius::all(10),
    body_text_size: 18,
    title_text_size: 28,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoundationPage {
    Journey,
    Bag,
    TrainerCard,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoundationPageAction {
    SelectPage(FoundationPage),
    Move(FoundationDirection),
    Interact,
    Encounter,
    ResolveBattle,
    BuyPotion,
    Save,
    Close,
}

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

/// 固定画布和世界图层组合期间产生的投影错误。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectionError {
    Surface(SurfaceError),
    ExpectedMapLayer { actual: LayerKind },
    MapLayerMissingSurface,
}

impl fmt::Display for ProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Surface(error) => write!(formatter, "fixed view surface failed: {error}"),
            Self::ExpectedMapLayer { actual } => {
                write!(formatter, "expected a map layer, received {actual:?}")
            }
            Self::MapLayerMissingSurface => write!(formatter, "map layer is missing its surface"),
        }
    }
}

impl Error for ProjectionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Surface(error) => Some(error),
            Self::ExpectedMapLayer { .. } | Self::MapLayerMissingSurface => None,
        }
    }
}

impl From<SurfaceError> for ProjectionError {
    fn from(error: SurfaceError) -> Self {
        Self::Surface(error)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// 依 `LayerKind` 顺序组织的可渲染游戏视图。
pub struct GameView {
    layers: Vec<ViewLayer>,
}

impl GameView {
    /// 由已按 `LayerKind` 非递减顺序排列的图层创建视图。
    ///
    /// # Panics
    ///
    /// 图层顺序不满足该要求时 panic。
    pub fn new(layers: impl IntoIterator<Item = ViewLayer>) -> Self {
        let layers = layers.into_iter().collect::<Vec<_>>();
        assert!(layers.windows(2).all(|pair| pair[0].kind <= pair[1].kind));
        Self { layers }
    }

    /// 返回保持绘制顺序的图层。
    pub fn layers(&self) -> &[ViewLayer] {
        &self.layers
    }

    /// 按图层和图层内的原始顺序遍历图片。
    pub fn images(&self) -> impl Iterator<Item = &ViewImage> {
        self.layers.iter().flat_map(|layer| &layer.images)
    }

    /// 按图层和图层内的原始顺序遍历文本标签。
    pub fn labels(&self) -> impl Iterator<Item = &TextLabel> {
        self.layers.iter().flat_map(|layer| &layer.labels)
    }
}

/// 基座的多个状态页只投影不可变状态；所有游戏变化仍由 host 路由 intent。
pub fn project_foundation(
    content: &ThinSliceContent,
    state: &GameState,
    page: FoundationPage,
) -> Result<UiTree<FoundationPageAction>, UiBuildError> {
    let tabs = [
        foundation_tab(
            "旅程",
            "foundation-tab-journey",
            FoundationPage::Journey,
            page,
        )?,
        foundation_tab("背包", "foundation-tab-bag", FoundationPage::Bag, page)?,
        foundation_tab(
            "训练家卡片",
            "foundation-tab-trainer-card",
            FoundationPage::TrainerCard,
            page,
        )?,
    ];
    let body = match page {
        FoundationPage::Journey => foundation_journey(content, state)?,
        FoundationPage::Bag => foundation_bag(content, state)?,
        FoundationPage::TrainerCard => foundation_trainer_card(content, state)?,
    };
    UiTree::new(ui_screen(
        &FOUNDATION_THEME,
        [
            ui_panel(
                &FOUNDATION_THEME,
                PanelTone::Header,
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(48),
                    direction: FlexDirection::Row,
                    main_align: MainAlign::SpaceBetween,
                    cross_align: CrossAlign::Center,
                    padding: Insets::symmetric(12, 8),
                    ..UiStyle::default()
                },
                [
                    ui_text(
                        &FOUNDATION_THEME,
                        TextTone::Default,
                        "旅程记录",
                        22,
                        Dimension::Fill,
                    ),
                    foundation_action_button("×", "foundation-close", FoundationPageAction::Close)?,
                ],
            ),
            ui_row(
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(38),
                    gap: 4,
                    padding: Insets::symmetric(8, 4),
                    ..UiStyle::default()
                },
                tabs,
            ),
            body,
        ],
    ))
}

fn foundation_tab(
    label: &str,
    key: &str,
    target: FoundationPage,
    selected: FoundationPage,
) -> Result<UiNode<FoundationPageAction>, UiBuildError> {
    let node = ui_button(
        &FOUNDATION_THEME,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(30),
            main_align: MainAlign::Center,
            cross_align: CrossAlign::Center,
            border_radius: FOUNDATION_THEME.small_radius,
            ..UiStyle::default()
        },
        target == selected,
        [ui_text(
            &FOUNDATION_THEME,
            if target == selected {
                TextTone::Selected
            } else {
                TextTone::Default
            },
            label,
            15,
            Dimension::Fill,
        )],
    )
    .with_key(UiKey::new(key)?);
    Ok(node.with_action(FoundationPageAction::SelectPage(target)))
}

fn foundation_journey(
    content: &ThinSliceContent,
    state: &GameState,
) -> Result<UiNode<FoundationPageAction>, UiBuildError> {
    let mut party = party_rows(content, state);
    if party.is_empty() {
        party.push(ui_text(
            &FOUNDATION_THEME,
            TextTone::Muted,
            "尚未获得伙伴",
            19,
            Dimension::Fill,
        ));
    }
    let encounter = match (state.pending_encounter(), state.active_battle()) {
        (Some(position), _) => format!("草丛遭遇  {}, {}", position.x(), position.y()),
        (_, Some(battle)) => format!("战斗中  {}", battle.battle().as_str()),
        (None, None) => String::from("探索中"),
    };
    let movement_actions = [
        foundation_action_button(
            "↑",
            "foundation-move-up",
            FoundationPageAction::Move(FoundationDirection::Up),
        )?,
        foundation_action_button(
            "←",
            "foundation-move-left",
            FoundationPageAction::Move(FoundationDirection::Left),
        )?,
        foundation_action_button(
            "↓",
            "foundation-move-down",
            FoundationPageAction::Move(FoundationDirection::Down),
        )?,
        foundation_action_button(
            "→",
            "foundation-move-right",
            FoundationPageAction::Move(FoundationDirection::Right),
        )?,
    ];
    let journey_actions = [
        foundation_action_button(
            "交互",
            "foundation-interact",
            FoundationPageAction::Interact,
        )?,
        foundation_action_button(
            "遭遇",
            "foundation-encounter",
            FoundationPageAction::Encounter,
        )?,
        foundation_action_button(
            "结算",
            "foundation-resolve",
            FoundationPageAction::ResolveBattle,
        )?,
        foundation_action_button("存档", "foundation-save", FoundationPageAction::Save)?,
    ];
    Ok(ui_panel(
        &FOUNDATION_THEME,
        PanelTone::Screen,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            gap: 6,
            padding: Insets::all(8),
            ..UiStyle::default()
        },
        [
            ui_row(
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(46),
                    gap: 4,
                    ..UiStyle::default()
                },
                [
                    foundation_info_panel("地点", state.map().as_str()),
                    foundation_info_panel(
                        "坐标",
                        format!("{}, {}", state.position().x(), state.position().y()),
                    ),
                    foundation_info_panel("状态", encounter),
                ],
            ),
            ui_panel(
                &FOUNDATION_THEME,
                PanelTone::Panel,
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    gap: 4,
                    padding: Insets::all(8),
                    border_radius: FOUNDATION_THEME.medium_radius,
                    ..UiStyle::default()
                },
                std::iter::once(ui_text(
                    &FOUNDATION_THEME,
                    TextTone::Default,
                    "队伍",
                    16,
                    Dimension::Fill,
                ))
                .chain(party),
            ),
            ui_column(
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(60),
                    gap: 4,
                    ..UiStyle::default()
                },
                [
                    ui_row(
                        UiStyle {
                            width: Dimension::Fill,
                            height: Dimension::Fill,
                            gap: 4,
                            ..UiStyle::default()
                        },
                        movement_actions,
                    ),
                    ui_row(
                        UiStyle {
                            width: Dimension::Fill,
                            height: Dimension::Fill,
                            gap: 4,
                            ..UiStyle::default()
                        },
                        journey_actions,
                    ),
                ],
            ),
        ],
    ))
}

fn foundation_bag(
    content: &ThinSliceContent,
    state: &GameState,
) -> Result<UiNode<FoundationPageAction>, UiBuildError> {
    let mut entries = state
        .inventory()
        .entries()
        .iter()
        .map(|(item, quantity)| {
            let category = content
                .item(item)
                .map(|definition| format!("{:?}", definition.category()))
                .unwrap_or_else(|| String::from("未知"));
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
                        item.as_str(),
                        19,
                        Dimension::Fill,
                    ),
                    ui_text(
                        &FOUNDATION_THEME,
                        TextTone::Muted,
                        category,
                        16,
                        Dimension::Fill,
                    ),
                    ui_text(
                        &FOUNDATION_THEME,
                        TextTone::Default,
                        format!("x{quantity}"),
                        19,
                        Dimension::Fill,
                    ),
                ],
            )
        })
        .collect::<Vec<_>>();
    if entries.is_empty() {
        entries.push(ui_text(
            &FOUNDATION_THEME,
            TextTone::Muted,
            "背包为空",
            19,
            Dimension::Fill,
        ));
    }
    Ok(ui_panel(
        &FOUNDATION_THEME,
        PanelTone::Screen,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            gap: 6,
            padding: Insets::all(8),
            ..UiStyle::default()
        },
        [
            ui_row(
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(46),
                    gap: 4,
                    ..UiStyle::default()
                },
                [
                    foundation_info_panel("金钱", state.money().amount().to_string()),
                    foundation_info_panel(
                        "容量",
                        format!(
                            "{}/{}",
                            state.inventory().entries().len(),
                            state.inventory().capacity()
                        ),
                    ),
                    foundation_action_button(
                        "购买伤药",
                        "foundation-buy-potion",
                        FoundationPageAction::BuyPotion,
                    )?,
                ],
            ),
            ui_panel(
                &FOUNDATION_THEME,
                PanelTone::Panel,
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    gap: 4,
                    padding: Insets::all(8),
                    border_radius: FOUNDATION_THEME.medium_radius,
                    ..UiStyle::default()
                },
                entries,
            ),
        ],
    ))
}

fn foundation_trainer_card(
    content: &ThinSliceContent,
    state: &GameState,
) -> Result<UiNode<FoundationPageAction>, UiBuildError> {
    let experience = state
        .party()
        .iter()
        .map(|creature| creature.experience())
        .sum::<u32>();
    let lead = state.party().first().map_or_else(
        || String::from("未登记"),
        |creature| {
            content
                .creature(creature.template())
                .map(|template| template.species().to_owned())
                .unwrap_or_else(|| creature.template().as_str().to_owned())
        },
    );
    Ok(ui_panel(
        &FOUNDATION_THEME,
        PanelTone::Screen,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            gap: 6,
            padding: Insets::all(8),
            ..UiStyle::default()
        },
        [ui_panel(
            &FOUNDATION_THEME,
            PanelTone::Card,
            UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                gap: 6,
                padding: Insets::all(12),
                border_radius: FOUNDATION_THEME.large_radius,
                ..UiStyle::default()
            },
            [
                ui_text(
                    &FOUNDATION_THEME,
                    TextTone::Ink,
                    "训练家卡片",
                    22,
                    Dimension::Fill,
                ),
                ui_text(
                    &FOUNDATION_THEME,
                    TextTone::MutedInk,
                    "LOCAL PLAYER",
                    14,
                    Dimension::Fill,
                ),
                trainer_card_row("伙伴", format!("{}  ·  {lead}", state.party().len())),
                trainer_card_row("经验", experience.to_string()),
                trainer_card_row("金钱", state.money().amount().to_string()),
                trainer_card_row("训练师胜场", state.defeated_trainers().len().to_string()),
                trainer_card_row("事件记录", state.flags().len().to_string()),
            ],
        )],
    ))
}

fn foundation_info_panel(
    label: impl Into<String>,
    value: impl Into<String>,
) -> UiNode<FoundationPageAction> {
    ui_panel(
        &FOUNDATION_THEME,
        PanelTone::Panel,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            gap: 2,
            padding: Insets::all(4),
            border_radius: FOUNDATION_THEME.small_radius,
            ..UiStyle::default()
        },
        [
            ui_text(
                &FOUNDATION_THEME,
                TextTone::Muted,
                label,
                12,
                Dimension::Fill,
            ),
            ui_text(
                &FOUNDATION_THEME,
                TextTone::Default,
                value,
                15,
                Dimension::Fill,
            ),
        ],
    )
}

fn trainer_card_row(
    label: impl Into<String>,
    value: impl Into<String>,
) -> UiNode<FoundationPageAction> {
    ui_row(
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(24),
            main_align: MainAlign::SpaceBetween,
            cross_align: CrossAlign::Center,
            ..UiStyle::default()
        },
        [
            ui_text(
                &FOUNDATION_THEME,
                TextTone::MutedInk,
                label,
                14,
                Dimension::Fill,
            ),
            ui_text(&FOUNDATION_THEME, TextTone::Ink, value, 15, Dimension::Fill),
        ],
    )
}

fn party_rows(content: &ThinSliceContent, state: &GameState) -> Vec<UiNode<FoundationPageAction>> {
    state
        .party()
        .iter()
        .map(|creature| {
            let definition = content.creature(creature.template());
            let name = definition
                .map(|template| template.species())
                .unwrap_or(creature.template().as_str());
            let max_hp = definition
                .map(|template| template.max_hp())
                .unwrap_or(creature.hp());
            ui_row(
                UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(32),
                    main_align: MainAlign::SpaceBetween,
                    cross_align: CrossAlign::Center,
                    ..UiStyle::default()
                },
                [
                    ui_text(
                        &FOUNDATION_THEME,
                        TextTone::Default,
                        name,
                        15,
                        Dimension::Fill,
                    ),
                    ui_text(
                        &FOUNDATION_THEME,
                        TextTone::Muted,
                        format!("HP {}/{}", creature.hp(), max_hp),
                        14,
                        Dimension::Fill,
                    ),
                    ui_text(
                        &FOUNDATION_THEME,
                        TextTone::Muted,
                        format!("PP {}", creature.pp()),
                        14,
                        Dimension::Fill,
                    ),
                    ui_text(
                        &FOUNDATION_THEME,
                        TextTone::Default,
                        format!("EXP {}", creature.experience()),
                        14,
                        Dimension::Fill,
                    ),
                ],
            )
        })
        .collect()
}

fn foundation_action_button(
    label: &str,
    key: &str,
    action: FoundationPageAction,
) -> Result<UiNode<FoundationPageAction>, UiBuildError> {
    let node = ui_button(
        &FOUNDATION_THEME,
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            main_align: MainAlign::Center,
            cross_align: CrossAlign::Center,
            border_radius: FOUNDATION_THEME.small_radius,
            ..UiStyle::default()
        },
        false,
        [ui_text(
            &FOUNDATION_THEME,
            TextTone::Default,
            label,
            15,
            Dimension::Fill,
        )],
    )
    .with_key(UiKey::new(key)?);
    Ok(node.with_action(action))
}

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

/// 将命令控制台投影为固定游戏画布上的 `Console` 图层。
pub fn project_console(console: &CommandConsoleView) -> Result<ViewLayer, ProjectionError> {
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
    let mut surface = Surface::filled(GridSize::new(CANVAS_WIDTH, CANVAS_HEIGHT), ViewCell::Empty)?;
    surface.fill_rect(panel, sprite(PANEL_EDGE))?;
    surface.fill_rect(
        GridRect::new(
            GridPos::new((PANEL_COL + 1) as i32, (PANEL_ROW + 1) as i32),
            GridSize::new(PANEL_WIDTH - 2, PANEL_HEIGHT - 2),
        ),
        sprite(PANEL),
    )?;

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
            surface.fill_rect(
                GridRect::new(GridPos::new(2, row as i32), GridSize::new(28, 1)),
                sprite(SELECTED),
            )?;
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
    Ok(ViewLayer::new(LayerKind::Console)
        .with_surface(surface)
        .with_labels(labels))
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

/// 将战斗快照和 UI 状态投影为固定游戏画布。
/// 当 UI 位于换宝可梦页面且存在交互提示时，结果改为该页面的独立视图。
pub fn project_battle(
    snapshot: &BattleSessionSnapshot,
    ui: BattleUiState,
    sprites: BattleSpriteResources,
    sprite_frame: usize,
) -> Result<GameView, ProjectionError> {
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

    Ok(GameView::new([
        ViewLayer::new(LayerKind::Map)
            .with_surface(canvas.finish()?)
            .with_images(battlefield_images),
        ViewLayer::new(LayerKind::Character).with_images(character_images),
        ViewLayer::new(LayerKind::Hud)
            .with_images(images)
            .with_labels(labels),
    ]))
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
) -> Result<GameView, ProjectionError> {
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
    Ok(GameView::new([
        ViewLayer::new(LayerKind::Map),
        ViewLayer::new(LayerKind::Character),
        ViewLayer::new(LayerKind::Hud)
            .with_surface(canvas.finish()?)
            .with_images(images)
            .with_labels(labels),
    ]))
}

/// 以静止动画和零偏移投影世界观察结果。
pub fn project_world(observation: &WorldObservation) -> Result<GameView, ProjectionError> {
    project_world_animated(observation, WorldAnimation::Stand, 0)
}

/// 以指定角色动画投影世界观察结果。
/// 角色和地图均不施加像素偏移。
pub fn project_world_animated(
    observation: &WorldObservation,
    animation: WorldAnimation,
    sprite_frame: usize,
) -> Result<GameView, ProjectionError> {
    project_world_presented(observation, animation, sprite_frame, PixelOffset::new(0, 0))
}

/// 以指定角色动画和像素偏移投影世界观察结果。
/// 偏移只应用于角色图像，不改变地图表面。
pub fn project_world_presented(
    observation: &WorldObservation,
    animation: WorldAnimation,
    sprite_frame: usize,
    pixel_offset: PixelOffset,
) -> Result<GameView, ProjectionError> {
    let mut actors = world_actor_images(
        observation,
        animation,
        sprite_frame,
        pixel_offset,
        PixelOffset::new(0, 0),
    );
    let speech = world_speech_overlay(observation, GridPos::new(0, 0), PixelOffset::new(0, 0));
    actors.extend(speech.images);
    Ok(GameView::new([
        ViewLayer::new(LayerKind::Map).with_surface(Canvas::new(MAP_GROUND).finish()?),
        ViewLayer::new(LayerKind::Character)
            .with_images(actors)
            .with_labels(speech.labels),
        ViewLayer::new(LayerKind::Hud),
    ]))
}

/// 将已渲染地图图层与世界角色、对话和可选控制台组合成游戏视图。
/// 相机会平移并裁剪角色和对话覆盖层。
///
pub fn compose_world(
    map: ViewLayer,
    camera: GridPos,
    observation: &WorldObservation,
    animation: WorldAnimation,
    sprite_frame: usize,
    npc_pixel_offset: PixelOffset,
    console: Option<&CommandConsoleView>,
) -> Result<GameView, ProjectionError> {
    if map.kind != LayerKind::Map {
        return Err(ProjectionError::ExpectedMapLayer { actual: map.kind });
    }
    let viewport_size = map
        .surface
        .as_ref()
        .ok_or(ProjectionError::MapLayerMissingSurface)?
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
        layers.push(project_console(console)?);
    }
    Ok(GameView::new(layers))
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
        let max_row = CANVAS_HEIGHT.saturating_sub(SPEECH_BUBBLE_HEIGHT) as i32;
        if row < 0 || row > max_row || center < 0 || center >= CANVAS_WIDTH as i32 {
            continue;
        }
        let content = speech_text(speech.as_str());
        let width = (content.chars().count() as u32 * 2 + 2).clamp(10, 18);
        let max_col = CANVAS_WIDTH.saturating_sub(width) as i32;
        let col = (center - width as i32 / 2).clamp(0, max_col) as u32;
        let row = row as u32;
        images.push(
            rounded_image(col, row, width, SPEECH_BUBBLE_HEIGHT, SPEECH_BUBBLE, 100)
                .with_pixel_offset(pixel_offset),
        );
        labels.push(label(
            TextRole::Message,
            col + 1,
            row,
            width.saturating_sub(2),
            SPEECH_BUBBLE_HEIGHT,
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

/// 在已有图层后附加可选控制台，并构建游戏视图。
/// `layers` 必须已按 `LayerKind` 非递减顺序排列，且不能包含晚于 `Console` 的图层。
pub fn with_console(
    mut layers: Vec<ViewLayer>,
    console: Option<&CommandConsoleView>,
) -> Result<GameView, ProjectionError> {
    if let Some(console) = console {
        layers.push(project_console(console)?);
    }
    Ok(GameView::new(layers))
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

/// 返回角色外观、朝向和动画帧对应的资源键。
/// 行走动画按四帧循环取模，静止动画始终使用静止帧。
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
    AssetKey::from_resource_template(format!(
        "character/{}/{direction_index}/{frame_offset}",
        appearance.as_str()
    ))
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

/// 返回指定宝可梦属性的战斗图标资源键。
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
    AssetKey::from_resource_template(format!("ui/battle/type/{name}"))
}

fn move_category_icon_image(col: u32, row: u32, category: MoveCategory) -> ViewImage {
    ViewImage::new(
        GridRect::new(GridPos::new(col as i32, row as i32), GridSize::new(2, 1)),
        move_category_icon_asset(category),
        Rgba8::new(255, 255, 255, 255),
        20,
    )
}

/// 返回指定招式分类的战斗图标资源键。
pub fn move_category_icon_asset(category: MoveCategory) -> AssetKey {
    let name = match category {
        MoveCategory::Physical => "physical",
        MoveCategory::Special => "special",
        MoveCategory::Status => "status",
    };
    AssetKey::from_resource_template(format!("ui/battle/move-category/{name}"))
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// 一场战斗中双方精灵资源键的两个动画帧。
pub struct BattleSpriteResources {
    own: [AssetKey; 2],
    opponent: [AssetKey; 2],
}

impl BattleSpriteResources {
    /// 为双方队伍槽位生成两个动画帧的精灵资源键。
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

/// 返回玩家后视精灵的资源键。
/// `frame` 按两个动画帧循环取模。
pub fn player_back_asset(slot: usize, frame: usize) -> AssetKey {
    AssetKey::from_resource_template(format!("battle/player/{slot}/back/{}", frame % 2))
}

/// 返回对手正视精灵的资源键。
/// `frame` 按两个动画帧循环取模。
pub fn opponent_front_asset(slot: usize, frame: usize) -> AssetKey {
    AssetKey::from_resource_template(format!("battle/opponent/{slot}/front/{}", frame % 2))
}

/// 返回队伍宝可梦图标的资源键。
/// `frame` 按两个动画帧循环取模。
pub fn pokemon_icon_asset(slot: usize, frame: usize) -> AssetKey {
    AssetKey::from_resource_template(format!("battle/team/{slot}/icon/{}", frame % 2))
}

/// 返回圆角矩形 UI 资源键。
pub fn rounded_ui_asset() -> AssetKey {
    AssetKey::from_resource_template("ui/rounded-rect".into())
}

/// 返回胶囊形 UI 资源键。
pub fn pill_ui_asset() -> AssetKey {
    AssetKey::from_resource_template("ui/pill".into())
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

    fn finish(self) -> Result<Surface<ViewCell>, SurfaceError> {
        Surface::from_cells(GridSize::new(CANVAS_WIDTH, CANVAS_HEIGHT), self.cells)
    }
}

const fn sprite(tint: Rgba8) -> ViewCell {
    ViewCell::Fill(tint)
}

#[cfg(test)]
#[path = "../tests/unit/projection.rs"]
mod tests;
