//! Pure GPU atlas, viewport, submission planning, and byte encoding.

#![forbid(unsafe_code)]

mod encoding;
mod model;
mod plan;

pub use encoding::{UNIFORM_SIZE, encode_instances, encode_uniform};
pub use model::{
    GpuAtlas, GpuAtlasError, GpuCell, GpuClip, GpuImage, GpuResource, PixelOffset, PixelRect,
    PixelSize, ResourceId, Rgba8, Viewport, ViewportError,
};
pub use plan::{
    GpuPlanError, INSTANCE_STRIDE, InstanceData, InstanceUpload, SubmissionMode, SubmissionPlan,
    plan_composite, plan_patch, plan_surface,
};
