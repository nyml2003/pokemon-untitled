use std::{error::Error, fmt};

use game_assets::AssetKey;
use punctum_gpu::GpuPlanError;
use punctum_grid::{GridSize, SurfaceError};

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
