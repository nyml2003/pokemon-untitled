use std::{error::Error, fmt};

use punctum_grid::{GridPos, GridRect, GridSize, Patch, PatchKind, Surface};

use crate::{
    GpuAtlas, GpuCell, GpuClip, GpuImage, GpuPixelImage, PixelOffset, PixelRect, PixelSize,
    ResourceId, Viewport,
};

pub const INSTANCE_STRIDE: u64 = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmissionMode {
    Replace,
    Delta,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InstanceData {
    pub grid_position: [u32; 2],
    pub grid_span: [u32; 2],
    pub pixel_offset: [i32; 2],
    pub atlas_rect: [u32; 4],
    pub tint: [u8; 4],
    pub visible: u32,
    /// Top-left, top-right, bottom-right, bottom-left, in physical pixels.
    pub corner_radii: [u32; 4],
}

pub fn plan_composite(
    surface: &Surface<GpuCell>,
    images: &[GpuImage],
    atlas: &GpuAtlas,
    max_instances: u32,
    viewport: Viewport,
    clip: GpuClip,
) -> Result<SubmissionPlan, GpuPlanError> {
    let size = surface.size();
    let surface_count = checked_instance_count(size, max_instances)?;
    let image_count =
        u32::try_from(images.len()).map_err(|_| GpuPlanError::CompositeInstanceCountOverflow {
            surface: size,
            images: images.len(),
            maximum: max_instances,
        })?;
    let instance_count = surface_count
        .checked_add(image_count)
        .filter(|count| *count <= max_instances)
        .ok_or(GpuPlanError::CompositeInstanceCountOverflow {
            surface: size,
            images: images.len(),
            maximum: max_instances,
        })?;

    let mut instances = Vec::with_capacity(instance_count as usize);
    for (index, cell) in surface.cells().iter().enumerate() {
        let col = index as u32 % size.cols;
        let row = index as u32 / size.cols;
        instances.push(plan_cell(
            GridPos::new(col as i32, row as i32),
            cell,
            atlas,
        )?);
    }

    let mut ordered_images: Vec<_> = images.iter().enumerate().collect();
    ordered_images.sort_by_key(|(index, image)| (image.z_index, *index));
    for (_, image) in ordered_images {
        if image.bounds.size.is_empty() || !rect_fits_grid(image.bounds, size) {
            return Err(GpuPlanError::ImageOutOfBounds {
                bounds: image.bounds,
                grid_size: size,
            });
        }
        let rect = atlas
            .resource(image.resource)
            .ok_or(GpuPlanError::MissingResource {
                position: image.bounds.origin,
                resource: image.resource,
            })?;
        instances.push(InstanceData {
            grid_position: [
                image.bounds.origin.col as u32,
                image.bounds.origin.row as u32,
            ],
            grid_span: [image.bounds.size.cols, image.bounds.size.rows],
            pixel_offset: [image.pixel_offset.x, image.pixel_offset.y],
            atlas_rect: [rect.x, rect.y, rect.width, rect.height],
            tint: image.tint.to_array(),
            visible: 1,
            corner_radii: [0; 4],
        });
    }

    let uploads = if instances.is_empty() {
        Vec::new()
    } else {
        vec![InstanceUpload {
            first_slot: 0,
            instances,
        }]
    };
    Ok(SubmissionPlan {
        grid_size: size,
        mode: SubmissionMode::Replace,
        viewport,
        scissor: plan_scissor(size, viewport, clip),
        instance_count,
        uploads,
    })
}

/// Plans standalone pixel rectangles without allocating a grid surface.
///
/// The existing GPU shader is reused with one target pixel per logical cell.
/// This keeps the pixel path independent from `Surface` allocation while retaining
/// the atlas, tint, and instance encoding contracts.
pub fn plan_pixels(
    images: &[GpuPixelImage],
    atlas: &GpuAtlas,
    max_instances: u32,
    target_size: PixelSize,
) -> Result<SubmissionPlan, GpuPlanError> {
    let instance_count =
        u32::try_from(images.len()).map_err(|_| GpuPlanError::PixelInstanceCountOverflow {
            images: images.len(),
            maximum: max_instances,
        })?;
    if instance_count > max_instances {
        return Err(GpuPlanError::PixelInstanceCountOverflow {
            images: images.len(),
            maximum: max_instances,
        });
    }
    let viewport = Viewport::new(target_size, PixelOffset::new(0, 0), PixelSize::new(1, 1))
        .map_err(|_| GpuPlanError::InvalidPixelViewport { target_size })?;
    let grid_size = GridSize::new(target_size.width, target_size.height);
    let mut ordered: Vec<_> = images.iter().enumerate().collect();
    ordered.sort_by_key(|(index, image)| (image.z_index, *index));
    let mut instances = Vec::with_capacity(images.len());
    for (_, image) in ordered {
        if image.bounds.size().is_empty() || !pixel_rect_fits(image.bounds, target_size) {
            return Err(GpuPlanError::PixelImageOutOfBounds {
                bounds: image.bounds,
                target_size,
            });
        }
        let rect = atlas
            .resource(image.resource)
            .ok_or(GpuPlanError::MissingPixelResource {
                bounds: image.bounds,
                resource: image.resource,
            })?;
        instances.push(InstanceData {
            grid_position: [image.bounds.x, image.bounds.y],
            grid_span: [image.bounds.width, image.bounds.height],
            pixel_offset: [image.pixel_offset.x, image.pixel_offset.y],
            atlas_rect: [rect.x, rect.y, rect.width, rect.height],
            tint: image.tint.to_array(),
            visible: 1,
            corner_radii: image.corner_radii,
        });
    }
    let uploads = (!instances.is_empty())
        .then_some(InstanceUpload {
            first_slot: 0,
            instances,
        })
        .into_iter()
        .collect();
    Ok(SubmissionPlan {
        grid_size,
        mode: SubmissionMode::Replace,
        viewport,
        scissor: (!target_size.is_empty()).then_some(PixelRect::new(
            0,
            0,
            target_size.width,
            target_size.height,
        )),
        instance_count,
        uploads,
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstanceUpload {
    pub first_slot: u32,
    pub instances: Vec<InstanceData>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubmissionPlan {
    pub grid_size: GridSize,
    pub mode: SubmissionMode,
    pub viewport: Viewport,
    pub scissor: Option<PixelRect>,
    pub instance_count: u32,
    pub uploads: Vec<InstanceUpload>,
}

pub fn plan_surface(
    surface: &Surface<GpuCell>,
    atlas: &GpuAtlas,
    max_instances: u32,
    viewport: Viewport,
    clip: GpuClip,
) -> Result<SubmissionPlan, GpuPlanError> {
    let size = surface.size();
    let instance_count = checked_instance_count(size, max_instances)?;
    let uploads = if surface.cells().is_empty() {
        Vec::new()
    } else {
        let mut instances = Vec::with_capacity(surface.cells().len());
        for (index, cell) in surface.cells().iter().enumerate() {
            let col = index as u32 % size.cols;
            let row = index as u32 / size.cols;
            instances.push(plan_cell(
                GridPos::new(col as i32, row as i32),
                cell,
                atlas,
            )?);
        }
        vec![InstanceUpload {
            first_slot: 0,
            instances,
        }]
    };

    Ok(SubmissionPlan {
        grid_size: size,
        mode: SubmissionMode::Replace,
        viewport,
        scissor: plan_scissor(size, viewport, clip),
        instance_count,
        uploads,
    })
}

pub fn plan_patch(
    patch: &Patch<GpuCell>,
    atlas: &GpuAtlas,
    max_instances: u32,
    viewport: Viewport,
    clip: GpuClip,
) -> Result<SubmissionPlan, GpuPlanError> {
    let size = patch.size();
    let instance_count = checked_instance_count(size, max_instances)?;
    let mut uploads = Vec::with_capacity(patch.spans().len());

    for span in patch.spans() {
        let first_slot = u64::from(span.row()) * u64::from(size.cols) + u64::from(span.start_col());
        let mut instances = Vec::with_capacity(span.cells().len());
        for (offset, cell) in span.cells().iter().enumerate() {
            let col = u64::from(span.start_col()) + offset as u64;
            let position = GridPos::new(col as i32, span.row() as i32);
            instances.push(plan_cell(position, cell, atlas)?);
        }
        uploads.push(InstanceUpload {
            first_slot: first_slot as u32,
            instances,
        });
    }

    Ok(SubmissionPlan {
        grid_size: size,
        mode: match patch.kind() {
            PatchKind::Replace => SubmissionMode::Replace,
            PatchKind::Delta => SubmissionMode::Delta,
        },
        viewport,
        scissor: plan_scissor(size, viewport, clip),
        instance_count,
        uploads,
    })
}

fn checked_instance_count(size: GridSize, maximum: u32) -> Result<u32, GpuPlanError> {
    let count = u64::from(size.cols) * u64::from(size.rows);
    if count > u64::from(maximum) {
        return Err(GpuPlanError::InstanceCountOverflow { size, maximum });
    }
    Ok(count as u32)
}

fn plan_cell(
    position: GridPos,
    cell: &GpuCell,
    atlas: &GpuAtlas,
) -> Result<InstanceData, GpuPlanError> {
    let grid_position = [position.col as u32, position.row as u32];
    match *cell {
        GpuCell::Empty => Ok(InstanceData {
            grid_position,
            grid_span: [1, 1],
            pixel_offset: [0, 0],
            atlas_rect: [0; 4],
            tint: [0; 4],
            visible: 0,
            corner_radii: [0; 4],
        }),
        GpuCell::Sprite { resource, tint } => {
            let rect = atlas
                .resource(resource)
                .ok_or(GpuPlanError::MissingResource { position, resource })?;
            Ok(InstanceData {
                grid_position,
                grid_span: [1, 1],
                pixel_offset: [0, 0],
                atlas_rect: [rect.x, rect.y, rect.width, rect.height],
                tint: tint.to_array(),
                visible: 1,
                corner_radii: [0; 4],
            })
        }
    }
}

fn rect_fits_grid(rect: GridRect, size: GridSize) -> bool {
    let right = i64::from(rect.origin.col) + i64::from(rect.size.cols);
    let bottom = i64::from(rect.origin.row) + i64::from(rect.size.rows);
    rect.origin.col >= 0
        && rect.origin.row >= 0
        && right <= i64::from(size.cols)
        && bottom <= i64::from(size.rows)
}

fn pixel_rect_fits(rect: PixelRect, size: PixelSize) -> bool {
    u64::from(rect.x) + u64::from(rect.width) <= u64::from(size.width)
        && u64::from(rect.y) + u64::from(rect.height) <= u64::from(size.height)
}

fn plan_scissor(size: GridSize, viewport: Viewport, clip: GpuClip) -> Option<PixelRect> {
    if viewport.target_size.is_empty() {
        return None;
    }

    let requested = match clip {
        GpuClip::Surface => GridRect::new(GridPos::new(0, 0), size),
        GpuClip::Rect(rect) => rect,
    };
    let clipped = requested.clip_to(size)?;

    let left = i128::from(viewport.origin.x)
        + i128::from(clipped.origin.col) * i128::from(viewport.cell_size.width);
    let top = i128::from(viewport.origin.y)
        + i128::from(clipped.origin.row) * i128::from(viewport.cell_size.height);
    let right = left + i128::from(clipped.size.cols) * i128::from(viewport.cell_size.width);
    let bottom = top + i128::from(clipped.size.rows) * i128::from(viewport.cell_size.height);

    let target_width = i128::from(viewport.target_size.width);
    let target_height = i128::from(viewport.target_size.height);
    let x0 = left.clamp(0, target_width);
    let y0 = top.clamp(0, target_height);
    let x1 = right.clamp(0, target_width);
    let y1 = bottom.clamp(0, target_height);

    if x0 >= x1 || y0 >= y1 {
        return None;
    }

    Some(PixelRect::new(
        x0 as u32,
        y0 as u32,
        (x1 - x0) as u32,
        (y1 - y0) as u32,
    ))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuPlanError {
    InstanceCountOverflow {
        size: GridSize,
        maximum: u32,
    },
    CompositeInstanceCountOverflow {
        surface: GridSize,
        images: usize,
        maximum: u32,
    },
    ImageOutOfBounds {
        bounds: GridRect,
        grid_size: GridSize,
    },
    MissingResource {
        position: GridPos,
        resource: ResourceId,
    },
    PixelInstanceCountOverflow {
        images: usize,
        maximum: u32,
    },
    InvalidPixelViewport {
        target_size: PixelSize,
    },
    PixelImageOutOfBounds {
        bounds: PixelRect,
        target_size: PixelSize,
    },
    MissingPixelResource {
        bounds: PixelRect,
        resource: ResourceId,
    },
}

impl fmt::Display for GpuPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InstanceCountOverflow { size, maximum } => {
                write!(
                    formatter,
                    "grid {size:?} exceeds the GPU instance limit {maximum}"
                )
            }
            Self::CompositeInstanceCountOverflow {
                surface,
                images,
                maximum,
            } => write!(
                formatter,
                "grid {surface:?} plus {images} images exceeds the GPU instance limit {maximum}"
            ),
            Self::ImageOutOfBounds { bounds, grid_size } => write!(
                formatter,
                "image bounds {bounds:?} are empty or outside grid {grid_size:?}"
            ),
            Self::MissingResource { position, resource } => write!(
                formatter,
                "GPU resource {resource:?} is missing for cell {position:?}"
            ),
            Self::PixelInstanceCountOverflow { images, maximum } => write!(
                formatter,
                "{images} pixel images exceeds the GPU instance limit {maximum}"
            ),
            Self::InvalidPixelViewport { target_size } => {
                write!(
                    formatter,
                    "cannot create a pixel viewport for {target_size:?}"
                )
            }
            Self::PixelImageOutOfBounds {
                bounds,
                target_size,
            } => write!(
                formatter,
                "pixel image {bounds:?} is empty or outside target {target_size:?}"
            ),
            Self::MissingPixelResource { bounds, resource } => write!(
                formatter,
                "GPU resource {resource:?} is missing for pixel image {bounds:?}"
            ),
        }
    }
}

impl Error for GpuPlanError {}
