//! Discrete geometry, dense surfaces, and bounded frame patches.

#![forbid(unsafe_code)]

mod geometry;
mod patch;
mod surface;

pub use geometry::{GridPos, GridRect, GridSize};
pub use patch::{Patch, PatchApplyError, PatchKind, PatchSpan, apply_patch, diff};
pub use surface::{Surface, SurfaceError};
