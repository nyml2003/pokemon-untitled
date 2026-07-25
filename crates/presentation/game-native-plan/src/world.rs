use game_view::{GameView, LayerKind, ViewCell};
use punctum_gpu::{
    GpuCell, GpuClip, GpuImage, SubmissionPlan, Viewport as GridViewport, plan_composite,
};
use punctum_grid::Surface;

use crate::{NativeAssets, NativeTextLabel, error::FramePlanError};

pub(crate) fn plan_game_view(
    view: &GameView,
    assets: &NativeAssets,
    viewport: GridViewport,
) -> Result<(SubmissionPlan, Vec<NativeTextLabel>), FramePlanError> {
    let size = view
        .layers()
        .iter()
        .find_map(|layer| layer.surface.as_ref().map(Surface::size))
        .ok_or(FramePlanError::MissingSurface)?;
    let mut cells = vec![GpuCell::Empty; (size.cols * size.rows) as usize];
    let white_key = game_assets::AssetKey::from_resource_template("solid/white".into());
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
    let gpu = plan_composite(
        &surface,
        &images,
        &assets.atlas,
        u32::MAX,
        viewport,
        GpuClip::Surface,
    )
    .map_err(FramePlanError::Gpu)?;
    Ok((gpu, labels))
}
