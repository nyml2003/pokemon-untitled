//! Pure GPU atlas, viewport, submission planning, and byte encoding.

#![forbid(unsafe_code)]

mod encoding;
mod model;
mod plan;

pub use encoding::{
    RADAR_INSTANCE_STRIDE, UNIFORM_SIZE, encode_instances, encode_radar_instances, encode_uniform,
};
pub use model::{
    GpuAtlas, GpuAtlasError, GpuCell, GpuClip, GpuImage, GpuPixelImage, GpuResource, PixelOffset,
    PixelRect, PixelSize, RadarInstanceData, ResourceId, Rgba8, Viewport, ViewportError,
};
pub use plan::{
    GpuPlanError, INSTANCE_STRIDE, InstanceData, InstanceUpload, RadarUpload, SubmissionMode,
    SubmissionPlan, plan_composite, plan_patch, plan_pixels, plan_surface,
};
