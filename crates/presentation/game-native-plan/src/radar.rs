use punctum_gpu::{GpuPixelImage, PixelRect, ResourceId, Rgba8};
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

pub(crate) fn radar_images(plan: RadarPlan<'_>) -> (Vec<GpuPixelImage>, Vec<NativeTextLabel>) {
    let width = plan.bounds.width as f32;
    let height = plan.bounds.height as f32;
    let center = RadarPoint {
        x: plan.bounds.x as f32 + width / 2.0,
        y: plan.bounds.y as f32 + height / 2.0,
    };
    let radius = (width.min(height) / 2.0 - 20.0).max(1.0);
    let outer = radar_points(center, radius, 1.0);
    let scale = if plan.max == 0 {
        0.0
    } else {
        1.0 / f32::from(plan.max)
    };
    let data = std::array::from_fn(|index| {
        radar_point(
            center,
            radius,
            f32::from(plan.values[index].min(plan.max)) * scale,
            index,
        )
    });
    let mut images = Vec::new();
    if plan.fill_color.alpha != 0 {
        fill_polygon(
            &mut images,
            &data,
            plan.clip,
            plan.resource,
            plan.fill_color,
            plan.z_index,
        );
    }
    let ring_count = u32::from(plan.rings.clamp(1, 8));
    for ring in 1..=ring_count {
        let ring_points = radar_points(center, radius, ring as f32 / ring_count as f32);
        for index in 0..6 {
            push_line(
                &mut images,
                ring_points[index],
                ring_points[(index + 1) % 6],
                plan.clip,
                plan.resource,
                plan.grid_color,
                plan.z_index,
            );
        }
    }
    for point in outer {
        push_line(
            &mut images,
            center,
            point,
            plan.clip,
            plan.resource,
            plan.axis_color,
            plan.z_index,
        );
    }
    for index in 0..6 {
        push_line(
            &mut images,
            data[index],
            data[(index + 1) % 6],
            plan.clip,
            plan.resource,
            plan.edge_color,
            plan.z_index,
        );
    }
    for point in data {
        push_point(
            &mut images,
            point,
            plan.clip,
            plan.resource,
            plan.point_color,
            plan.z_index,
        );
    }
    let text = radar_labels(
        outer,
        plan.bounds,
        plan.clip,
        plan.labels,
        plan.label_color,
        plan.label_font_size,
    );
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
