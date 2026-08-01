use game_assets::AssetKey;
use punctum_gpu::{
    GpuPixelImage, PixelOffset, PixelRect, PixelSize, ResourceId, Rgba8, SubmissionPlan,
    plan_pixels,
};
use punctum_ui::{UiDrawCommand, UiFrame, UiRect};

use crate::{
    NativeAssets, NativeTextLabel,
    error::FramePlanError,
    radar::{RadarPlan, radar_image},
};

pub(crate) fn plan_ui_frame<Action>(
    frame: &UiFrame<Action>,
    assets: &NativeAssets,
) -> Result<(SubmissionPlan, Vec<NativeTextLabel>), FramePlanError> {
    let white_key = AssetKey::from_resource_template("solid/white".into());
    let white = assets
        .resource(&white_key)
        .ok_or(FramePlanError::UnknownAsset(white_key))?;
    let mut images = Vec::new();
    let mut labels = Vec::new();
    for (z_index, command) in frame.commands().iter().enumerate() {
        match command {
            UiDrawCommand::Weather {
                bounds,
                pattern,
                frame,
                color,
                clip,
            } => {
                if let Some(bounds) = ui_visible_bounds(*bounds, *clip) {
                    images.push(
                        GpuPixelImage::new(bounds, white, ui_color(*color), z_index as i32)
                            .with_weather(*pattern, *frame),
                    );
                }
            }
            UiDrawCommand::Shadow {
                bounds,
                color,
                border_radius,
                clip,
            } => {
                if let Some(bounds) = ui_visible_bounds(*bounds, *clip) {
                    images.extend(shadow_images(
                        bounds,
                        *border_radius,
                        white,
                        ui_color(*color),
                        z_index as i32,
                    ));
                }
            }
            UiDrawCommand::Fill {
                bounds,
                color,
                border_radius,
                clip,
            } => {
                if let Some(bounds) = ui_visible_bounds(*bounds, *clip) {
                    images.push(
                        GpuPixelImage::new(bounds, white, ui_color(*color), z_index as i32)
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
                    let (chart_image, chart_text) = radar_image(RadarPlan {
                        bounds: *bounds,
                        clip: visible,
                        resource: white,
                        values: *values,
                        max: *max,
                        rings: *rings,
                        grid_color: ui_color(*grid_color),
                        axis_color: ui_color(*axis_color),
                        fill_color: ui_color(*fill_color),
                        edge_color: ui_color(*edge_color),
                        point_color: ui_color(*point_color),
                        label_color: ui_color(*label_color),
                        labels: chart_labels,
                        label_font_size: *label_font_size,
                        z_index: z_index as i32,
                    });
                    images.push(chart_image);
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
                        GpuPixelImage::new(bounds, resource, ui_color(*tint), z_index as i32)
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
                        color: ui_color(*color),
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
                        ui_color(*color),
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
                        GpuPixelImage::new(bounds, white, ui_color(*color), z_index as i32)
                            .with_circle(center, *radius),
                    );
                }
            }
        }
    }
    let gpu = plan_pixels(
        &images,
        &assets.atlas,
        u32::MAX,
        PixelSize::new(frame.viewport().width, frame.viewport().height),
    )
    .map_err(FramePlanError::Gpu)?;
    Ok((gpu, labels))
}

fn ui_color(color: punctum_ui::UiColor) -> Rgba8 {
    Rgba8::new(color.red, color.green, color.blue, color.alpha)
}

/// 将阴影矩形渲染为三圈同心的半透明矩形，从内到外透明度递减以近似光晕。
fn shadow_images(
    bounds: PixelRect,
    border_radius: punctum_ui::UiBorderRadius,
    resource: ResourceId,
    color: Rgba8,
    z_index: i32,
) -> Vec<GpuPixelImage> {
    (0..3)
        .map(|ring| {
            let inset = 2u32.saturating_sub(ring);
            let ring_bounds = PixelRect::new(
                bounds.x.saturating_add(inset),
                bounds.y.saturating_add(inset),
                bounds.width.saturating_sub(inset * 2),
                bounds.height.saturating_sub(inset * 2),
            );
            let alpha = (u32::from(color.alpha) / 3 * (ring + 1)).min(255) as u8;
            GpuPixelImage::new(
                ring_bounds,
                resource,
                Rgba8::new(color.red, color.green, color.blue, alpha),
                z_index,
            )
            .with_corner_radii(ui_corner_radii(border_radius, ring_bounds))
        })
        .collect()
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

fn ui_visible_bounds(bounds: UiRect, clip: UiRect) -> Option<PixelRect> {
    bounds
        .intersect(clip)
        .map(|rect| PixelRect::new(rect.x, rect.y, rect.width, rect.height))
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
