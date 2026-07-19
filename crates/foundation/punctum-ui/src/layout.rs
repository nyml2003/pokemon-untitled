use crate::{
    CrossAlign, Dimension, FlexDirection, Insets, MainAlign, Position, UiBorderRadius, UiColor,
    UiContent, UiContentId, UiId, UiKey, UiLayoutError, UiPixelOffset, UiRect, UiSize,
    tree::UiNode,
};

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
    #[deprecated(note = "页面交互请使用 UiFrame::hit_action。")]
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
    /// 返回坐标命中的最上层动作。
    /// 多个可交互节点重叠时，后绘制的节点优先。
    pub fn hit_action(&self, x: u32, y: u32) -> Option<&Action> {
        self.action_hits
            .iter()
            .rev()
            .find(|region| region.bounds.contains(x, y))
            .map(|region| &region.action)
    }
}

pub(crate) fn resolve_tree<Action: Clone>(
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
