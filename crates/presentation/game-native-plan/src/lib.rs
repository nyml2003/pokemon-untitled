//! Pure native asset and frame planning.

#![forbid(unsafe_code)]

mod assets;
mod error;
mod frame;
mod radar;
mod text;
mod ui;
mod world;

pub use assets::{NativeAssetError, NativeAssets};
pub use error::FramePlanError;
pub use frame::{FramePass, FramePlan};
pub use text::{NativeTextBounds, NativeTextLabel, TextScale, text_bounds};

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
