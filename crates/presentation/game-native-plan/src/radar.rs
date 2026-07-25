use punctum_gpu::{GpuPixelImage, PixelRect, RadarInstanceData, ResourceId, Rgba8};
use punctum_ui::UiRect;

use crate::NativeTextLabel;

#[derive(Clone, Copy)]
struct RadarPoint {
    x: f32,
    y: f32,
}

pub(crate) struct RadarPlan<'a> {
    pub(crate) bounds: UiRect,
    pub(crate) clip: PixelRect,
    pub(crate) resource: ResourceId,
    pub(crate) values: [u16; 6],
    pub(crate) max: u16,
    pub(crate) rings: u8,
    pub(crate) grid_color: Rgba8,
    pub(crate) axis_color: Rgba8,
    pub(crate) fill_color: Rgba8,
    pub(crate) edge_color: Rgba8,
    pub(crate) point_color: Rgba8,
    pub(crate) label_color: Rgba8,
    pub(crate) labels: &'a [String; 6],
    pub(crate) label_font_size: u32,
    pub(crate) z_index: i32,
}

pub(crate) fn radar_image(plan: RadarPlan<'_>) -> (GpuPixelImage, Vec<NativeTextLabel>) {
    let center = RadarPoint {
        x: plan.bounds.x as f32 + plan.bounds.width as f32 / 2.0,
        y: plan.bounds.y as f32 + plan.bounds.height as f32 / 2.0,
    };
    let radius = (plan.bounds.width.min(plan.bounds.height) as f32 / 2.0 - 20.0).max(1.0);
    let outer = radar_points(center, radius, 1.0);
    let radar = RadarInstanceData::new(
        plan.values,
        plan.max,
        plan.rings.clamp(1, 8),
        [
            plan.grid_color,
            plan.axis_color,
            plan.fill_color,
            plan.edge_color,
            plan.point_color,
        ],
    )
    .with_bounds(PixelRect::new(
        plan.bounds.x,
        plan.bounds.y,
        plan.bounds.width,
        plan.bounds.height,
    ));
    let image = GpuPixelImage::new(
        plan.clip,
        plan.resource,
        Rgba8::new(255, 255, 255, 255),
        plan.z_index,
    )
    .with_radar(radar);
    let labels = radar_labels(
        outer,
        plan.bounds,
        plan.clip,
        plan.labels,
        plan.label_color,
        plan.label_font_size,
    );
    (image, labels)
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

fn radar_labels(
    points: [RadarPoint; 6],
    bounds: UiRect,
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
            let left = label_bounds.x.max(clip.x);
            let right = label_bounds
                .x
                .saturating_add(label_bounds.width)
                .min(clip.x.saturating_add(clip.width));
            if right <= left {
                return None;
            }
            let top = label_bounds.y.max(clip.y);
            let bottom = label_bounds
                .y
                .saturating_add(label_bounds.height)
                .min(clip.y.saturating_add(clip.height));
            if bottom <= top {
                return None;
            }
            Some(NativeTextLabel {
                col: left,
                row: top,
                width: right.saturating_sub(left),
                height: bottom.saturating_sub(top),
                content: label.clone(),
                color,
                font_size: Some(font_size),
            })
        })
        .collect()
}
