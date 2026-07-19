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

/// A stable semantic identity for a dynamic node across pure tree rebuilds.
/// Static nodes should not need a key.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UiKey(String);
impl UiKey {
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
    #[deprecated(note = "Use UiNode::with_action for activatable nodes.")]
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiNode<Action = ()> {
    /// Legacy structural identity. New page code should use `UiNode::auto()`.
    pub id: UiId,
    pub key: Option<UiKey>,
    pub action: Option<Action>,
    pub style: UiStyle,
    pub content: UiContent,
    pub children: Vec<UiNode<Action>>,
    automatic_id: bool,
}
impl UiNode<()> {
    #[deprecated(
        note = "Use UiNode::auto(); UiTree::new assigns structural IDs deterministically."
    )]
    pub fn new(id: UiId) -> Self {
        Self {
            id,
            key: None,
            action: None,
            style: UiStyle::default(),
            content: UiContent::Empty,
            children: Vec::new(),
            automatic_id: false,
        }
    }
}
impl<Action> UiNode<Action> {
    /// Creates a node whose structural ID is assigned by `UiTree::new`.
    pub fn auto() -> Self {
        Self {
            id: UiId::default(),
            key: None,
            action: None,
            style: UiStyle::default(),
            content: UiContent::Empty,
            children: Vec::new(),
            automatic_id: true,
        }
    }
    #[deprecated(
        note = "Use UiNode::auto(); UiTree::new assigns structural IDs deterministically."
    )]
    pub fn legacy(id: UiId) -> Self {
        Self {
            id,
            key: None,
            action: None,
            style: UiStyle::default(),
            content: UiContent::Empty,
            children: Vec::new(),
            automatic_id: false,
        }
    }
    pub fn with_key(mut self, key: UiKey) -> Self {
        self.key = Some(key);
        self
    }
    #[allow(deprecated)]
    pub fn with_action(mut self, action: Action) -> Self {
        self.action = Some(action);
        self.style.interactive = true;
        self
    }
    pub fn with_style(mut self, style: UiStyle) -> Self {
        self.style = style;
        self
    }
    pub fn with_content(mut self, content: UiContent) -> Self {
        self.content = content;
        self
    }
    pub fn with_children(mut self, children: impl IntoIterator<Item = UiNode<Action>>) -> Self {
        self.children = children.into_iter().collect();
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiTree<Action = ()> {
    root: UiNode<Action>,
}
impl<Action> UiTree<Action> {
    pub fn new(mut root: UiNode<Action>) -> Result<Self, UiBuildError> {
        let mut reserved_ids = BTreeSet::new();
        collect_explicit_ids(&root, &mut reserved_ids)?;
        assign_automatic_ids(&mut root, &reserved_ids)?;
        let mut ids = BTreeSet::new();
        let mut keys = BTreeSet::new();
        validate_node(&root, &mut ids, &mut keys)?;
        Ok(Self { root })
    }
    pub fn root(&self) -> &UiNode<Action> {
        &self.root
    }
}
impl<Action: Clone> UiTree<Action> {
    pub fn resolve(&self, viewport: UiSize) -> Result<UiFrame<Action>, UiLayoutError> {
        resolve_tree(&self.root, viewport)
    }
}
fn collect_explicit_ids<Action>(
    node: &UiNode<Action>,
    ids: &mut BTreeSet<UiId>,
) -> Result<(), UiBuildError> {
    if !node.automatic_id && !ids.insert(node.id) {
        return Err(UiBuildError::DuplicateId(node.id));
    }
    for child in &node.children {
        collect_explicit_ids(child, ids)?;
    }
    Ok(())
}
fn assign_automatic_ids<Action>(
    node: &mut UiNode<Action>,
    reserved: &BTreeSet<UiId>,
) -> Result<(), UiBuildError> {
    fn next_available(
        next: &mut u32,
        reserved: &BTreeSet<UiId>,
        assigned: &BTreeSet<UiId>,
    ) -> Result<UiId, UiBuildError> {
        loop {
            let id = UiId(*next);
            if !reserved.contains(&id) && !assigned.contains(&id) {
                return Ok(id);
            }
            *next = next.checked_add(1).ok_or(UiBuildError::IdExhausted)?;
        }
    }
    fn visit<Action>(
        node: &mut UiNode<Action>,
        next: &mut u32,
        reserved: &BTreeSet<UiId>,
        assigned: &mut BTreeSet<UiId>,
    ) -> Result<(), UiBuildError> {
        if node.automatic_id {
            node.id = next_available(next, reserved, assigned)?;
            node.automatic_id = false;
            assigned.insert(node.id);
        }
        for child in &mut node.children {
            visit(child, next, reserved, assigned)?;
        }
        Ok(())
    }
    visit(node, &mut 0, reserved, &mut BTreeSet::new())
}
fn validate_node<Action>(
    node: &UiNode<Action>,
    ids: &mut BTreeSet<UiId>,
    keys: &mut BTreeSet<UiKey>,
) -> Result<(), UiBuildError> {
    if !ids.insert(node.id) {
        return Err(UiBuildError::DuplicateId(node.id));
    }
    if let Some(key) = &node.key
        && !keys.insert(key.clone())
    {
        return Err(UiBuildError::DuplicateKey(key.clone()));
    }
    validate_style(node.id, node.style)?;
    validate_content(node.id, &node.content)?;
    for child in &node.children {
        validate_node(child, ids, keys)?;
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
pub struct UiActionHit<Action> {
    pub key: Option<UiKey>,
    pub action: Action,
    pub bounds: UiRect,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiFrame<Action = ()> {
    viewport: UiSize,
    commands: Vec<UiDrawCommand>,
    hits: Vec<UiHitRegion>,
    action_hits: Vec<UiActionHit<Action>>,
}
impl<Action> UiFrame<Action> {
    pub const fn viewport(&self) -> UiSize {
        self.viewport
    }
    pub fn commands(&self) -> &[UiDrawCommand] {
        &self.commands
    }
    pub fn hit_regions(&self) -> &[UiHitRegion] {
        &self.hits
    }
    #[deprecated(note = "Use UiFrame::hit_action for page interaction.")]
    pub fn hit_test(&self, x: u32, y: u32) -> Option<UiId> {
        self.hits
            .iter()
            .rev()
            .find(|region| region.bounds.contains(x, y))
            .map(|region| region.id)
    }
    pub fn action_hits(&self) -> &[UiActionHit<Action>] {
        &self.action_hits
    }
    pub fn hit_action(&self, x: u32, y: u32) -> Option<&Action> {
        self.action_hits
            .iter()
            .rev()
            .find(|region| region.bounds.contains(x, y))
            .map(|region| &region.action)
    }
}

fn resolve_tree<Action: Clone>(
    root: &UiNode<Action>,
    viewport: UiSize,
) -> Result<UiFrame<Action>, UiLayoutError> {
    let root_bounds = UiRect::new(0, 0, viewport.width, viewport.height);
    let mut commands = Vec::new();
    let mut hits = Vec::new();
    let mut action_hits = Vec::new();
    resolve_node(
        root,
        root_bounds,
        viewport,
        root_bounds,
        &mut commands,
        &mut hits,
        &mut action_hits,
    )?;
    Ok(UiFrame {
        viewport,
        commands,
        hits,
        action_hits,
    })
}
#[allow(deprecated)]
fn resolve_node<Action: Clone>(
    node: &UiNode<Action>,
    offered: UiRect,
    ratio_basis: UiSize,
    inherited_clip: UiRect,
    commands: &mut Vec<UiDrawCommand>,
    hits: &mut Vec<UiHitRegion>,
    action_hits: &mut Vec<UiActionHit<Action>>,
) -> Result<(), UiLayoutError> {
    let bounds = constrain(node, offered, ratio_basis)?;
    let clip = if node.style.clip {
        inherited_clip.intersect(bounds).unwrap_or_default()
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
    let hit_bounds = bounds.intersect(clip).unwrap_or_default();
    if node.style.interactive || node.action.is_some() {
        hits.push(UiHitRegion {
            id: node.id,
            bounds: hit_bounds,
        });
    }
    if let Some(action) = &node.action {
        action_hits.push(UiActionHit {
            key: node.key.clone(),
            action: action.clone(),
            bounds: hit_bounds,
        });
    }
    layout_children(
        node,
        inset(paint_bounds, node.style.padding),
        clip,
        commands,
        hits,
        action_hits,
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
fn constrain<Action>(
    node: &UiNode<Action>,
    offered: UiRect,
    ratio_basis: UiSize,
) -> Result<UiRect, UiLayoutError> {
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
fn intrinsic_size<Action>(node: &UiNode<Action>, ratio_basis: UiSize) -> UiSize {
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
fn layout_children<Action: Clone>(
    node: &UiNode<Action>,
    content: UiRect,
    clip: UiRect,
    commands: &mut Vec<UiDrawCommand>,
    hits: &mut Vec<UiHitRegion>,
    action_hits: &mut Vec<UiActionHit<Action>>,
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
    let fill = remaining.checked_div(fills).unwrap_or(0);
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
        resolve_node(
            child,
            offered,
            content.size(),
            clip,
            commands,
            hits,
            action_hits,
        )?;
        if !stacked {
            cursor = cursor
                .saturating_add(main)
                .saturating_add(margin_after)
                .saturating_add(distributed_gap);
        }
    }
    for (child, left, top) in node.children.iter().filter_map(|child| {
        let (left, top) = match child.style.position {
            Position::Absolute { left, top } => (left, top),
            Position::AbsoluteRatio { left, top, base } => (
                content.width.saturating_mul(left) / base.width,
                content.height.saturating_mul(top) / base.height,
            ),
            Position::Flow => return None,
        };
        Some((child, left, top))
    }) {
        let offered = UiRect::new(
            content.x.saturating_add(left),
            content.y.saturating_add(top),
            content.width.saturating_sub(left),
            content.height.saturating_sub(top),
        );
        resolve_node(
            child,
            offered,
            content.size(),
            clip,
            commands,
            hits,
            action_hits,
        )?;
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
    EmptyKey,
    DuplicateId(UiId),
    DuplicateKey(UiKey),
    IdExhausted,
    ZeroRatioBase(UiId),
    ZeroLogicalCanvas(UiId),
    ZeroTextSizeBase(UiId),
}
impl fmt::Display for UiBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyContentId => f.write_str("UI content id must not be empty"),
            Self::EmptyKey => f.write_str("UI key must not be empty"),
            Self::DuplicateId(id) => write!(f, "UI node id {:?} is duplicated", id),
            Self::DuplicateKey(key) => write!(f, "UI key {:?} is duplicated", key),
            Self::IdExhausted => f.write_str("UI tree exhausted all structural IDs"),
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
#[allow(deprecated)]
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

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum TestAction {
        Back,
        Front,
    }

    #[test]
    fn automatic_ids_and_typed_actions_are_deterministic() {
        let item_style = UiStyle {
            width: Dimension::Px(30),
            height: Dimension::Px(30),
            position: Position::Absolute { left: 10, top: 10 },
            ..UiStyle::default()
        };
        let tree = UiTree::<TestAction>::new(
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    direction: FlexDirection::Stack,
                    ..UiStyle::default()
                })
                .with_children([
                    UiNode::auto()
                        .with_key(UiKey::new("back").unwrap())
                        .with_style(item_style)
                        .with_action(TestAction::Back),
                    UiNode::auto()
                        .with_key(UiKey::new("front").unwrap())
                        .with_style(item_style)
                        .with_action(TestAction::Front),
                    UiNode::legacy(UiId(1)).with_style(UiStyle::fixed(1, 1)),
                ]),
        )
        .unwrap();

        assert_eq!(tree.root().id, UiId(0));
        assert_eq!(tree.root().children[0].id, UiId(2));
        assert_eq!(tree.root().children[1].id, UiId(3));
        assert_eq!(tree.root().children[2].id, UiId(1));

        let frame = tree.resolve(UiSize::new(40, 40)).unwrap();
        assert_eq!(frame.hit_action(15, 15), Some(&TestAction::Front));
        assert_eq!(frame.action_hits().len(), 2);
        assert_eq!(
            frame.action_hits()[1].key,
            Some(UiKey::new("front").unwrap())
        );
    }

    #[test]
    fn a_large_automatic_tree_needs_no_caller_supplied_ids() {
        let tree = UiTree::new(
            UiNode::<()>::auto()
                .with_children((0..1_000).map(|_| UiNode::auto().with_style(UiStyle::fixed(1, 1)))),
        )
        .unwrap();

        assert_eq!(tree.root().id, UiId(0));
        assert_eq!(tree.root().children.len(), 1_000);
        assert_eq!(tree.root().children[999].id, UiId(1_000));
    }

    #[test]
    fn duplicate_ui_keys_are_build_errors() {
        let duplicate = UiTree::<TestAction>::new(UiNode::auto().with_children([
            UiNode::auto().with_key(UiKey::new("entry").unwrap()),
            UiNode::auto().with_key(UiKey::new("entry").unwrap()),
        ]));
        assert!(matches!(
            duplicate,
            Err(UiBuildError::DuplicateKey(key)) if key == UiKey::new("entry").unwrap()
        ));
        assert_eq!(UiKey::new(""), Err(UiBuildError::EmptyKey));
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

    #[test]
    fn scalar_values_and_build_failures_are_explicit() {
        assert_eq!(UiPixelOffset::new(-2, 3), UiPixelOffset { x: -2, y: 3 });
        assert_eq!(UiContentId::new(""), Err(UiBuildError::EmptyContentId));
        assert!(UiBorderRadius::default().is_zero());
        assert_eq!(
            UiTextSize::Ratio {
                units: 3,
                base: 4,
                minimum: 5,
                maximum: 7,
            }
            .resolve(20),
            7
        );

        let root = UiNode::new(UiId(1));
        assert_eq!(UiTree::new(root.clone()).unwrap().root(), &root);
        let invalid = [
            UiNode::new(UiId(2)).with_content(UiContent::TextScaled {
                content: "x".into(),
                color: UiColor::default(),
                font_size: UiTextSize::Ratio {
                    units: 1,
                    base: 0,
                    minimum: 1,
                    maximum: 2,
                },
            }),
            UiNode::new(UiId(3)).with_style(UiStyle {
                width: Dimension::Ratio { units: 1, base: 0 },
                ..UiStyle::default()
            }),
            UiNode::new(UiId(4)).with_style(UiStyle {
                position: Position::AbsoluteRatio {
                    left: 0,
                    top: 0,
                    base: UiSize::new(0, 1),
                },
                ..UiStyle::default()
            }),
            UiNode::new(UiId(5)).with_style(UiStyle {
                logical_canvas: Some(UiSize::new(1, 0)),
                ..UiStyle::default()
            }),
        ];
        for node in invalid {
            assert!(UiTree::new(node).is_err());
        }
        for error in [
            UiBuildError::EmptyContentId,
            UiBuildError::DuplicateId(UiId(1)),
            UiBuildError::ZeroRatioBase(UiId(1)),
            UiBuildError::ZeroLogicalCanvas(UiId(1)),
            UiBuildError::ZeroTextSizeBase(UiId(1)),
        ] {
            assert!(!error.to_string().is_empty());
        }
        assert!(
            !UiLayoutError::InsufficientSpace { id: UiId(1) }
                .to_string()
                .is_empty()
        );
    }

    #[test]
    fn uncommon_content_and_layout_modes_resolve_deterministically() {
        let styled = UiTree::new(
            UiNode::new(UiId(1))
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    logical_canvas: Some(UiSize::new(10, 5)),
                    max_size: Some(UiSize::new(30, 20)),
                    ..UiStyle::default()
                })
                .with_content(UiContent::ImageStyled {
                    content: UiContentId::new("panel").unwrap(),
                    tint: UiColor::new(1, 2, 3, 4),
                    pixel_offset: UiPixelOffset::new(2, -1),
                }),
        )
        .unwrap();
        let styled_frame = styled.resolve(UiSize::new(40, 20)).unwrap();
        assert!(matches!(
            styled_frame.commands()[0],
            UiDrawCommand::Image {
                bounds: UiRect {
                    x: 0,
                    y: 0,
                    width: 40,
                    height: 20
                },
                pixel_offset: UiPixelOffset { x: 2, y: -1 },
                ..
            }
        ));

        let text = |id| {
            UiNode::new(UiId(id)).with_content(UiContent::Text {
                content: "wide".into(),
                color: UiColor::default(),
                font_size: 4,
            })
        };
        let horizontal = UiTree::new(
            UiNode::new(UiId(10))
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    direction: FlexDirection::Row,
                    main_align: MainAlign::End,
                    cross_align: CrossAlign::Stretch,
                    ..UiStyle::default()
                })
                .with_children([text(11)]),
        )
        .unwrap();
        assert!(horizontal.resolve(UiSize::new(30, 12)).is_ok());

        let vertical = UiTree::new(
            UiNode::new(UiId(20))
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    cross_align: CrossAlign::End,
                    ..UiStyle::default()
                })
                .with_children([text(21)]),
        )
        .unwrap();
        assert!(vertical.resolve(UiSize::new(30, 12)).is_ok());
    }

    #[test]
    fn remaining_content_and_flex_branches_stay_pure() {
        assert_eq!(UiContentId::new("asset").unwrap().as_str(), "asset");
        assert_eq!(UiBorderRadius::all(3).bottom_left, 3);
        assert_eq!(Insets::all(2), Insets::symmetric(2, 2));
        assert_eq!(UiTextSize::Px(6).resolve(99), 6);

        let image = UiContentId::new("asset").unwrap();
        let bordered_image = UiTree::new(
            UiNode::new(UiId(1))
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    border: UiBorder {
                        widths: Insets::all(1),
                        color: UiColor::new(9, 8, 7, 6),
                    },
                    interactive: true,
                    ..UiStyle::default()
                })
                .with_content(UiContent::Image(image.clone())),
        )
        .unwrap();
        let frame = bordered_image.resolve(UiSize::new(10, 8)).unwrap();
        assert_eq!(frame.viewport(), UiSize::new(10, 8));
        assert_eq!(frame.commands().len(), 2);
        assert_eq!(
            frame.hit_regions(),
            &[UiHitRegion {
                id: UiId(1),
                bounds: UiRect::new(0, 0, 10, 8)
            }]
        );

        let tinted = UiTree::new(
            UiNode::new(UiId(2))
                .with_style(UiStyle::fixed(10, 8))
                .with_content(UiContent::ImageTinted {
                    content: image.clone(),
                    tint: UiColor::new(1, 2, 3, 4),
                }),
        )
        .unwrap();
        assert!(matches!(
            tinted.resolve(UiSize::new(10, 8)).unwrap().commands()[0],
            UiDrawCommand::Image { .. }
        ));

        let scaled = UiTree::new(
            UiNode::new(UiId(3))
                .with_style(UiStyle::fixed(20, 10))
                .with_content(UiContent::TextScaled {
                    content: "x".into(),
                    color: UiColor::default(),
                    font_size: UiTextSize::Ratio {
                        units: 1,
                        base: 2,
                        minimum: 1,
                        maximum: 8,
                    },
                }),
        )
        .unwrap();
        assert!(matches!(
            scaled.resolve(UiSize::new(20, 10)).unwrap().commands()[0],
            UiDrawCommand::Text { font_size: 5, .. }
        ));
        assert!(
            UiTree::new(UiNode::new(UiId(4)).with_style(UiStyle::fixed(0, 1)))
                .unwrap()
                .resolve(UiSize::new(10, 10))
                .unwrap()
                .commands()
                .is_empty()
        );

        let ratio_row = UiTree::new(
            UiNode::new(UiId(10))
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    direction: FlexDirection::Row,
                    main_align: MainAlign::Center,
                    cross_align: CrossAlign::Center,
                    ..UiStyle::default()
                })
                .with_children([fill(
                    11,
                    UiStyle {
                        width: Dimension::Ratio { units: 1, base: 2 },
                        height: Dimension::Ratio { units: 1, base: 2 },
                        ..UiStyle::default()
                    },
                )]),
        )
        .unwrap();
        assert!(ratio_row.resolve(UiSize::new(20, 10)).is_ok());

        let ratio_column = UiTree::new(
            UiNode::new(UiId(15))
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    ..UiStyle::default()
                })
                .with_children([fill(
                    16,
                    UiStyle {
                        width: Dimension::Ratio { units: 1, base: 2 },
                        height: Dimension::Ratio { units: 1, base: 2 },
                        ..UiStyle::default()
                    },
                )]),
        )
        .unwrap();
        assert!(ratio_column.resolve(UiSize::new(20, 10)).is_ok());

        let spaced_stack = UiTree::new(
            UiNode::new(UiId(20))
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    direction: FlexDirection::Stack,
                    main_align: MainAlign::SpaceBetween,
                    ..UiStyle::default()
                })
                .with_children([
                    fill(21, UiStyle::fixed(2, 2)),
                    fill(22, UiStyle::fixed(2, 2)),
                ]),
        )
        .unwrap();
        assert!(spaced_stack.resolve(UiSize::new(10, 10)).is_ok());
    }
}
