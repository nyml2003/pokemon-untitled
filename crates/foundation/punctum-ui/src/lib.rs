//! Backend-neutral, stateless layout and paint primitives for discrete surfaces.
//!
//! The pipeline has three boundaries:
//!
//! 1. [`Ui::measure`] applies integer [`Constraints`] and asks a [`TextLayouter`]
//!    for backend-owned text layout results.
//! 2. [`Measured::layout`] places the measured tree without consulting the backend.
//! 3. [`Frame::paint`] reuses the stored text layout results and sends clipped draw
//!    operations to a [`PaintTarget`]. It never measures text again.
//!
//! Resizing reruns the same stateless measure/layout pipeline with new constraints.
//! Colors, fonts, border appearance, and themes belong to backend `PaintTarget`
//! implementations; they are intentionally outside this layout crate.

#![forbid(unsafe_code)]

use std::{error::Error, fmt};

use punctum_grid::{GridPos, GridRect, GridSize};

const MAX_COORDINATE: u32 = i32::MAX as u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Constraints {
    min: GridSize,
    max: GridSize,
}

impl Constraints {
    pub fn new(min: GridSize, max: GridSize) -> Result<Self, ConstraintError> {
        if min.cols > max.cols || min.rows > max.rows {
            return Err(ConstraintError::MinimumExceedsMaximum { min, max });
        }
        if max.cols > MAX_COORDINATE || max.rows > MAX_COORDINATE {
            return Err(ConstraintError::CoordinateRangeExceeded { size: max });
        }
        Ok(Self { min, max })
    }

    pub fn loose(max: GridSize) -> Result<Self, ConstraintError> {
        Self::new(GridSize::new(0, 0), max)
    }

    pub fn tight(size: GridSize) -> Result<Self, ConstraintError> {
        Self::new(size, size)
    }

    pub const fn min(self) -> GridSize {
        self.min
    }

    pub const fn max(self) -> GridSize {
        self.max
    }

    fn constrain(self, size: GridSize) -> GridSize {
        GridSize::new(
            size.cols.clamp(self.min.cols, self.max.cols),
            size.rows.clamp(self.min.rows, self.max.rows),
        )
    }

    fn loosened_with_max(self, max: GridSize) -> Self {
        Self {
            min: GridSize::new(0, 0),
            max,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstraintError {
    MinimumExceedsMaximum { min: GridSize, max: GridSize },
    CoordinateRangeExceeded { size: GridSize },
}

impl fmt::Display for ConstraintError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MinimumExceedsMaximum { min, max } => {
                write!(
                    formatter,
                    "minimum size {min:?} exceeds maximum size {max:?}"
                )
            }
            Self::CoordinateRangeExceeded { size } => {
                write!(
                    formatter,
                    "size {size:?} exceeds the signed UI coordinate range"
                )
            }
        }
    }
}

impl Error for ConstraintError {}

pub trait TextLayout {
    fn size(&self) -> GridSize;
}

pub trait TextLayouter {
    type Layout: TextLayout;
    type Error;

    fn layout_text(
        &mut self,
        content: &str,
        constraints: Constraints,
    ) -> Result<Self::Layout, Self::Error>;
}

pub trait PaintTarget<S, L> {
    type Error;

    fn paint_text(
        &mut self,
        content: &str,
        layout: &L,
        bounds: GridRect,
        clip: GridRect,
    ) -> Result<(), Self::Error>;

    fn paint_border(&mut self, bounds: GridRect, clip: GridRect) -> Result<(), Self::Error>;

    fn paint_surface(
        &mut self,
        surface: &S,
        bounds: GridRect,
        clip: GridRect,
    ) -> Result<(), Self::Error>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Text {
    content: String,
}

impl Text {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }

    pub fn content(&self) -> &str {
        &self.content
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row<S> {
    children: Vec<Node<S>>,
    gap: u32,
}

impl<S> Row<S> {
    pub fn new(children: Vec<Node<S>>) -> Self {
        Self { children, gap: 0 }
    }

    pub fn with_gap(mut self, gap: u32) -> Self {
        self.gap = gap;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Column<S> {
    children: Vec<Node<S>>,
    gap: u32,
}

impl<S> Column<S> {
    pub fn new(children: Vec<Node<S>>) -> Self {
        Self { children, gap: 0 }
    }

    pub fn with_gap(mut self, gap: u32) -> Self {
        self.gap = gap;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Border<S> {
    child: Box<Node<S>>,
}

impl<S> Border<S> {
    pub fn new(child: impl Into<Node<S>>) -> Self {
        Self {
            child: Box::new(child.into()),
        }
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
    pub const fn new(left: u32, top: u32, right: u32, bottom: u32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub const fn all(value: u32) -> Self {
        Self::new(value, value, value, value)
    }

    pub const fn symmetric(horizontal: u32, vertical: u32) -> Self {
        Self::new(horizontal, vertical, horizontal, vertical)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Padding<S> {
    insets: Insets,
    child: Box<Node<S>>,
}

impl<S> Padding<S> {
    pub fn new(insets: Insets, child: impl Into<Node<S>>) -> Self {
        Self {
            insets,
            child: Box::new(child.into()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spacer {
    size: GridSize,
}

impl Spacer {
    pub const fn new(size: GridSize) -> Self {
        Self { size }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HorizontalAlign {
    Start,
    Center,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerticalAlign {
    Start,
    Center,
    End,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Align<S> {
    horizontal: HorizontalAlign,
    vertical: VerticalAlign,
    child: Box<Node<S>>,
}

impl<S> Align<S> {
    pub fn new(
        horizontal: HorizontalAlign,
        vertical: VerticalAlign,
        child: impl Into<Node<S>>,
    ) -> Self {
        Self {
            horizontal,
            vertical,
            child: Box::new(child.into()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurfaceView<S> {
    surface: S,
    size: GridSize,
}

impl<S> SurfaceView<S> {
    pub const fn new(surface: S, size: GridSize) -> Self {
        Self { surface, size }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Node<S> {
    Text(Text),
    Row(Row<S>),
    Column(Column<S>),
    Border(Border<S>),
    Padding(Padding<S>),
    Spacer(Spacer),
    Align(Align<S>),
    SurfaceView(SurfaceView<S>),
}

impl<S> From<Text> for Node<S> {
    fn from(value: Text) -> Self {
        Self::Text(value)
    }
}

impl<S> From<Row<S>> for Node<S> {
    fn from(value: Row<S>) -> Self {
        Self::Row(value)
    }
}

impl<S> From<Column<S>> for Node<S> {
    fn from(value: Column<S>) -> Self {
        Self::Column(value)
    }
}

impl<S> From<Border<S>> for Node<S> {
    fn from(value: Border<S>) -> Self {
        Self::Border(value)
    }
}

impl<S> From<Padding<S>> for Node<S> {
    fn from(value: Padding<S>) -> Self {
        Self::Padding(value)
    }
}

impl<S> From<Spacer> for Node<S> {
    fn from(value: Spacer) -> Self {
        Self::Spacer(value)
    }
}

impl<S> From<Align<S>> for Node<S> {
    fn from(value: Align<S>) -> Self {
        Self::Align(value)
    }
}

impl<S> From<SurfaceView<S>> for Node<S> {
    fn from(value: SurfaceView<S>) -> Self {
        Self::SurfaceView(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ui<S> {
    root: Node<S>,
}

impl<S> Ui<S> {
    pub fn new(root: impl Into<Node<S>>) -> Self {
        Self { root: root.into() }
    }

    pub fn measure<'a, T>(
        &'a self,
        constraints: Constraints,
        text_layouter: &mut T,
    ) -> Result<Measured<'a, S, T::Layout>, LayoutError<T::Error>>
    where
        T: TextLayouter,
    {
        Ok(Measured {
            root: measure_node(&self.root, constraints, text_layouter)?,
        })
    }

    pub fn layout<'a, T>(
        &'a self,
        constraints: Constraints,
        text_layouter: &mut T,
    ) -> Result<Frame<'a, S, T::Layout>, LayoutError<T::Error>>
    where
        T: TextLayouter,
    {
        Ok(self.measure(constraints, text_layouter)?.layout())
    }
}

pub struct Measured<'a, S, L> {
    root: MeasuredNode<'a, S, L>,
}

impl<'a, S, L> Measured<'a, S, L> {
    pub fn size(&self) -> GridSize {
        self.root.size()
    }

    pub fn layout(self) -> Frame<'a, S, L> {
        let size = self.root.size();
        Frame {
            size,
            root: place_node(self.root, GridPos::new(0, 0)),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum LayoutError<E> {
    TextLayout { source: E },
    TextLayoutOutOfBounds { size: GridSize, max: GridSize },
}

impl<E: fmt::Display> fmt::Display for LayoutError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TextLayout { source } => write!(formatter, "text layout failed: {source}"),
            Self::TextLayoutOutOfBounds { size, max } => {
                write!(
                    formatter,
                    "text layout size {size:?} exceeds maximum {max:?}"
                )
            }
        }
    }
}

impl<E: Error + 'static> Error for LayoutError<E> {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutKind {
    Text,
    Row,
    Column,
    Border,
    Padding,
    Spacer,
    Align,
    SurfaceView,
}

pub struct Frame<'a, S, L> {
    size: GridSize,
    root: PlacedNode<'a, S, L>,
}

impl<S, L> Frame<'_, S, L> {
    pub const fn size(&self) -> GridSize {
        self.size
    }

    pub fn entries(&self) -> Vec<(LayoutKind, GridRect)> {
        let mut entries = Vec::new();
        self.root.collect_entries(&mut entries);
        entries
    }

    pub fn paint<P>(&self, target: &mut P) -> Result<(), PaintError<P::Error>>
    where
        P: PaintTarget<S, L>,
    {
        self.paint_clipped(GridRect::new(GridPos::new(0, 0), self.size), target)
    }

    pub fn paint_clipped<P>(
        &self,
        clip: GridRect,
        target: &mut P,
    ) -> Result<(), PaintError<P::Error>>
    where
        P: PaintTarget<S, L>,
    {
        let Some(clip) = clip.intersection(GridRect::new(GridPos::new(0, 0), self.size)) else {
            return Ok(());
        };
        self.root.paint(clip, target)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum PaintError<E> {
    Target { source: E },
}

impl<E: fmt::Display> fmt::Display for PaintError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Target { source } => write!(formatter, "paint target failed: {source}"),
        }
    }
}

impl<E: Error + 'static> Error for PaintError<E> {}

enum MeasuredNode<'a, S, L> {
    Text {
        content: &'a str,
        layout: L,
        size: GridSize,
    },
    Row {
        children: Vec<Self>,
        gaps: Vec<u32>,
        size: GridSize,
    },
    Column {
        children: Vec<Self>,
        gaps: Vec<u32>,
        size: GridSize,
    },
    Border {
        child: Box<Self>,
        size: GridSize,
    },
    Padding {
        insets: Insets,
        child: Box<Self>,
        size: GridSize,
    },
    Spacer {
        size: GridSize,
    },
    Align {
        horizontal: HorizontalAlign,
        vertical: VerticalAlign,
        child: Box<Self>,
        size: GridSize,
    },
    SurfaceView {
        surface: &'a S,
        size: GridSize,
    },
}

impl<S, L> MeasuredNode<'_, S, L> {
    fn size(&self) -> GridSize {
        match self {
            Self::Text { size, .. }
            | Self::Row { size, .. }
            | Self::Column { size, .. }
            | Self::Border { size, .. }
            | Self::Padding { size, .. }
            | Self::Spacer { size }
            | Self::Align { size, .. }
            | Self::SurfaceView { size, .. } => *size,
        }
    }
}

fn measure_node<'a, S, T>(
    node: &'a Node<S>,
    constraints: Constraints,
    text_layouter: &mut T,
) -> Result<MeasuredNode<'a, S, T::Layout>, LayoutError<T::Error>>
where
    T: TextLayouter,
{
    match node {
        Node::Text(text) => {
            let layout = text_layouter
                .layout_text(text.content(), constraints)
                .map_err(|source| LayoutError::TextLayout { source })?;
            let layout_size = layout.size();
            if layout_size.cols > constraints.max.cols || layout_size.rows > constraints.max.rows {
                return Err(LayoutError::TextLayoutOutOfBounds {
                    size: layout_size,
                    max: constraints.max,
                });
            }
            Ok(MeasuredNode::Text {
                content: text.content(),
                layout,
                size: constraints.constrain(layout_size),
            })
        }
        Node::Row(row) => measure_linear(
            &row.children,
            row.gap,
            constraints,
            text_layouter,
            Axis::Horizontal,
        ),
        Node::Column(column) => measure_linear(
            &column.children,
            column.gap,
            constraints,
            text_layouter,
            Axis::Vertical,
        ),
        Node::Border(border) => {
            let inner_max = subtract_size(constraints.max, GridSize::new(2, 2));
            let child = measure_node(
                &border.child,
                constraints.loosened_with_max(inner_max),
                text_layouter,
            )?;
            let desired = add_size(child.size(), GridSize::new(2, 2));
            Ok(MeasuredNode::Border {
                child: Box::new(child),
                size: constraints.constrain(desired),
            })
        }
        Node::Padding(padding) => {
            let inset_size = GridSize::new(
                padding.insets.left.saturating_add(padding.insets.right),
                padding.insets.top.saturating_add(padding.insets.bottom),
            );
            let inner_max = subtract_size(constraints.max, inset_size);
            let child = measure_node(
                &padding.child,
                constraints.loosened_with_max(inner_max),
                text_layouter,
            )?;
            let desired = add_size(child.size(), inset_size);
            Ok(MeasuredNode::Padding {
                insets: padding.insets,
                child: Box::new(child),
                size: constraints.constrain(desired),
            })
        }
        Node::Spacer(spacer) => Ok(MeasuredNode::Spacer {
            size: constraints.constrain(spacer.size),
        }),
        Node::Align(align) => {
            let child = measure_node(
                &align.child,
                constraints.loosened_with_max(constraints.max),
                text_layouter,
            )?;
            Ok(MeasuredNode::Align {
                horizontal: align.horizontal,
                vertical: align.vertical,
                child: Box::new(child),
                size: constraints.max,
            })
        }
        Node::SurfaceView(view) => Ok(MeasuredNode::SurfaceView {
            surface: &view.surface,
            size: constraints.constrain(view.size),
        }),
    }
}

#[derive(Clone, Copy)]
enum Axis {
    Horizontal,
    Vertical,
}

fn measure_linear<'a, S, T>(
    nodes: &'a [Node<S>],
    requested_gap: u32,
    constraints: Constraints,
    text_layouter: &mut T,
    axis: Axis,
) -> Result<MeasuredNode<'a, S, T::Layout>, LayoutError<T::Error>>
where
    T: TextLayouter,
{
    let main_max = main(constraints.max, axis);
    let gap_count = u32::try_from(nodes.len().saturating_sub(1)).unwrap_or(u32::MAX);
    let mut gap_budget = requested_gap.saturating_mul(gap_count).min(main_max);
    let mut gaps = Vec::with_capacity(nodes.len().saturating_sub(1));
    for _ in 1..nodes.len() {
        let gap = requested_gap.min(gap_budget);
        gaps.push(gap);
        gap_budget -= gap;
    }

    let total_gaps: u32 = gaps.iter().copied().sum();
    let mut remaining = main_max - total_gaps;
    let mut cross_size = 0;
    let mut children = Vec::with_capacity(nodes.len());
    for node in nodes {
        let child_max = with_main(constraints.max, axis, remaining);
        let child = measure_node(
            node,
            constraints.loosened_with_max(child_max),
            text_layouter,
        )?;
        remaining -= main(child.size(), axis);
        cross_size = cross_size.max(cross(child.size(), axis));
        children.push(child);
    }

    let used_main = main_max - remaining;
    let desired = from_axes(used_main, cross_size, axis);
    let size = constraints.constrain(desired);
    Ok(match axis {
        Axis::Horizontal => MeasuredNode::Row {
            children,
            gaps,
            size,
        },
        Axis::Vertical => MeasuredNode::Column {
            children,
            gaps,
            size,
        },
    })
}

fn main(size: GridSize, axis: Axis) -> u32 {
    match axis {
        Axis::Horizontal => size.cols,
        Axis::Vertical => size.rows,
    }
}

fn cross(size: GridSize, axis: Axis) -> u32 {
    match axis {
        Axis::Horizontal => size.rows,
        Axis::Vertical => size.cols,
    }
}

fn with_main(size: GridSize, axis: Axis, value: u32) -> GridSize {
    match axis {
        Axis::Horizontal => GridSize::new(value, size.rows),
        Axis::Vertical => GridSize::new(size.cols, value),
    }
}

fn from_axes(main: u32, cross: u32, axis: Axis) -> GridSize {
    match axis {
        Axis::Horizontal => GridSize::new(main, cross),
        Axis::Vertical => GridSize::new(cross, main),
    }
}

fn add_size(left: GridSize, right: GridSize) -> GridSize {
    GridSize::new(
        left.cols.saturating_add(right.cols).min(MAX_COORDINATE),
        left.rows.saturating_add(right.rows).min(MAX_COORDINATE),
    )
}

fn subtract_size(size: GridSize, amount: GridSize) -> GridSize {
    GridSize::new(
        size.cols.saturating_sub(amount.cols),
        size.rows.saturating_sub(amount.rows),
    )
}

enum PlacedNode<'a, S, L> {
    Text {
        content: &'a str,
        layout: L,
        bounds: GridRect,
    },
    Row {
        children: Vec<Self>,
        bounds: GridRect,
    },
    Column {
        children: Vec<Self>,
        bounds: GridRect,
    },
    Border {
        child: Box<Self>,
        bounds: GridRect,
    },
    Padding {
        child: Box<Self>,
        bounds: GridRect,
    },
    Spacer {
        bounds: GridRect,
    },
    Align {
        child: Box<Self>,
        bounds: GridRect,
    },
    SurfaceView {
        surface: &'a S,
        bounds: GridRect,
    },
}

impl<'a, S, L> PlacedNode<'a, S, L> {
    fn bounds(&self) -> GridRect {
        match self {
            Self::Text { bounds, .. }
            | Self::Row { bounds, .. }
            | Self::Column { bounds, .. }
            | Self::Border { bounds, .. }
            | Self::Padding { bounds, .. }
            | Self::Spacer { bounds }
            | Self::Align { bounds, .. }
            | Self::SurfaceView { bounds, .. } => *bounds,
        }
    }

    fn kind(&self) -> LayoutKind {
        match self {
            Self::Text { .. } => LayoutKind::Text,
            Self::Row { .. } => LayoutKind::Row,
            Self::Column { .. } => LayoutKind::Column,
            Self::Border { .. } => LayoutKind::Border,
            Self::Padding { .. } => LayoutKind::Padding,
            Self::Spacer { .. } => LayoutKind::Spacer,
            Self::Align { .. } => LayoutKind::Align,
            Self::SurfaceView { .. } => LayoutKind::SurfaceView,
        }
    }

    fn collect_entries(&self, entries: &mut Vec<(LayoutKind, GridRect)>) {
        entries.push((self.kind(), self.bounds()));
        match self {
            Self::Row { children, .. } | Self::Column { children, .. } => {
                for child in children {
                    child.collect_entries(entries);
                }
            }
            Self::Border { child, .. }
            | Self::Padding { child, .. }
            | Self::Align { child, .. } => child.collect_entries(entries),
            Self::Text { .. } | Self::Spacer { .. } | Self::SurfaceView { .. } => {}
        }
    }

    fn paint<P>(&self, parent_clip: GridRect, target: &mut P) -> Result<(), PaintError<P::Error>>
    where
        P: PaintTarget<S, L>,
    {
        let Some(clip) = parent_clip.intersection(self.bounds()) else {
            return Ok(());
        };
        match self {
            Self::Text {
                content,
                layout,
                bounds,
            } => target
                .paint_text(content, layout, *bounds, clip)
                .map_err(|source| PaintError::Target { source }),
            Self::Row { children, .. } | Self::Column { children, .. } => {
                for child in children {
                    child.paint(clip, target)?;
                }
                Ok(())
            }
            Self::Border { child, bounds } => {
                target
                    .paint_border(*bounds, clip)
                    .map_err(|source| PaintError::Target { source })?;
                child.paint(clip, target)
            }
            Self::Padding { child, .. } | Self::Align { child, .. } => child.paint(clip, target),
            Self::Spacer { .. } => Ok(()),
            Self::SurfaceView { surface, bounds } => target
                .paint_surface(surface, *bounds, clip)
                .map_err(|source| PaintError::Target { source }),
        }
    }
}

fn place_node<S, L>(measured: MeasuredNode<'_, S, L>, origin: GridPos) -> PlacedNode<'_, S, L> {
    let bounds = GridRect::new(origin, measured.size());
    match measured {
        MeasuredNode::Text {
            content, layout, ..
        } => PlacedNode::Text {
            content,
            layout,
            bounds,
        },
        MeasuredNode::Row { children, gaps, .. } => PlacedNode::Row {
            children: place_linear(children, gaps, origin, Axis::Horizontal),
            bounds,
        },
        MeasuredNode::Column { children, gaps, .. } => PlacedNode::Column {
            children: place_linear(children, gaps, origin, Axis::Vertical),
            bounds,
        },
        MeasuredNode::Border { child, .. } => PlacedNode::Border {
            child: Box::new(place_node(
                *child,
                offset(origin, bounds.size.cols.min(1), bounds.size.rows.min(1)),
            )),
            bounds,
        },
        MeasuredNode::Padding { insets, child, .. } => PlacedNode::Padding {
            child: Box::new(place_node(
                *child,
                offset(
                    origin,
                    insets.left.min(bounds.size.cols),
                    insets.top.min(bounds.size.rows),
                ),
            )),
            bounds,
        },
        MeasuredNode::Spacer { .. } => PlacedNode::Spacer { bounds },
        MeasuredNode::Align {
            horizontal,
            vertical,
            child,
            ..
        } => {
            let child_size = child.size();
            let col = alignment_offset(bounds.size.cols, child_size.cols, horizontal);
            let row = vertical_alignment_offset(bounds.size.rows, child_size.rows, vertical);
            PlacedNode::Align {
                child: Box::new(place_node(*child, offset(origin, col, row))),
                bounds,
            }
        }
        MeasuredNode::SurfaceView { surface, .. } => PlacedNode::SurfaceView { surface, bounds },
    }
}

fn place_linear<S, L>(
    children: Vec<MeasuredNode<'_, S, L>>,
    gaps: Vec<u32>,
    origin: GridPos,
    axis: Axis,
) -> Vec<PlacedNode<'_, S, L>> {
    let mut offset_main = 0;
    children
        .into_iter()
        .enumerate()
        .map(|(index, child)| {
            let child_main = main(child.size(), axis);
            let child_origin = match axis {
                Axis::Horizontal => offset(origin, offset_main, 0),
                Axis::Vertical => offset(origin, 0, offset_main),
            };
            offset_main += child_main;
            if let Some(gap) = gaps.get(index) {
                offset_main += gap;
            }
            place_node(child, child_origin)
        })
        .collect()
}

fn offset(origin: GridPos, cols: u32, rows: u32) -> GridPos {
    GridPos::new(origin.col + cols as i32, origin.row + rows as i32)
}

fn alignment_offset(outer: u32, inner: u32, alignment: HorizontalAlign) -> u32 {
    let remaining = outer.saturating_sub(inner);
    match alignment {
        HorizontalAlign::Start => 0,
        HorizontalAlign::Center => remaining / 2,
        HorizontalAlign::End => remaining,
    }
}

fn vertical_alignment_offset(outer: u32, inner: u32, alignment: VerticalAlign) -> u32 {
    let remaining = outer.saturating_sub(inner);
    match alignment {
        VerticalAlign::Start => 0,
        VerticalAlign::Center => remaining / 2,
        VerticalAlign::End => remaining,
    }
}
