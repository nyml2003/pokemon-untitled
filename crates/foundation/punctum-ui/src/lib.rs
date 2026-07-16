//! Pure pixel UI tree, restricted Flex layout, paint commands, and hit regions.

#![forbid(unsafe_code)]

use std::{collections::BTreeSet, error::Error, fmt};

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
    /// Returns the inner corner radii after a border consumes the outer box.
    /// A circular shader mask uses the larger adjacent edge at each corner.
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
    /// A fraction of the containing content box, represented as `units / base`.
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
    /// Absolute placement in a logical canvas. This is deliberately expressed
    /// with `UiSize`, rather than a Grid type, so UI remains renderer-neutral.
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
    fn resolve(self, basis: u32) -> u32 {
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
    /// Fits this node to an integer-scaled, centered logical canvas.
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
    pub interactive: bool,
}
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiNode {
    pub id: UiId,
    pub style: UiStyle,
    pub content: UiContent,
    pub children: Vec<UiNode>,
}
impl UiNode {
    pub fn new(id: UiId) -> Self {
        Self {
            id,
            style: UiStyle::default(),
            content: UiContent::Empty,
            children: Vec::new(),
        }
    }
    pub fn with_style(mut self, style: UiStyle) -> Self {
        self.style = style;
        self
    }
    pub fn with_content(mut self, content: UiContent) -> Self {
        self.content = content;
        self
    }
    pub fn with_children(mut self, children: impl IntoIterator<Item = UiNode>) -> Self {
        self.children = children.into_iter().collect();
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiTree {
    root: UiNode,
}
impl UiTree {
    pub fn new(root: UiNode) -> Result<Self, UiBuildError> {
        let mut ids = BTreeSet::new();
        validate_node(&root, &mut ids)?;
        Ok(Self { root })
    }
    pub fn root(&self) -> &UiNode {
        &self.root
    }
    pub fn resolve(&self, viewport: UiSize) -> Result<UiFrame, UiLayoutError> {
        resolve_tree(&self.root, viewport)
    }
}
fn validate_node(node: &UiNode, ids: &mut BTreeSet<UiId>) -> Result<(), UiBuildError> {
    if !ids.insert(node.id) {
        return Err(UiBuildError::DuplicateId(node.id));
    }
    validate_style(node.id, node.style)?;
    validate_content(node.id, &node.content)?;
    for child in &node.children {
        validate_node(child, ids)?;
    }
    Ok(())
}

fn validate_content(id: UiId, content: &UiContent) -> Result<(), UiBuildError> {
    if let UiContent::TextScaled {
        font_size: UiTextSize::Ratio { base: 0, .. },
        ..
    } = content
    {
        return Err(UiBuildError::ZeroTextSizeBase(id));
    }
    Ok(())
}

fn validate_style(id: UiId, style: UiStyle) -> Result<(), UiBuildError> {
    for dimension in [style.width, style.height] {
        if let Dimension::Ratio { base: 0, .. } = dimension {
            return Err(UiBuildError::ZeroRatioBase(id));
        }
    }
    if let Position::AbsoluteRatio { base, .. } = style.position
        && (base.width == 0 || base.height == 0)
    {
        return Err(UiBuildError::ZeroRatioBase(id));
    }
    if let Some(canvas) = style.logical_canvas
        && (canvas.width == 0 || canvas.height == 0)
    {
        return Err(UiBuildError::ZeroLogicalCanvas(id));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiDrawCommand {
    Fill {
        bounds: UiRect,
        color: UiColor,
        border_radius: UiBorderRadius,
        clip: UiRect,
    },
    Image {
        bounds: UiRect,
        content: UiContentId,
        tint: UiColor,
        pixel_offset: UiPixelOffset,
        border_radius: UiBorderRadius,
        clip: UiRect,
    },
    Text {
        bounds: UiRect,
        content: String,
        color: UiColor,
        font_size: u32,
        clip: UiRect,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiHitRegion {
    pub id: UiId,
    pub bounds: UiRect,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiFrame {
    viewport: UiSize,
    commands: Vec<UiDrawCommand>,
    hits: Vec<UiHitRegion>,
}
impl UiFrame {
    pub const fn viewport(&self) -> UiSize {
        self.viewport
    }
    pub fn commands(&self) -> &[UiDrawCommand] {
        &self.commands
    }
    pub fn hit_regions(&self) -> &[UiHitRegion] {
        &self.hits
    }
    pub fn hit_test(&self, x: u32, y: u32) -> Option<UiId> {
        self.hits
            .iter()
            .rev()
            .find(|region| region.bounds.contains(x, y))
            .map(|region| region.id)
    }
}

fn resolve_tree(root: &UiNode, viewport: UiSize) -> Result<UiFrame, UiLayoutError> {
    let root_bounds = UiRect::new(0, 0, viewport.width, viewport.height);
    let mut commands = Vec::new();
    let mut hits = Vec::new();
    resolve_node(
        root,
        root_bounds,
        viewport,
        root_bounds,
        &mut commands,
        &mut hits,
    )?;
    Ok(UiFrame {
        viewport,
        commands,
        hits,
    })
}
fn resolve_node(
    node: &UiNode,
    offered: UiRect,
    ratio_basis: UiSize,
    inherited_clip: UiRect,
    commands: &mut Vec<UiDrawCommand>,
    hits: &mut Vec<UiHitRegion>,
) -> Result<(), UiLayoutError> {
    let bounds = constrain(node, offered, ratio_basis)?;
    let clip = if node.style.clip {
        inherited_clip
            .intersect(bounds)
            .unwrap_or(UiRect::default())
    } else {
        inherited_clip
    };
    if bounds.is_empty() || clip.is_empty() {
        return Ok(());
    }
    let radius = node.style.border_radius.clamped(bounds);
    if node.style.border.is_visible() {
        commands.push(UiDrawCommand::Fill {
            bounds,
            color: node.style.border.color,
            border_radius: radius,
            clip,
        });
    }
    let paint_bounds = inset(bounds, node.style.border.widths);
    let content_radius = radius.inset(node.style.border.widths).clamped(paint_bounds);
    match &node.content {
        UiContent::Empty => {}
        UiContent::Fill(color) => commands.push(UiDrawCommand::Fill {
            bounds: paint_bounds,
            color: *color,
            border_radius: content_radius,
            clip,
        }),
        UiContent::Image(content) => commands.push(UiDrawCommand::Image {
            bounds: paint_bounds,
            content: content.clone(),
            tint: UiColor::new(255, 255, 255, 255),
            pixel_offset: UiPixelOffset::default(),
            border_radius: content_radius,
            clip,
        }),
        UiContent::ImageTinted { content, tint } => commands.push(UiDrawCommand::Image {
            bounds: paint_bounds,
            content: content.clone(),
            tint: *tint,
            pixel_offset: UiPixelOffset::default(),
            border_radius: content_radius,
            clip,
        }),
        UiContent::ImageStyled {
            content,
            tint,
            pixel_offset,
        } => commands.push(UiDrawCommand::Image {
            bounds: paint_bounds,
            content: content.clone(),
            tint: *tint,
            pixel_offset: *pixel_offset,
            border_radius: radius.clamped(paint_bounds),
            clip,
        }),
        UiContent::Text {
            content,
            color,
            font_size,
        } => commands.push(UiDrawCommand::Text {
            bounds: paint_bounds,
            content: content.clone(),
            color: *color,
            font_size: *font_size,
            clip,
        }),
        UiContent::TextScaled {
            content,
            color,
            font_size,
        } => commands.push(UiDrawCommand::Text {
            bounds: paint_bounds,
            content: content.clone(),
            color: *color,
            font_size: font_size.resolve(ratio_basis.height),
            clip,
        }),
    }
    if node.style.interactive {
        hits.push(UiHitRegion {
            id: node.id,
            bounds: bounds.intersect(clip).unwrap_or_default(),
        });
    }
    layout_children(
        node,
        inset(paint_bounds, node.style.padding),
        clip,
        commands,
        hits,
    )
}

fn inset(bounds: UiRect, insets: Insets) -> UiRect {
    UiRect::new(
        bounds.x.saturating_add(insets.left),
        bounds.y.saturating_add(insets.top),
        bounds.width.saturating_sub(insets.horizontal()),
        bounds.height.saturating_sub(insets.vertical()),
    )
}
fn constrain(node: &UiNode, offered: UiRect, ratio_basis: UiSize) -> Result<UiRect, UiLayoutError> {
    let intrinsic = intrinsic_size(node, ratio_basis);
    let width = dimension(node.style.width, ratio_basis.width, intrinsic.width);
    let height = dimension(node.style.height, ratio_basis.height, intrinsic.height);
    let width = width.max(node.style.min_size.width);
    let height = height.max(node.style.min_size.height);
    let (width, height) = match node.style.max_size {
        Some(max) => (width.min(max.width), height.min(max.height)),
        None => (width, height),
    };
    let width = width.min(offered.width);
    let height = height.min(offered.height);
    if let Some(canvas) = node.style.logical_canvas {
        let scale = (offered.width / canvas.width).min(offered.height / canvas.height);
        let width = canvas.width.saturating_mul(scale);
        let height = canvas.height.saturating_mul(scale);
        return Ok(UiRect::new(
            offered
                .x
                .saturating_add(offered.width.saturating_sub(width) / 2),
            offered
                .y
                .saturating_add(offered.height.saturating_sub(height) / 2),
            width,
            height,
        ));
    }
    Ok(UiRect::new(offered.x, offered.y, width, height))
}
fn intrinsic_size(node: &UiNode, ratio_basis: UiSize) -> UiSize {
    match &node.content {
        UiContent::Text {
            content, font_size, ..
        } => UiSize::new(
            (content.chars().count() as u32).saturating_mul((*font_size).max(1) / 2 + 1),
            font_size.saturating_add(4),
        ),
        UiContent::TextScaled {
            content, font_size, ..
        } => {
            let font_size = font_size.resolve(ratio_basis.height);
            UiSize::new(
                (content.chars().count() as u32).saturating_mul(font_size.max(1) / 2 + 1),
                font_size.saturating_add(4),
            )
        }
        UiContent::Image(_) | UiContent::ImageTinted { .. } | UiContent::ImageStyled { .. } => {
            UiSize::new(1, 1)
        }
        _ => UiSize::default(),
    }
}
fn dimension(dimension: Dimension, offered: u32, intrinsic: u32) -> u32 {
    match dimension {
        Dimension::Auto => intrinsic,
        Dimension::Px(value) => value,
        Dimension::Ratio { units, base } => offered.saturating_mul(units) / base,
        Dimension::Fill => offered,
    }
}
fn layout_children(
    node: &UiNode,
    content: UiRect,
    clip: UiRect,
    commands: &mut Vec<UiDrawCommand>,
    hits: &mut Vec<UiHitRegion>,
) -> Result<(), UiLayoutError> {
    let flow: Vec<_> = node
        .children
        .iter()
        .filter(|child| matches!(child.style.position, Position::Flow))
        .collect();
    let horizontal = matches!(node.style.direction, FlexDirection::Row);
    let stacked = matches!(node.style.direction, FlexDirection::Stack);
    let main_available = if horizontal {
        content.width
    } else {
        content.height
    };
    let cross_available = if horizontal {
        content.height
    } else {
        content.width
    };
    let gap_total = node
        .style
        .gap
        .saturating_mul(flow.len().saturating_sub(1) as u32);
    let fixed: u32 = flow
        .iter()
        .map(|child| {
            let size = intrinsic_size(child, content.size());
            let main_dimension = if horizontal {
                child.style.width
            } else {
                child.style.height
            };
            match main_dimension {
                Dimension::Px(value) => {
                    value.saturating_add(main_margin(child.style.margin, horizontal))
                }
                Dimension::Auto => {
                    if horizontal {
                        size.width
                            .saturating_add(main_margin(child.style.margin, horizontal))
                    } else {
                        size.height
                            .saturating_add(main_margin(child.style.margin, horizontal))
                    }
                }
                Dimension::Ratio { .. } => dimension(main_dimension, main_available, 0)
                    .saturating_add(main_margin(child.style.margin, horizontal)),
                Dimension::Fill => 0,
            }
        })
        .sum();
    let required_minimum = flow
        .iter()
        .map(|child| {
            if horizontal {
                child.style.min_size.width
            } else {
                child.style.min_size.height
            }
        })
        .sum::<u32>()
        .saturating_add(gap_total);
    if required_minimum > main_available && !stacked {
        return Err(UiLayoutError::InsufficientSpace { id: node.id });
    }
    let fills = flow
        .iter()
        .filter(|child| {
            matches!(
                if horizontal {
                    child.style.width
                } else {
                    child.style.height
                },
                Dimension::Fill
            )
        })
        .count() as u32;
    let remaining = main_available.saturating_sub(fixed.saturating_add(gap_total));
    let fill = if fills == 0 { 0 } else { remaining / fills };
    let extra = if fills == 0 {
        remaining
    } else {
        remaining % fills
    };
    let used = fixed
        .saturating_add(gap_total)
        .saturating_add(fill.saturating_mul(fills));
    let start = match node.style.main_align {
        MainAlign::Start | MainAlign::SpaceBetween => 0,
        MainAlign::Center => extra / 2,
        MainAlign::End => extra,
    };
    let distributed_gap =
        if matches!(node.style.main_align, MainAlign::SpaceBetween) && flow.len() > 1 {
            node.style
                .gap
                .saturating_add(extra / (flow.len() as u32 - 1))
        } else {
            node.style.gap
        };
    let mut cursor = start;
    for child in flow {
        let intrinsic = intrinsic_size(child, content.size());
        let margin_before = if horizontal {
            child.style.margin.left
        } else {
            child.style.margin.top
        };
        let margin_after = if horizontal {
            child.style.margin.right
        } else {
            child.style.margin.bottom
        };
        cursor = cursor.saturating_add(margin_before);
        let main = match if horizontal {
            child.style.width
        } else {
            child.style.height
        } {
            Dimension::Px(value) => value,
            Dimension::Auto => {
                if horizontal {
                    intrinsic.width
                } else {
                    intrinsic.height
                }
            }
            Dimension::Ratio { .. } => dimension(
                if horizontal {
                    child.style.width
                } else {
                    child.style.height
                },
                main_available,
                0,
            ),
            Dimension::Fill => fill,
        };
        let cross_dimension = if horizontal {
            child.style.height
        } else {
            child.style.width
        };
        let cross_margin = cross_margin(child.style.margin, horizontal);
        let mut cross = dimension(
            cross_dimension,
            cross_available.saturating_sub(cross_margin),
            if horizontal {
                intrinsic.height
            } else {
                intrinsic.width
            },
        );
        if matches!(node.style.cross_align, CrossAlign::Stretch)
            && matches!(cross_dimension, Dimension::Auto | Dimension::Fill)
        {
            cross = cross_available.saturating_sub(cross_margin);
        }
        let cross_offset = match node.style.cross_align {
            CrossAlign::Start | CrossAlign::Stretch => 0,
            CrossAlign::Center => {
                cross_available.saturating_sub(cross.saturating_add(cross_margin)) / 2
            }
            CrossAlign::End => cross_available.saturating_sub(cross.saturating_add(cross_margin)),
        };
        let offered = if stacked {
            content
        } else if horizontal {
            UiRect::new(
                content.x.saturating_add(cursor),
                content
                    .y
                    .saturating_add(cross_offset)
                    .saturating_add(child.style.margin.top),
                main,
                cross,
            )
        } else {
            UiRect::new(
                content
                    .x
                    .saturating_add(cross_offset)
                    .saturating_add(child.style.margin.left),
                content.y.saturating_add(cursor),
                cross,
                main,
            )
        };
        let offered = if horizontal {
            UiRect::new(
                offered.x,
                offered.y,
                offered.width.min(main_available.saturating_sub(cursor)),
                offered.height,
            )
        } else {
            UiRect::new(
                offered.x,
                offered.y,
                offered.width,
                offered.height.min(main_available.saturating_sub(cursor)),
            )
        };
        resolve_node(child, offered, content.size(), clip, commands, hits)?;
        if !stacked {
            cursor = cursor
                .saturating_add(main)
                .saturating_add(margin_after)
                .saturating_add(distributed_gap);
        }
    }
    for child in node.children.iter().filter(|child| {
        matches!(
            child.style.position,
            Position::Absolute { .. } | Position::AbsoluteRatio { .. }
        )
    }) {
        let (left, top) = match child.style.position {
            Position::Absolute { left, top } => (left, top),
            Position::AbsoluteRatio { left, top, base } => (
                content.width.saturating_mul(left) / base.width,
                content.height.saturating_mul(top) / base.height,
            ),
            Position::Flow => unreachable!(),
        };
        let offered = UiRect::new(
            content.x.saturating_add(left),
            content.y.saturating_add(top),
            content.width.saturating_sub(left),
            content.height.saturating_sub(top),
        );
        resolve_node(child, offered, content.size(), clip, commands, hits)?;
    }
    let _ = used;
    Ok(())
}

const fn main_margin(margin: Insets, horizontal: bool) -> u32 {
    if horizontal {
        margin.horizontal()
    } else {
        margin.vertical()
    }
}
const fn cross_margin(margin: Insets, horizontal: bool) -> u32 {
    if horizontal {
        margin.vertical()
    } else {
        margin.horizontal()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiBuildError {
    EmptyContentId,
    DuplicateId(UiId),
    ZeroRatioBase(UiId),
    ZeroLogicalCanvas(UiId),
    ZeroTextSizeBase(UiId),
}
impl fmt::Display for UiBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyContentId => f.write_str("UI content id must not be empty"),
            Self::DuplicateId(id) => write!(f, "UI node id {:?} is duplicated", id),
            Self::ZeroRatioBase(id) => write!(f, "UI node {:?} has a zero ratio base", id),
            Self::ZeroLogicalCanvas(id) => {
                write!(f, "UI node {:?} has a zero logical canvas", id)
            }
            Self::ZeroTextSizeBase(id) => {
                write!(f, "UI node {:?} has a zero text-size base", id)
            }
        }
    }
}
impl Error for UiBuildError {}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiLayoutError {
    InsufficientSpace { id: UiId },
}
impl fmt::Display for UiLayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InsufficientSpace { id } => {
                write!(f, "UI node {:?} does not fit its container", id)
            }
        }
    }
}
impl Error for UiLayoutError {}

#[cfg(test)]
mod tests {
    use super::*;
    fn fill(id: u32, style: UiStyle) -> UiNode {
        UiNode::new(UiId(id))
            .with_style(style)
            .with_content(UiContent::Fill(UiColor::new(1, 2, 3, 255)))
    }
    #[test]
    fn row_allocates_fill_and_aligns_children() {
        let tree = UiTree::new(
            UiNode::new(UiId(1))
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    direction: FlexDirection::Row,
                    gap: 10,
                    ..UiStyle::default()
                })
                .with_children([
                    fill(2, UiStyle::fixed(20, 20)),
                    fill(
                        3,
                        UiStyle {
                            width: Dimension::Fill,
                            height: Dimension::Px(20),
                            ..UiStyle::default()
                        },
                    ),
                ]),
        )
        .unwrap();
        let frame = tree.resolve(UiSize::new(100, 40)).unwrap();
        match &frame.commands()[1] {
            UiDrawCommand::Fill { bounds, .. } => assert_eq!(*bounds, UiRect::new(30, 0, 70, 20)),
            _ => panic!(),
        }
    }
    #[test]
    fn clipping_and_topmost_hit_are_deterministic() {
        let interactive = UiStyle {
            width: Dimension::Px(30),
            height: Dimension::Px(30),
            position: Position::Absolute { left: 10, top: 10 },
            interactive: true,
            ..UiStyle::default()
        };
        let tree = UiTree::new(
            UiNode::new(UiId(1))
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    clip: true,
                    direction: FlexDirection::Stack,
                    ..UiStyle::default()
                })
                .with_children([
                    fill(2, interactive),
                    fill(
                        3,
                        UiStyle {
                            interactive: true,
                            ..interactive
                        },
                    ),
                ]),
        )
        .unwrap();
        let frame = tree.resolve(UiSize::new(20, 20)).unwrap();
        assert_eq!(frame.hit_test(15, 15), Some(UiId(3)));
        assert_eq!(frame.hit_test(25, 15), None);
    }
    #[test]
    fn logical_canvas_coordinates_resolve_without_grid_types() {
        let tree = UiTree::new(
            UiNode::new(UiId(1))
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    direction: FlexDirection::Stack,
                    ..UiStyle::default()
                })
                .with_children([fill(
                    2,
                    UiStyle {
                        width: Dimension::Ratio {
                            units: 10,
                            base: 32,
                        },
                        height: Dimension::Ratio { units: 4, base: 24 },
                        position: Position::AbsoluteRatio {
                            left: 10,
                            top: 3,
                            base: UiSize::new(32, 24),
                        },
                        ..UiStyle::default()
                    },
                )]),
        )
        .unwrap();

        let frame = tree.resolve(UiSize::new(640, 480)).unwrap();
        match &frame.commands()[0] {
            UiDrawCommand::Fill { bounds, .. } => {
                assert_eq!(*bounds, UiRect::new(200, 60, 200, 80));
            }
            _ => panic!(),
        }
    }
    #[test]
    fn duplicate_ids_and_conflicting_minimum_rows_are_errors() {
        assert_eq!(
            UiTree::new(UiNode::new(UiId(1)).with_children([UiNode::new(UiId(1))])),
            Err(UiBuildError::DuplicateId(UiId(1)))
        );
        let tree = UiTree::new(
            UiNode::new(UiId(1))
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    direction: FlexDirection::Row,
                    ..UiStyle::default()
                })
                .with_children([
                    fill(
                        2,
                        UiStyle {
                            min_size: UiSize::new(8, 1),
                            ..UiStyle::fixed(8, 1)
                        },
                    ),
                    fill(
                        3,
                        UiStyle {
                            min_size: UiSize::new(8, 1),
                            ..UiStyle::fixed(8, 1)
                        },
                    ),
                ]),
        )
        .unwrap();
        assert!(matches!(
            tree.resolve(UiSize::new(10, 2)),
            Err(UiLayoutError::InsufficientSpace { .. })
        ));
    }
}
