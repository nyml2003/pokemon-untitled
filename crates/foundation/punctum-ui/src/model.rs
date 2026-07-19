use crate::UiBuildError;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiSize {
    pub width: u32,
    pub height: u32,
}
impl UiSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
impl UiRect {
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
    pub const fn size(self) -> UiSize {
        UiSize::new(self.width, self.height)
    }
    pub const fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }
    pub fn intersect(self, other: Self) -> Option<Self> {
        let left = self.x.max(other.x);
        let top = self.y.max(other.y);
        let right = self
            .x
            .saturating_add(self.width)
            .min(other.x.saturating_add(other.width));
        let bottom = self
            .y
            .saturating_add(self.height)
            .min(other.y.saturating_add(other.height));
        (left < right && top < bottom).then_some(Self::new(left, top, right - left, bottom - top))
    }
    pub fn contains(self, x: u32, y: u32) -> bool {
        x >= self.x
            && y >= self.y
            && x < self.x.saturating_add(self.width)
            && y < self.y.saturating_add(self.height)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiPixelOffset {
    pub x: i32,
    pub y: i32,
}
impl UiPixelOffset {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}
impl UiColor {
    pub const fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UiId(pub u32);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UiContentId(String);
impl UiContentId {
    /// 以非空资源标识创建内容 ID。
    pub fn new(value: impl Into<String>) -> Result<Self, UiBuildError> {
        let value = value.into();
        if value.is_empty() {
            Err(UiBuildError::EmptyContentId)
        } else {
            Ok(Self(value))
        }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Builds a content ID from an already-validated resource key.
    pub fn from_resource_key(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

/// 在纯树重建过程中保持稳定的动态节点语义标识。
/// 静态节点不应需要 `UiKey`。
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UiKey(String);
impl UiKey {
    /// 以非空字符串创建稳定节点 key。
    pub fn new(value: impl Into<String>) -> Result<Self, UiBuildError> {
        let value = value.into();
        if value.is_empty() {
            Err(UiBuildError::EmptyKey)
        } else {
            Ok(Self(value))
        }
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Insets {
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}

impl Insets {
    pub const fn horizontal(self) -> u32 {
        self.left.saturating_add(self.right)
    }
    pub const fn vertical(self) -> u32 {
        self.top.saturating_add(self.bottom)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiBorder {
    pub widths: Insets,
    pub color: UiColor,
}
impl UiBorder {
    pub const fn is_visible(self) -> bool {
        self.widths.left != 0
            || self.widths.top != 0
            || self.widths.right != 0
            || self.widths.bottom != 0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UiBorderRadius {
    pub top_left: u32,
    pub top_right: u32,
    pub bottom_right: u32,
    pub bottom_left: u32,
}
impl UiBorderRadius {
    pub const fn all(radius: u32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }
    pub const fn is_zero(self) -> bool {
        self.top_left == 0 && self.top_right == 0 && self.bottom_right == 0 && self.bottom_left == 0
    }
    pub fn clamped(self, bounds: UiRect) -> Self {
        let maximum = bounds.width.min(bounds.height) / 2;
        Self {
            top_left: self.top_left.min(maximum),
            top_right: self.top_right.min(maximum),
            bottom_right: self.bottom_right.min(maximum),
            bottom_left: self.bottom_left.min(maximum),
        }
    }
    /// 返回边框占用外框后得到的内角半径。
    /// 圆形 shader 遮罩在每个角使用相邻边中较大的宽度。
    pub fn inset(self, border: Insets) -> Self {
        Self {
            top_left: self.top_left.saturating_sub(border.left.max(border.top)),
            top_right: self.top_right.saturating_sub(border.right.max(border.top)),
            bottom_right: self
                .bottom_right
                .saturating_sub(border.right.max(border.bottom)),
            bottom_left: self
                .bottom_left
                .saturating_sub(border.left.max(border.bottom)),
        }
    }
}
impl Insets {
    pub const fn all(value: u32) -> Self {
        Self {
            left: value,
            top: value,
            right: value,
            bottom: value,
        }
    }
    pub const fn symmetric(horizontal: u32, vertical: u32) -> Self {
        Self {
            left: horizontal,
            top: vertical,
            right: horizontal,
            bottom: vertical,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Dimension {
    #[default]
    Auto,
    Px(u32),
    /// 容器内容框的比例，表示为 `units / base`。
    Ratio {
        units: u32,
        base: u32,
    },
    Fill,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlexDirection {
    Row,
    #[default]
    Column,
    Stack,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MainAlign {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CrossAlign {
    #[default]
    Start,
    Center,
    End,
    Stretch,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Position {
    #[default]
    Flow,
    Absolute {
        left: u32,
        top: u32,
    },
    /// 在逻辑画布中的绝对位置。
    /// 此处刻意使用 `UiSize` 而非 Grid 类型，以保持 UI 与渲染器无关。
    AbsoluteRatio {
        left: u32,
        top: u32,
        base: UiSize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiTextSize {
    Px(u32),
    Ratio {
        units: u32,
        base: u32,
        minimum: u32,
        maximum: u32,
    },
}
impl UiTextSize {
    pub(crate) fn resolve(self, basis: u32) -> u32 {
        match self {
            Self::Px(size) => size,
            Self::Ratio {
                units,
                base,
                minimum,
                maximum,
            } => (basis.saturating_mul(units) / base).clamp(minimum, maximum),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiStyle {
    pub width: Dimension,
    pub height: Dimension,
    pub min_size: UiSize,
    pub max_size: Option<UiSize>,
    /// 将此节点适配到整数缩放且居中的逻辑画布。
    pub logical_canvas: Option<UiSize>,
    pub margin: Insets,
    pub border: UiBorder,
    pub padding: Insets,
    pub border_radius: UiBorderRadius,
    pub gap: u32,
    pub direction: FlexDirection,
    pub main_align: MainAlign,
    pub cross_align: CrossAlign,
    pub position: Position,
    pub clip: bool,
    #[deprecated(note = "可触发节点请使用 UiNode::with_action。")]
    pub interactive: bool,
}
#[allow(deprecated)]
impl Default for UiStyle {
    fn default() -> Self {
        Self {
            width: Dimension::Auto,
            height: Dimension::Auto,
            min_size: UiSize::default(),
            max_size: None,
            logical_canvas: None,
            margin: Insets::default(),
            border: UiBorder::default(),
            padding: Insets::default(),
            border_radius: UiBorderRadius::default(),
            gap: 0,
            direction: FlexDirection::Column,
            main_align: MainAlign::Start,
            cross_align: CrossAlign::Start,
            position: Position::Flow,
            clip: false,
            interactive: false,
        }
    }
}
impl UiStyle {
    pub fn fixed(width: u32, height: u32) -> Self {
        Self {
            width: Dimension::Px(width),
            height: Dimension::Px(height),
            ..Self::default()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiContent {
    Empty,
    Fill(UiColor),
    Image(UiContentId),
    ImageTinted {
        content: UiContentId,
        tint: UiColor,
    },
    ImageStyled {
        content: UiContentId,
        tint: UiColor,
        pixel_offset: UiPixelOffset,
    },
    Text {
        content: String,
        color: UiColor,
        font_size: u32,
    },
    TextScaled {
        content: String,
        color: UiColor,
        font_size: UiTextSize,
    },
}
