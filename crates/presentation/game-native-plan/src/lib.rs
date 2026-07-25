//! Pure native asset and frame planning.

#![forbid(unsafe_code)]

use std::{collections::BTreeMap, error::Error, fmt};

use game_assets::{AssetKey, DecodedImage};
use game_view::{GameView, LayerKind, ViewCell};
use punctum_gpu::{
    GpuAtlas, GpuCell, GpuClip, GpuImage, GpuPixelImage, GpuPlanError, PixelOffset, PixelRect,
    PixelSize, ResourceId, Rgba8, SubmissionPlan, Viewport as GridViewport, plan_composite,
    plan_pixels,
};
use punctum_grid::{GridSize, Surface, SurfaceError};
use punctum_ui::{UiDrawCommand, UiFrame};

pub struct NativeAssets {
    atlas: GpuAtlas,
    resources: BTreeMap<AssetKey, ResourceId>,
}

impl NativeAssets {
    pub fn new(images: Vec<(AssetKey, DecodedImage)>) -> Result<Self, NativeAssetError> {
        let mut resources = BTreeMap::new();
        let mut numbered = Vec::with_capacity(images.len());
        for (index, (key, image)) in images.iter().enumerate() {
            let id = resource_id(index)?;
            if resources.insert(key.clone(), id).is_some() {
                return Err(NativeAssetError::DuplicateKey(key.clone()));
            }
            numbered.push((id, image));
        }
        let atlas = game_assets::build_atlas(&numbered)
            .map_err(|error| NativeAssetError::Atlas(error.to_string()))?;
        Ok(Self { atlas, resources })
    }

    pub fn resource(&self, key: &AssetKey) -> Option<ResourceId> {
        self.resources.get(key).copied()
    }

    pub const fn atlas_size(&self) -> PixelSize {
        self.atlas.size()
    }

    pub const fn atlas(&self) -> &GpuAtlas {
        &self.atlas
    }
}

fn resource_id(index: usize) -> Result<ResourceId, NativeAssetError> {
    Ok(ResourceId(
        u32::try_from(index)
            .map_err(|_| NativeAssetError::TooManyAssets)?
            .checked_add(1)
            .ok_or(NativeAssetError::TooManyAssets)?,
    ))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeAssetError {
    DuplicateKey(AssetKey),
    TooManyAssets,
    Atlas(String),
}

impl fmt::Display for NativeAssetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateKey(key) => write!(formatter, "duplicate asset key {}", key.as_str()),
            Self::TooManyAssets => formatter.write_str("native asset count exceeds u32"),
            Self::Atlas(message) => write!(formatter, "cannot build native atlas: {message}"),
        }
    }
}

impl Error for NativeAssetError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeTextLabel {
    pub col: u32,
    pub row: u32,
    pub width: u32,
    pub height: u32,
    pub content: String,
    pub color: Rgba8,
    /// Pixel UI supplies this value; Grid labels derive it from `TextScale`.
    pub font_size: Option<u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeTextBounds {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl NativeTextBounds {
    pub fn width(self) -> i32 {
        self.right.saturating_sub(self.left)
    }

    pub fn height(self) -> i32 {
        self.bottom.saturating_sub(self.top)
    }
}

pub fn text_bounds(
    label: &NativeTextLabel,
    viewport: GridViewport,
) -> Result<NativeTextBounds, std::num::TryFromIntError> {
    let left =
        i64::from(viewport.origin.x) + i64::from(label.col) * i64::from(viewport.cell_size.width);
    let top =
        i64::from(viewport.origin.y) + i64::from(label.row) * i64::from(viewport.cell_size.height);
    let right = left + i64::from(label.width) * i64::from(viewport.cell_size.width);
    let bottom = top + i64::from(label.height) * i64::from(viewport.cell_size.height);
    Ok(NativeTextBounds {
        left: i32::try_from(left)?,
        top: i32::try_from(top)?,
        right: i32::try_from(right)?,
        bottom: i32::try_from(bottom)?,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextScale {
    numerator: u32,
    denominator: u32,
    minimum: u32,
    maximum: u32,
}

impl TextScale {
    pub const fn new(numerator: u32, denominator: u32, minimum: u32, maximum: u32) -> Self {
        assert!(denominator > 0);
        assert!(minimum <= maximum);
        Self {
            numerator,
            denominator,
            minimum,
            maximum,
        }
    }

    pub fn font_size(self, cell_height: u32) -> f32 {
        (cell_height * self.numerator / self.denominator).clamp(self.minimum, self.maximum) as f32
    }
}

pub struct FramePass {
    gpu: SubmissionPlan,
    labels: Vec<NativeTextLabel>,
    text_scale: TextScale,
}

impl FramePass {
    pub fn viewport(&self) -> GridViewport {
        self.gpu.viewport
    }

    pub fn gpu(&self) -> &SubmissionPlan {
        &self.gpu
    }

    pub fn labels(&self) -> &[NativeTextLabel] {
        &self.labels
    }

    pub fn text_scale(&self) -> TextScale {
        self.text_scale
    }
}

pub struct FramePlan {
    passes: Vec<FramePass>,
}

impl FramePlan {
    /// Converts an already-resolved pixel UI frame at the adapter boundary.
    /// The UI crate stays independent from atlas IDs and GPU plans.
    pub fn from_ui_frame<Action>(
        frame: &UiFrame<Action>,
        assets: &NativeAssets,
        text_scale: TextScale,
    ) -> Result<Self, FramePlanError> {
        let white_key = AssetKey::from_resource_template("solid/white".into());
        let white = assets
            .resource(&white_key)
            .ok_or(FramePlanError::UnknownAsset(white_key))?;
        let mut images = Vec::new();
        let mut labels = Vec::new();
        for (z_index, command) in frame.commands().iter().enumerate() {
            match command {
                UiDrawCommand::Fill {
                    bounds,
                    color,
                    border_radius,
                    clip,
                } => {
                    if let Some(bounds) = ui_visible_bounds(*bounds, *clip) {
                        images.push(
                            GpuPixelImage::new(
                                bounds,
                                white,
                                Rgba8::new(color.red, color.green, color.blue, color.alpha),
                                z_index as i32,
                            )
                            .with_corner_radii(ui_corner_radii(*border_radius, bounds)),
                        );
                    }
                }
                UiDrawCommand::RadarChart {
                    bounds,
                    values,
                    max,
                    rings,
                    grid_color,
                    axis_color,
                    fill_color,
                    edge_color,
                    point_color,
                    label_color,
                    labels: chart_labels,
                    label_font_size,
                    clip,
                } => {
                    if let Some(visible) = ui_visible_bounds(*bounds, *clip) {
                        let (chart_images, chart_text) = radar_images(
                            *bounds,
                            visible,
                            white,
                            *values,
                            *max,
                            *rings,
                            ui_color(*grid_color),
                            ui_color(*axis_color),
                            ui_color(*fill_color),
                            ui_color(*edge_color),
                            ui_color(*point_color),
                            ui_color(*label_color),
                            chart_labels,
                            *label_font_size,
                            z_index as i32,
                        );
                        images.extend(chart_images);
                        labels.extend(chart_text);
                    }
                }
                UiDrawCommand::Image {
                    bounds,
                    content,
                    tint,
                    pixel_offset,
                    border_radius,
                    clip,
                    ..
                } => {
                    if let Some(bounds) = ui_visible_bounds(*bounds, *clip) {
                        let key = AssetKey::new(content.as_str()).map_err(|_| {
                            FramePlanError::InvalidUiContent(content.as_str().to_owned())
                        })?;
                        let resource = assets
                            .resource(&key)
                            .ok_or(FramePlanError::UnknownAsset(key))?;
                        images.push(
                            GpuPixelImage::new(
                                bounds,
                                resource,
                                Rgba8::new(tint.red, tint.green, tint.blue, tint.alpha),
                                z_index as i32,
                            )
                            .with_pixel_offset(PixelOffset::new(pixel_offset.x, pixel_offset.y))
                            .with_corner_radii(ui_corner_radii(*border_radius, bounds)),
                        );
                    }
                }
                UiDrawCommand::Text {
                    bounds,
                    content,
                    color,
                    font_size,
                    clip,
                    ..
                } => {
                    if let Some(bounds) = ui_visible_bounds(*bounds, *clip) {
                        labels.push(NativeTextLabel {
                            col: bounds.x,
                            row: bounds.y,
                            width: bounds.width,
                            height: bounds.height,
                            content: content.clone(),
                            color: Rgba8::new(color.red, color.green, color.blue, color.alpha),
                            font_size: Some(*font_size),
                        });
                    }
                }
                UiDrawCommand::Outline {
                    bounds,
                    color,
                    width,
                    clip,
                    ..
                } => {
                    if let Some(bounds) = ui_visible_bounds(*bounds, *clip) {
                        images.extend(outline_images(
                            bounds,
                            white,
                            *width,
                            Rgba8::new(color.red, color.green, color.blue, color.alpha),
                            z_index as i32,
                        ));
                    }
                }
                UiDrawCommand::Ripple {
                    bounds,
                    center,
                    radius,
                    color,
                    clip,
                } => {
                    if let Some(bounds) = ui_visible_bounds(*bounds, *clip) {
                        let center = ripple_center(*center, bounds)?;
                        images.push(
                            GpuPixelImage::new(
                                bounds,
                                white,
                                Rgba8::new(color.red, color.green, color.blue, color.alpha),
                                z_index as i32,
                            )
                            .with_circle(center, *radius),
                        );
                    }
                }
            }
        }
        Ok(Self::single(
            plan_pixels(
                &images,
                &assets.atlas,
                u32::MAX,
                PixelSize::new(frame.viewport().width, frame.viewport().height),
            )
            .map_err(FramePlanError::Gpu)?,
            labels,
            text_scale,
        ))
    }

    pub fn from_game_view(
        view: &GameView,
        assets: &NativeAssets,
        viewport: GridViewport,
        text_scale: TextScale,
    ) -> Result<Self, FramePlanError> {
        let size = view
            .layers()
            .iter()
            .find_map(|layer| layer.surface.as_ref().map(Surface::size))
            .ok_or(FramePlanError::MissingSurface)?;
        let mut cells = vec![GpuCell::Empty; (size.cols * size.rows) as usize];
        let white_key = AssetKey::from_resource_template("solid/white".into());
        let white = assets
            .resource(&white_key)
            .ok_or(FramePlanError::UnknownAsset(white_key))?;
        let mut images = Vec::new();
        let mut labels = Vec::new();
        for layer in view.layers() {
            if let Some(surface) = &layer.surface {
                if surface.size() != size {
                    return Err(FramePlanError::SurfaceSizeMismatch {
                        expected: size,
                        actual: surface.size(),
                    });
                }
                for (target, source) in cells.iter_mut().zip(surface.cells()) {
                    if let ViewCell::Fill(tint) = source {
                        *target = GpuCell::Sprite {
                            resource: white,
                            tint: *tint,
                        };
                    }
                }
            }
            let layer_z = match layer.kind {
                LayerKind::Map => 0,
                LayerKind::Character => 100,
                LayerKind::Hud => 200,
                LayerKind::Console => 300,
            };
            for image in &layer.images {
                let resource = assets
                    .resource(&image.asset)
                    .ok_or_else(|| FramePlanError::UnknownAsset(image.asset.clone()))?;
                images.push(
                    GpuImage::new(
                        image.bounds,
                        resource,
                        image.tint,
                        layer_z + i32::from(image.z_index),
                    )
                    .with_pixel_offset(image.pixel_offset),
                );
            }
            labels.extend(layer.labels.iter().map(|label| NativeTextLabel {
                col: label.col,
                row: label.row,
                width: label.width,
                height: label.height,
                content: label.content.clone(),
                color: label.color,
                font_size: None,
            }));
        }
        let surface = Surface::from_cells(size, cells).map_err(FramePlanError::Surface)?;
        Self::new(
            &surface,
            &images,
            &assets.atlas,
            u32::MAX,
            viewport,
            GpuClip::Surface,
            labels,
            text_scale,
        )
        .map_err(FramePlanError::Gpu)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        surface: &Surface<GpuCell>,
        images: &[GpuImage],
        atlas: &GpuAtlas,
        max_instances: u32,
        viewport: GridViewport,
        clip: GpuClip,
        labels: impl IntoIterator<Item = NativeTextLabel>,
        text_scale: TextScale,
    ) -> Result<Self, GpuPlanError> {
        Ok(Self::single(
            plan_composite(surface, images, atlas, max_instances, viewport, clip)?,
            labels.into_iter().collect(),
            text_scale,
        ))
    }

    fn single(gpu: SubmissionPlan, labels: Vec<NativeTextLabel>, text_scale: TextScale) -> Self {
        Self {
            passes: vec![FramePass {
                gpu,
                labels,
                text_scale,
            }],
        }
    }

    /// Keeps independent viewport mappings separate while rendering them in order.
    pub fn compose(mut base: Self, overlay: Self) -> Self {
        base.passes.extend(overlay.passes);
        base
    }

    pub fn passes(&self) -> &[FramePass] {
        &self.passes
    }

    pub fn viewport(&self) -> GridViewport {
        self.passes[0].viewport()
    }

    pub fn gpu(&self) -> &SubmissionPlan {
        self.passes[0].gpu()
    }

    pub fn labels(&self) -> &[NativeTextLabel] {
        self.passes[0].labels()
    }

    pub fn text_scale(&self) -> TextScale {
        self.passes[0].text_scale()
    }
}

fn ui_corner_radii(radius: punctum_ui::UiBorderRadius, bounds: PixelRect) -> [u32; 4] {
    let maximum = bounds.width.min(bounds.height) / 2;
    [
        radius.top_left.min(maximum),
        radius.top_right.min(maximum),
        radius.bottom_right.min(maximum),
        radius.bottom_left.min(maximum),
    ]
}

fn ui_color(color: punctum_ui::UiColor) -> Rgba8 {
    Rgba8::new(color.red, color.green, color.blue, color.alpha)
}

fn ui_visible_bounds(bounds: punctum_ui::UiRect, clip: punctum_ui::UiRect) -> Option<PixelRect> {
    bounds
        .intersect(clip)
        .map(|rect| PixelRect::new(rect.x, rect.y, rect.width, rect.height))
}

#[derive(Clone, Copy)]
struct RadarPoint {
    x: f32,
    y: f32,
}

fn radar_images(
    bounds: punctum_ui::UiRect,
    clip: PixelRect,
    resource: ResourceId,
    values: [u16; 6],
    max: u16,
    rings: u8,
    grid_color: Rgba8,
    axis_color: Rgba8,
    fill_color: Rgba8,
    edge_color: Rgba8,
    point_color: Rgba8,
    label_color: Rgba8,
    labels: &[String; 6],
    label_font_size: u32,
    z_index: i32,
) -> (Vec<GpuPixelImage>, Vec<NativeTextLabel>) {
    let width = bounds.width as f32;
    let height = bounds.height as f32;
    let center = RadarPoint {
        x: bounds.x as f32 + width / 2.0,
        y: bounds.y as f32 + height / 2.0,
    };
    let radius = (width.min(height) / 2.0 - 20.0).max(1.0);
    let outer = radar_points(center, radius, 1.0);
    let scale = if max == 0 { 0.0 } else { 1.0 / f32::from(max) };
    let data = std::array::from_fn(|index| {
        radar_point(
            center,
            radius,
            f32::from(values[index].min(max)) * scale,
            index,
        )
    });
    let mut images = Vec::new();
    if fill_color.alpha != 0 {
        fill_polygon(&mut images, &data, clip, resource, fill_color, z_index);
    }
    let ring_count = u32::from(rings.clamp(1, 8));
    for ring in 1..=ring_count {
        let ring_points = radar_points(center, radius, ring as f32 / ring_count as f32);
        for index in 0..6 {
            push_line(
                &mut images,
                ring_points[index],
                ring_points[(index + 1) % 6],
                clip,
                resource,
                grid_color,
                z_index,
            );
        }
    }
    for point in outer {
        push_line(
            &mut images,
            center,
            point,
            clip,
            resource,
            axis_color,
            z_index,
        );
    }
    for index in 0..6 {
        push_line(
            &mut images,
            data[index],
            data[(index + 1) % 6],
            clip,
            resource,
            edge_color,
            z_index,
        );
    }
    for point in data {
        push_point(&mut images, point, clip, resource, point_color, z_index);
    }
    let text = radar_labels(outer, bounds, clip, labels, label_color, label_font_size);
    (images, text)
}

fn radar_points(center: RadarPoint, radius: f32, scale: f32) -> [RadarPoint; 6] {
    std::array::from_fn(|index| radar_point(center, radius, scale, index))
}

fn radar_point(center: RadarPoint, radius: f32, scale: f32, index: usize) -> RadarPoint {
    let angle = -std::f32::consts::FRAC_PI_2 + index as f32 * std::f32::consts::PI / 3.0;
    RadarPoint {
        x: center.x + angle.cos() * radius * scale,
        y: center.y + angle.sin() * radius * scale,
    }
}

fn fill_polygon(
    images: &mut Vec<GpuPixelImage>,
    points: &[RadarPoint; 6],
    clip: PixelRect,
    resource: ResourceId,
    tint: Rgba8,
    z_index: i32,
) {
    let minimum_y = points
        .iter()
        .map(|point| point.y.floor() as i64)
        .min()
        .unwrap_or(0)
        .max(i64::from(clip.y));
    let maximum_y = points
        .iter()
        .map(|point| point.y.ceil() as i64)
        .max()
        .unwrap_or(0)
        .min(i64::from(
            clip.y.saturating_add(clip.height).saturating_sub(1),
        ));
    if minimum_y > maximum_y {
        return;
    }
    for scan_y in minimum_y..=maximum_y {
        let y = scan_y as f32 + 0.5;
        let mut intersections = Vec::with_capacity(6);
        for index in 0..6 {
            let first = points[index];
            let second = points[(index + 1) % 6];
            if (first.y <= y && second.y > y) || (second.y <= y && first.y > y) {
                let ratio = (y - first.y) / (second.y - first.y);
                intersections.push(first.x + ratio * (second.x - first.x));
            }
        }
        intersections.sort_by(|first, second| {
            first
                .partial_cmp(second)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        for pair in intersections.chunks_exact(2) {
            let left = pair[0].ceil() as i64;
            let right = pair[1].floor() as i64;
            let left = left.max(i64::from(clip.x));
            let right = right.min(i64::from(
                clip.x.saturating_add(clip.width).saturating_sub(1),
            ));
            if right >= left
                && let (Ok(x), Ok(y), Ok(width)) = (
                    u32::try_from(left),
                    u32::try_from(scan_y),
                    u32::try_from(right.saturating_sub(left).saturating_add(1)),
                )
            {
                images.push(GpuPixelImage::new(
                    PixelRect::new(x, y, width, 1),
                    resource,
                    tint,
                    z_index,
                ));
            }
        }
    }
}

fn push_line(
    images: &mut Vec<GpuPixelImage>,
    first: RadarPoint,
    second: RadarPoint,
    clip: PixelRect,
    resource: ResourceId,
    tint: Rgba8,
    z_index: i32,
) {
    let Some((mut x0, mut y0)) = radar_pixel(first) else {
        return;
    };
    let Some((x1, y1)) = radar_pixel(second) else {
        return;
    };
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        push_pixel(images, x0, y0, clip, resource, tint, z_index);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let double_error = error.saturating_mul(2);
        if double_error >= dy {
            error += dy;
            x0 += sx;
        }
        if double_error <= dx {
            error += dx;
            y0 += sy;
        }
    }
}

fn push_point(
    images: &mut Vec<GpuPixelImage>,
    point: RadarPoint,
    clip: PixelRect,
    resource: ResourceId,
    tint: Rgba8,
    z_index: i32,
) {
    let Some((x, y)) = radar_pixel(point) else {
        return;
    };
    for offset_y in -2_i32..=2 {
        for offset_x in -2_i32..=2 {
            if offset_x.abs() + offset_y.abs() <= 3 {
                push_pixel(
                    images,
                    x + offset_x,
                    y + offset_y,
                    clip,
                    resource,
                    tint,
                    z_index,
                );
            }
        }
    }
}

fn radar_pixel(point: RadarPoint) -> Option<(i32, i32)> {
    if !point.x.is_finite() || !point.y.is_finite() {
        return None;
    }
    Some((point.x.round() as i32, point.y.round() as i32))
}

fn push_pixel(
    images: &mut Vec<GpuPixelImage>,
    x: i32,
    y: i32,
    clip: PixelRect,
    resource: ResourceId,
    tint: Rgba8,
    z_index: i32,
) {
    let Ok(x) = u32::try_from(x) else {
        return;
    };
    let Ok(y) = u32::try_from(y) else {
        return;
    };
    if x < clip.x
        || y < clip.y
        || x >= clip.x.saturating_add(clip.width)
        || y >= clip.y.saturating_add(clip.height)
        || tint.alpha == 0
    {
        return;
    }
    images.push(GpuPixelImage::new(
        PixelRect::new(x, y, 1, 1),
        resource,
        tint,
        z_index,
    ));
}

fn radar_labels(
    points: [RadarPoint; 6],
    bounds: punctum_ui::UiRect,
    clip: PixelRect,
    labels: &[String; 6],
    color: Rgba8,
    font_size: u32,
) -> Vec<NativeTextLabel> {
    let width = 34_u32;
    let height = font_size.saturating_add(4).max(12);
    points
        .into_iter()
        .zip(labels)
        .filter_map(|(point, label)| {
            let x = (point.x.round() as i64).saturating_sub(i64::from(width) / 2);
            let y = (point.y.round() as i64).saturating_sub(i64::from(height) / 2);
            let x = x.clamp(
                i64::from(bounds.x),
                i64::from(bounds.x.saturating_add(bounds.width.saturating_sub(width))),
            );
            let y = y.clamp(
                i64::from(bounds.y),
                i64::from(
                    bounds
                        .y
                        .saturating_add(bounds.height.saturating_sub(height)),
                ),
            );
            let Ok(x) = u32::try_from(x) else {
                return None;
            };
            let Ok(y) = u32::try_from(y) else {
                return None;
            };
            let label_bounds = PixelRect::new(x, y, width, height);
            let visible = label_bounds.x.max(clip.x).checked_add(0).and_then(|left| {
                let right = label_bounds
                    .x
                    .saturating_add(label_bounds.width)
                    .min(clip.x.saturating_add(clip.width));
                (right > left).then_some((left, right))
            })?;
            let top = label_bounds.y.max(clip.y);
            let bottom = label_bounds
                .y
                .saturating_add(label_bounds.height)
                .min(clip.y.saturating_add(clip.height));
            if bottom <= top {
                return None;
            }
            Some(NativeTextLabel {
                col: visible.0,
                row: top,
                width: visible.1.saturating_sub(visible.0),
                height: bottom.saturating_sub(top),
                content: label.clone(),
                color,
                font_size: Some(font_size),
            })
        })
        .collect()
}

fn ripple_center(
    center: punctum_ui::UiPixelOffset,
    bounds: PixelRect,
) -> Result<PixelOffset, FramePlanError> {
    let x = i64::from(center.x) - i64::from(bounds.x);
    let y = i64::from(center.y) - i64::from(bounds.y);
    Ok(PixelOffset::new(
        i32::try_from(x).map_err(|_| FramePlanError::InvalidRippleCenter)?,
        i32::try_from(y).map_err(|_| FramePlanError::InvalidRippleCenter)?,
    ))
}

fn outline_images(
    bounds: PixelRect,
    resource: ResourceId,
    width: u32,
    tint: Rgba8,
    z_index: i32,
) -> Vec<GpuPixelImage> {
    let width = width.min(bounds.width.min(bounds.height));
    if width == 0 {
        return Vec::new();
    }
    let right_x = bounds.x.saturating_add(bounds.width.saturating_sub(width));
    let bottom_y = bounds.y.saturating_add(bounds.height.saturating_sub(width));
    [
        PixelRect::new(bounds.x, bounds.y, bounds.width, width),
        PixelRect::new(bounds.x, bottom_y, bounds.width, width),
        PixelRect::new(bounds.x, bounds.y, width, bounds.height),
        PixelRect::new(right_x, bounds.y, width, bounds.height),
    ]
    .into_iter()
    .map(|bounds| GpuPixelImage::new(bounds, resource, tint, z_index))
    .collect()
}

#[derive(Debug)]
pub enum FramePlanError {
    MissingSurface,
    SurfaceSizeMismatch {
        expected: GridSize,
        actual: GridSize,
    },
    UnknownAsset(AssetKey),
    InvalidUiContent(String),
    InvalidRippleCenter,
    Surface(SurfaceError),
    Gpu(GpuPlanError),
}

impl fmt::Display for FramePlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSurface => formatter.write_str("product frame has no grid surface"),
            Self::SurfaceSizeMismatch { expected, actual } => write!(
                formatter,
                "product layer surface {actual:?} does not match {expected:?}"
            ),
            Self::UnknownAsset(key) => write!(formatter, "unknown asset key {}", key.as_str()),
            Self::InvalidUiContent(content) => {
                write!(formatter, "invalid UI content key {content}")
            }
            Self::InvalidRippleCenter => formatter.write_str("UI ripple center is out of range"),
            Self::Surface(error) => write!(formatter, "cannot build product surface: {error}"),
            Self::Gpu(error) => write!(formatter, "cannot plan product frame: {error}"),
        }
    }
}

impl Error for FramePlanError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Surface(error) => Some(error),
            Self::Gpu(error) => Some(error),
            Self::MissingSurface
            | Self::SurfaceSizeMismatch { .. }
            | Self::UnknownAsset(_)
            | Self::InvalidUiContent(_)
            | Self::InvalidRippleCenter => None,
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
