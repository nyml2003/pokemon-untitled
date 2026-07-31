//! 游戏视图的公共模型、主题和固定画布常量。

use std::{error::Error, fmt};

use battle_session::Participant;
use game_assets::AssetKey;
use game_foundation::Direction as FoundationDirection;
use game_ui_kit::{GameButtonTheme, GameUiTheme};
use punctum_gpu::{PixelOffset, Rgba8};
use punctum_grid::{GridRect, Surface, SurfaceError};
use punctum_ui::UiColor;
pub const CANVAS_WIDTH: u32 = 32;
pub const CANVAS_HEIGHT: u32 = 24;
pub(crate) const SPEECH_BUBBLE_HEIGHT: u32 = 2;

pub(crate) const SKY: Rgba8 = Rgba8::new(146, 211, 218, 255);
pub(crate) const SKY_DEEP: Rgba8 = Rgba8::new(102, 177, 184, 255);
pub(crate) const DISTANT_GRASS: Rgba8 = Rgba8::new(75, 143, 105, 255);
pub(crate) const GROUND: Rgba8 = Rgba8::new(54, 105, 76, 255);
pub(crate) const GROUND_DARK: Rgba8 = Rgba8::new(37, 78, 62, 255);
pub(crate) const PLATFORM: Rgba8 = Rgba8::new(174, 201, 145, 255);
pub(crate) const PLATFORM_SHADOW: Rgba8 = Rgba8::new(45, 82, 64, 150);
pub(crate) const PANEL: Rgba8 = Rgba8::new(28, 34, 45, 248);
pub(crate) const PANEL_EDGE: Rgba8 = Rgba8::new(218, 225, 214, 255);
pub(crate) const SELECTED: Rgba8 = Rgba8::new(73, 211, 168, 255);
pub(crate) const SELECTED_DARK: Rgba8 = Rgba8::new(29, 70, 67, 255);
pub(crate) const BATTLE_CARD: Rgba8 = Rgba8::new(242, 246, 239, 255);
pub(crate) const BATTLE_CARD_SHADOW: Rgba8 = Rgba8::new(24, 37, 45, 190);
pub(crate) const BATTLE_INK: Rgba8 = Rgba8::new(26, 39, 45, 255);
pub(crate) const BATTLE_MUTED: Rgba8 = Rgba8::new(82, 96, 98, 255);
pub(crate) const OPPONENT_ACCENT: Rgba8 = Rgba8::new(241, 112, 116, 255);
pub(crate) const PLAYER_ACCENT: Rgba8 = Rgba8::new(57, 190, 151, 255);
pub(crate) const ACTION_PANEL: Rgba8 = Rgba8::new(19, 25, 34, 255);
pub(crate) const ACTION_PANEL_ALT: Rgba8 = Rgba8::new(30, 38, 49, 255);
pub(crate) const ACTION_BORDER: Rgba8 = Rgba8::new(83, 98, 112, 255);
pub(crate) const PARTY_BG: Rgba8 = Rgba8::new(13, 18, 27, 255);
pub(crate) const PARTY_PANEL: Rgba8 = Rgba8::new(25, 33, 44, 255);
pub(crate) const PARTY_PANEL_ALT: Rgba8 = Rgba8::new(34, 44, 57, 255);
pub(crate) const PARTY_EDGE: Rgba8 = Rgba8::new(73, 89, 105, 255);
pub(crate) const HP_GOOD: Rgba8 = Rgba8::new(74, 190, 102, 255);
pub(crate) const HP_MID: Rgba8 = Rgba8::new(226, 177, 66, 255);
pub(crate) const HP_LOW: Rgba8 = Rgba8::new(224, 91, 72, 255);
pub(crate) const HP_TRACK_EDGE: Rgba8 = Rgba8::new(38, 46, 55, 255);
pub(crate) const HP_GOOD_GLOW: Rgba8 = Rgba8::new(119, 231, 142, 255);
pub(crate) const HP_MID_GLOW: Rgba8 = Rgba8::new(255, 214, 101, 255);
pub(crate) const HP_LOW_GLOW: Rgba8 = Rgba8::new(255, 133, 111, 255);
pub(crate) const TEXT: Rgba8 = Rgba8::new(244, 246, 239, 255);
pub(crate) const MUTED_TEXT: Rgba8 = Rgba8::new(182, 194, 194, 255);
pub(crate) const CONSOLE_ERROR: Rgba8 = Rgba8::new(255, 142, 126, 255);
pub(crate) const MAP_GROUND: Rgba8 = Rgba8::new(138, 187, 116, 255);
pub(crate) const SPEECH_BUBBLE: Rgba8 = Rgba8::new(83, 89, 96, 236);

pub(crate) const POKEDEX_THEME: GameUiTheme = GameUiTheme {
    screen: UiColor::new(13, 21, 29, 255),
    header: UiColor::new(21, 47, 60, 255),
    panel: UiColor::new(31, 52, 64, 255),
    selected: UiColor::new(29, 70, 67, 255),
    selected_text: UiColor::new(73, 211, 168, 255),
    card: UiColor::new(237, 242, 233, 255),
    modal_scrim: UiColor::new(5, 12, 18, 144),
    modal_border: UiColor::new(123, 160, 159, 255),
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
    button: GameButtonTheme {
        hover_color: UiColor::new(73, 211, 168, 42),
        pressed_color: UiColor::new(73, 211, 168, 92),
        disabled_color: UiColor::new(7, 13, 18, 128),
        focus_color: UiColor::new(73, 211, 168, 220),
        ripple_color: UiColor::new(154, 255, 219, 150),
        focus_width: 1,
        ripple_duration_ms: 160,
    },
};

pub(crate) const BATTLE_THEME: GameUiTheme = GameUiTheme {
    screen: UiColor::new(146, 211, 218, 255),
    header: UiColor::new(19, 25, 34, 255),
    panel: UiColor::new(30, 38, 49, 255),
    selected: UiColor::new(73, 211, 168, 255),
    selected_text: UiColor::new(26, 39, 45, 255),
    card: UiColor::new(242, 246, 239, 255),
    modal_scrim: UiColor::new(5, 10, 16, 144),
    modal_border: UiColor::new(162, 189, 191, 255),
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
    button: GameButtonTheme {
        hover_color: UiColor::new(255, 255, 255, 38),
        pressed_color: UiColor::new(255, 255, 255, 86),
        disabled_color: UiColor::new(8, 12, 17, 128),
        focus_color: UiColor::new(73, 211, 168, 220),
        ripple_color: UiColor::new(255, 255, 255, 130),
        focus_width: 1,
        ripple_duration_ms: 160,
    },
};

pub(crate) const FOUNDATION_THEME: GameUiTheme = GameUiTheme {
    screen: UiColor::new(9, 16, 29, 255),
    header: UiColor::new(22, 57, 94, 255),
    panel: UiColor::new(18, 34, 55, 255),
    selected: UiColor::new(211, 48, 55, 255),
    selected_text: UiColor::new(255, 248, 231, 255),
    card: UiColor::new(243, 232, 195, 255),
    modal_scrim: UiColor::new(4, 8, 16, 144),
    modal_border: UiColor::new(199, 178, 125, 255),
    image_backdrop: UiColor::new(45, 93, 119, 255),
    text: UiColor::new(255, 248, 231, 255),
    muted_text: UiColor::new(165, 184, 207, 255),
    ink: UiColor::new(24, 35, 52, 255),
    muted_ink: UiColor::new(78, 91, 112, 255),
    small_spacing: 6,
    medium_spacing: 12,
    large_spacing: 20,
    small_radius: punctum_ui::UiBorderRadius::all(4),
    medium_radius: punctum_ui::UiBorderRadius::all(6),
    large_radius: punctum_ui::UiBorderRadius::all(8),
    body_text_size: 16,
    title_text_size: 24,
    button: GameButtonTheme {
        hover_color: UiColor::new(255, 210, 120, 38),
        pressed_color: UiColor::new(255, 210, 120, 88),
        disabled_color: UiColor::new(5, 10, 18, 140),
        focus_color: UiColor::new(255, 210, 120, 230),
        ripple_color: UiColor::new(255, 236, 175, 150),
        focus_width: 1,
        ripple_duration_ms: 160,
    },
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
