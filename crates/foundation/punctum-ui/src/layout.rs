use crate::{
    CrossAlign, Dimension, FlexDirection, Insets, MainAlign, Position, UiBorderRadius,
    UiButtonState, UiButtonStyle, UiColor, UiContent, UiContentId, UiId, UiInteractionSnapshot,
    UiKey, UiLayoutError, UiPixelOffset, UiRect, UiRipple, UiSize, tree::UiNode,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiDrawCommand {
    Fill {
        bounds: UiRect,
        color: UiColor,
        border_radius: UiBorderRadius,
        clip: UiRect,
    },
    RadarChart {
        bounds: UiRect,
        values: [u16; 6],
        max: u16,
        rings: u8,
        grid_color: UiColor,
        axis_color: UiColor,
        fill_color: UiColor,
        edge_color: UiColor,
        point_color: UiColor,
        label_color: UiColor,
        labels: [String; 6],
        label_font_size: u32,
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
    Outline {
        bounds: UiRect,
        color: UiColor,
        radius: UiBorderRadius,
        width: u32,
        clip: UiRect,
    },
    Ripple {
        bounds: UiRect,
        center: UiPixelOffset,
        radius: u32,
        color: UiColor,
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
    pub id: UiId,
    pub key: Option<UiKey>,
    pub action: Action,
    pub bounds: UiRect,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UiInteractionTarget {
    pub id: UiId,
    pub bounds: UiRect,
    pub style: UiButtonStyle,
    pub command_index: usize,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiFrame<Action = ()> {
    viewport: UiSize,
    commands: Vec<UiDrawCommand>,
    hits: Vec<UiHitRegion>,
    action_hits: Vec<UiActionHit<Action>>,
    interaction_targets: Vec<UiInteractionTarget>,
}

struct ResolveBuffers<Action> {
    commands: Vec<UiDrawCommand>,
    hits: Vec<UiHitRegion>,
    action_hits: Vec<UiActionHit<Action>>,
    interaction_targets: Vec<UiInteractionTarget>,
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
    pub fn action_hits(&self) -> &[UiActionHit<Action>] {
        &self.action_hits
    }
    pub fn interaction_targets(&self) -> &[UiInteractionTarget] {
        &self.interaction_targets
    }
    /// 返回坐标命中的最上层动作及其自动分配的结构 ID。
    pub fn action_hit_at(&self, x: u32, y: u32) -> Option<&UiActionHit<Action>> {
        self.action_hits
            .iter()
            .rev()
            .find(|region| region.bounds.contains(x, y))
    }
    /// 返回坐标命中的最上层动作。
    /// 多个可交互节点重叠时，后绘制的节点优先。
    pub fn hit_action(&self, x: u32, y: u32) -> Option<&Action> {
        self.action_hit_at(x, y).map(|region| &region.action)
    }
    pub fn action_hit_by_id(&self, id: UiId) -> Option<&UiActionHit<Action>> {
        self.action_hits.iter().rev().find(|hit| hit.id == id)
    }

    pub fn with_interaction(&self, interaction: &UiInteractionSnapshot) -> Self
    where
        Action: Clone,
    {
        let mut commands = Vec::with_capacity(self.commands.len());
        for (index, command) in self.commands.iter().enumerate() {
            for target in &self.interaction_targets {
                if target.command_index == index {
                    commands.extend(dynamic_commands(
                        *target,
                        interaction.button_state(target),
                        &interaction.ripples,
                    ));
                }
            }
            commands.push(command.clone());
        }
        for target in &self.interaction_targets {
            if target.command_index == self.commands.len() {
                commands.extend(dynamic_commands(
                    *target,
                    interaction.button_state(target),
                    &interaction.ripples,
                ));
            }
        }
        Self {
            viewport: self.viewport,
            commands,
            hits: self.hits.clone(),
            action_hits: self.action_hits.clone(),
            interaction_targets: self.interaction_targets.clone(),
        }
    }
}

pub(crate) fn resolve_tree<Action: Clone>(
    root: &UiNode<Action>,
    viewport: UiSize,
) -> Result<UiFrame<Action>, UiLayoutError> {
    let root_bounds = UiRect::new(0, 0, viewport.width, viewport.height);
    let mut buffers = ResolveBuffers {
        commands: Vec::new(),
        hits: Vec::new(),
        action_hits: Vec::new(),
        interaction_targets: Vec::new(),
    };
    resolve_node(
        root,
        root_bounds,
        viewport,
        root_bounds,
        UiPixelOffset::default(),
        &mut buffers,
    )?;
    Ok(UiFrame {
        viewport,
        commands: buffers.commands,
        hits: buffers.hits,
        action_hits: buffers.action_hits,
        interaction_targets: buffers.interaction_targets,
    })
}
fn resolve_node<Action: Clone>(
    node: &UiNode<Action>,
    offered: UiRect,
    ratio_basis: UiSize,
    inherited_clip: UiRect,
    inherited_offset: UiPixelOffset,
    buffers: &mut ResolveBuffers<Action>,
) -> Result<(), UiLayoutError> {
    let layout_bounds = constrain(node, offered, ratio_basis)?;
    let offset = inherited_offset.saturating_add(node.style.visual_offset);
    let bounds = translate(layout_bounds, offset);
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
        buffers.commands.push(UiDrawCommand::Fill {
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
        UiContent::Fill(color) => buffers.commands.push(UiDrawCommand::Fill {
            bounds: paint_bounds,
            color: *color,
            border_radius: content_radius,
            clip,
        }),
        UiContent::RadarChart {
            values,
            max,
            rings,
            grid_color,
            axis_color,
            fill_color,
            edge_color,
            point_color,
            label_color,
            labels,
            label_font_size,
        } => buffers.commands.push(UiDrawCommand::RadarChart {
            bounds: paint_bounds,
            values: *values,
            max: *max,
            rings: *rings,
            grid_color: *grid_color,
            axis_color: *axis_color,
            fill_color: *fill_color,
            edge_color: *edge_color,
            point_color: *point_color,
            label_color: *label_color,
            labels: labels.clone(),
            label_font_size: *label_font_size,
            clip,
        }),
        UiContent::Image(content) => buffers.commands.push(UiDrawCommand::Image {
            bounds: paint_bounds,
            content: content.clone(),
            tint: UiColor::new(255, 255, 255, 255),
            pixel_offset: UiPixelOffset::default(),
            border_radius: content_radius,
            clip,
        }),
        UiContent::ImageTinted { content, tint } => buffers.commands.push(UiDrawCommand::Image {
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
        } => buffers.commands.push(UiDrawCommand::Image {
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
        } => buffers.commands.push(UiDrawCommand::Text {
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
        } => buffers.commands.push(UiDrawCommand::Text {
            bounds: paint_bounds,
            content: content.clone(),
            color: *color,
            font_size: font_size.resolve(ratio_basis.height),
            clip,
        }),
    }
    let hit_bounds = bounds.intersect(clip).unwrap_or_default();
    if let Some(button) = node.button {
        buffers.interaction_targets.push(UiInteractionTarget {
            id: node.id,
            bounds: hit_bounds,
            style: button,
            command_index: buffers.commands.len(),
        });
    }
    let disabled = node.button.is_some_and(|button| button.disabled);
    if node.action.is_some() && !disabled {
        buffers.hits.push(UiHitRegion {
            id: node.id,
            bounds: hit_bounds,
        });
    }
    if let Some(action) = &node.action
        && !disabled
    {
        buffers.action_hits.push(UiActionHit {
            id: node.id,
            key: node.key.clone(),
            action: action.clone(),
            bounds: hit_bounds,
        });
    }
    layout_children(
        node,
        inset(
            inset(layout_bounds, node.style.border.widths),
            node.style.padding,
        ),
        clip,
        offset,
        buffers,
    )
}

fn dynamic_commands(
    target: UiInteractionTarget,
    state: UiButtonState,
    ripples: &[UiRipple],
) -> Vec<UiDrawCommand> {
    let mut commands = Vec::new();
    if state.disabled {
        push_fill(&mut commands, target.bounds, target.style.disabled_color);
    } else {
        if state.hovered {
            push_fill(&mut commands, target.bounds, target.style.hover_color);
        }
        if state.pressed {
            push_fill(&mut commands, target.bounds, target.style.pressed_color);
        }
        if state.focused && target.style.focus_width != 0 {
            commands.push(UiDrawCommand::Outline {
                bounds: target.bounds,
                color: target.style.focus_color,
                radius: UiBorderRadius::all(0),
                width: target.style.focus_width,
                clip: target.bounds,
            });
        }
        for ripple in ripples.iter().filter(|ripple| ripple.target == target.id) {
            let radius = ripple.radius(target.bounds);
            let left = ripple.origin.x.saturating_sub(radius as i32).max(0) as u32;
            let top = ripple.origin.y.saturating_sub(radius as i32).max(0) as u32;
            let diameter = radius.saturating_mul(2).max(1);
            commands.push(UiDrawCommand::Ripple {
                bounds: UiRect::new(left, top, diameter, diameter),
                center: ripple.origin,
                radius,
                color: ripple.color,
                clip: target.bounds,
            });
        }
    }
    commands
}

fn push_fill(commands: &mut Vec<UiDrawCommand>, bounds: UiRect, color: UiColor) {
    if color.alpha != 0 && !bounds.is_empty() {
        commands.push(UiDrawCommand::Fill {
            bounds,
            color,
            border_radius: UiBorderRadius::default(),
            clip: bounds,
        });
    }
}

fn inset(bounds: UiRect, insets: Insets) -> UiRect {
    UiRect::new(
        bounds.x.saturating_add(insets.left),
        bounds.y.saturating_add(insets.top),
        bounds.width.saturating_sub(insets.horizontal()),
        bounds.height.saturating_sub(insets.vertical()),
    )
}

fn translate(bounds: UiRect, offset: UiPixelOffset) -> UiRect {
    let left = i64::from(bounds.x) + i64::from(offset.x);
    let top = i64::from(bounds.y) + i64::from(offset.y);
    let right = left + i64::from(bounds.width);
    let bottom = top + i64::from(bounds.height);
    if right <= 0 || bottom <= 0 {
        return UiRect::default();
    }
    let left = left.max(0).min(i64::from(u32::MAX)) as u32;
    let top = top.max(0).min(i64::from(u32::MAX)) as u32;
    let right = right.max(0).min(i64::from(u32::MAX)) as u32;
    let bottom = bottom.max(0).min(i64::from(u32::MAX)) as u32;
    if left >= right || top >= bottom {
        UiRect::default()
    } else {
        UiRect::new(left, top, right - left, bottom - top)
    }
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
    let content = match &node.content {
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
    };
    let children = intrinsic_flow_children_size(node, ratio_basis);
    let horizontal_insets = node
        .style
        .border
        .widths
        .horizontal()
        .saturating_add(node.style.padding.horizontal());
    let vertical_insets = node
        .style
        .border
        .widths
        .vertical()
        .saturating_add(node.style.padding.vertical());
    UiSize::new(
        content
            .width
            .max(children.width)
            .saturating_add(horizontal_insets),
        content
            .height
            .max(children.height)
            .saturating_add(vertical_insets),
    )
}

fn intrinsic_flow_children_size<Action>(node: &UiNode<Action>, ratio_basis: UiSize) -> UiSize {
    let children = node
        .children
        .iter()
        .filter(|child| matches!(child.style.position, Position::Flow))
        .map(|child| intrinsic_outer_size(child, ratio_basis))
        .collect::<Vec<_>>();
    let gap = node
        .style
        .gap
        .saturating_mul(children.len().saturating_sub(1) as u32);
    match node.style.direction {
        FlexDirection::Row => UiSize::new(
            children
                .iter()
                .fold(gap, |width, child| width.saturating_add(child.width)),
            children.iter().map(|child| child.height).max().unwrap_or(0),
        ),
        FlexDirection::Column => UiSize::new(
            children.iter().map(|child| child.width).max().unwrap_or(0),
            children
                .iter()
                .fold(gap, |height, child| height.saturating_add(child.height)),
        ),
        FlexDirection::Stack => UiSize::new(
            children.iter().map(|child| child.width).max().unwrap_or(0),
            children.iter().map(|child| child.height).max().unwrap_or(0),
        ),
    }
}

fn intrinsic_outer_size<Action>(node: &UiNode<Action>, ratio_basis: UiSize) -> UiSize {
    let intrinsic = intrinsic_size(node, ratio_basis);
    let width = intrinsic_dimension(node.style.width, ratio_basis.width, intrinsic.width)
        .max(node.style.min_size.width);
    let height = intrinsic_dimension(node.style.height, ratio_basis.height, intrinsic.height)
        .max(node.style.min_size.height);
    let (width, height) = node.style.max_size.map_or((width, height), |maximum| {
        (width.min(maximum.width), height.min(maximum.height))
    });
    UiSize::new(
        width.saturating_add(node.style.margin.horizontal()),
        height.saturating_add(node.style.margin.vertical()),
    )
}

fn intrinsic_dimension(value: Dimension, offered: u32, intrinsic: u32) -> u32 {
    match value {
        Dimension::Fill => 0,
        _ => dimension(value, offered, intrinsic),
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
    visual_offset: UiPixelOffset,
    buffers: &mut ResolveBuffers<Action>,
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
        resolve_node(child, offered, content.size(), clip, visual_offset, buffers)?;
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
        resolve_node(child, offered, content.size(), clip, visual_offset, buffers)?;
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
