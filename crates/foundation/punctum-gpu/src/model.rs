use std::{collections::BTreeMap, error::Error, fmt};

use punctum_grid::GridRect;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResourceId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgba8 {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl Default for Rgba8 {
    fn default() -> Self {
        Self::new(0, 0, 0, 0)
    }
}

impl Rgba8 {
    pub const fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    pub const fn to_array(self) -> [u8; 4] {
        [self.red, self.green, self.blue, self.alpha]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelSize {
    pub width: u32,
    pub height: u32,
}

impl Default for PixelSize {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

impl PixelSize {
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub const fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelOffset {
    pub x: i32,
    pub y: i32,
}

impl Default for PixelOffset {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

impl PixelOffset {
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PixelRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Default for PixelRect {
    fn default() -> Self {
        Self::new(0, 0, 0, 0)
    }
}

impl PixelRect {
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn size(self) -> PixelSize {
        PixelSize::new(self.width, self.height)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpuResource {
    pub id: ResourceId,
    pub atlas_rect: PixelRect,
}

impl GpuResource {
    pub const fn new(id: ResourceId, atlas_rect: PixelRect) -> Self {
        Self { id, atlas_rect }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GpuAtlas {
    size: PixelSize,
    rgba8: Vec<u8>,
    resources: BTreeMap<ResourceId, PixelRect>,
    fingerprint: u64,
}

impl GpuAtlas {
    pub fn new(
        size: PixelSize,
        rgba8: Vec<u8>,
        resources: &[GpuResource],
    ) -> Result<Self, GpuAtlasError> {
        if size.is_empty() {
            return Err(GpuAtlasError::EmptyAtlas { size });
        }
        if size.width.checked_mul(4).is_none() {
            return Err(GpuAtlasError::RowByteLengthOverflow { size });
        }

        let expected = checked_rgba8_length(size)?;
        if rgba8.len() != expected {
            return Err(GpuAtlasError::PixelLengthMismatch {
                size,
                expected,
                actual: rgba8.len(),
            });
        }

        let mut entries = BTreeMap::new();
        for &resource in resources {
            if resource.atlas_rect.size().is_empty() {
                return Err(GpuAtlasError::EmptyResource { id: resource.id });
            }
            if !rect_fits(resource.atlas_rect, size) {
                return Err(GpuAtlasError::ResourceOutOfBounds {
                    id: resource.id,
                    rect: resource.atlas_rect,
                    atlas_size: size,
                });
            }
            if entries.insert(resource.id, resource.atlas_rect).is_some() {
                return Err(GpuAtlasError::DuplicateResource { id: resource.id });
            }
        }

        let fingerprint = fnv1a(&rgba8);
        Ok(Self {
            size,
            rgba8,
            resources: entries,
            fingerprint,
        })
    }

    pub const fn size(&self) -> PixelSize {
        self.size
    }

    /// 基于像素内容的确定性指纹，用于检测图集内容是否变化。
    pub const fn fingerprint(&self) -> u64 {
        self.fingerprint
    }

    pub fn rgba8(&self) -> &[u8] {
        &self.rgba8
    }

    pub fn resource(&self, id: ResourceId) -> Option<PixelRect> {
        self.resources.get(&id).copied()
    }
}

/// FNV-1a 哈希，用作图集内容的确定性指纹。
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn checked_rgba8_length(size: PixelSize) -> Result<usize, GpuAtlasError> {
    let length = u128::from(size.width) * u128::from(size.height) * 4;
    if length > u128::from(u32::MAX) {
        return Err(GpuAtlasError::PixelLengthOverflow { size });
    }
    Ok(length as usize)
}

fn rect_fits(rect: PixelRect, size: PixelSize) -> bool {
    u64::from(rect.x) + u64::from(rect.width) <= u64::from(size.width)
        && u64::from(rect.y) + u64::from(rect.height) <= u64::from(size.height)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuAtlasError {
    EmptyAtlas {
        size: PixelSize,
    },
    PixelLengthOverflow {
        size: PixelSize,
    },
    RowByteLengthOverflow {
        size: PixelSize,
    },
    PixelLengthMismatch {
        size: PixelSize,
        expected: usize,
        actual: usize,
    },
    EmptyResource {
        id: ResourceId,
    },
    ResourceOutOfBounds {
        id: ResourceId,
        rect: PixelRect,
        atlas_size: PixelSize,
    },
    DuplicateResource {
        id: ResourceId,
    },
}

impl fmt::Display for GpuAtlasError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAtlas { size } => {
                write!(formatter, "GPU atlas must be non-empty, got {size:?}")
            }
            Self::PixelLengthOverflow { size } => {
                write!(formatter, "RGBA8 byte length overflows for atlas {size:?}")
            }
            Self::RowByteLengthOverflow { size } => {
                write!(
                    formatter,
                    "RGBA8 row byte length overflows for atlas {size:?}"
                )
            }
            Self::PixelLengthMismatch {
                size,
                expected,
                actual,
            } => write!(
                formatter,
                "atlas {size:?} requires {expected} RGBA8 bytes, received {actual}"
            ),
            Self::EmptyResource { id } => {
                write!(
                    formatter,
                    "GPU resource {id:?} has an empty atlas rectangle"
                )
            }
            Self::ResourceOutOfBounds {
                id,
                rect,
                atlas_size,
            } => write!(
                formatter,
                "GPU resource {id:?} rectangle {rect:?} is outside atlas {atlas_size:?}"
            ),
            Self::DuplicateResource { id } => {
                write!(formatter, "GPU resource {id:?} is defined more than once")
            }
        }
    }
}

impl Error for GpuAtlasError {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GpuCell {
    #[default]
    Empty,
    Sprite {
        resource: ResourceId,
        tint: Rgba8,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpuImage {
    pub bounds: GridRect,
    pub pixel_offset: PixelOffset,
    pub resource: ResourceId,
    pub tint: Rgba8,
    pub z_index: i32,
}

/// Compact data consumed by the radar-chart fragment shader.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RadarInstanceData {
    pub(crate) values: [u32; 6],
    pub(crate) max: u32,
    pub(crate) rings: u32,
    pub(crate) grid_color: [u32; 4],
    pub(crate) axis_color: [u32; 4],
    pub(crate) fill_color: [u32; 4],
    pub(crate) edge_color: [u32; 4],
    pub(crate) point_color: [u32; 4],
    pub(crate) bounds: [u32; 4],
}

impl RadarInstanceData {
    pub const fn new(values: [u16; 6], max: u16, rings: u8, colors: [Rgba8; 5]) -> Self {
        Self {
            values: [
                values[0] as u32,
                values[1] as u32,
                values[2] as u32,
                values[3] as u32,
                values[4] as u32,
                values[5] as u32,
            ],
            max: max as u32,
            rings: rings as u32,
            grid_color: color_words(colors[0]),
            axis_color: color_words(colors[1]),
            fill_color: color_words(colors[2]),
            edge_color: color_words(colors[3]),
            point_color: color_words(colors[4]),
            bounds: [0; 4],
        }
    }

    pub const fn with_bounds(mut self, bounds: PixelRect) -> Self {
        self.bounds = [bounds.x, bounds.y, bounds.width, bounds.height];
        self
    }
}

const fn color_words(color: Rgba8) -> [u32; 4] {
    [
        color.red as u32,
        color.green as u32,
        color.blue as u32,
        color.alpha as u32,
    ]
}

/// A textured rectangle positioned directly in target pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpuPixelImage {
    pub bounds: PixelRect,
    pub pixel_offset: PixelOffset,
    pub corner_radii: [u32; 4],
    pub resource: ResourceId,
    pub tint: Rgba8,
    pub z_index: i32,
    pub(crate) circle: Option<GpuPixelCircle>,
    pub(crate) radar: Option<RadarInstanceData>,
    pub(crate) weather: Option<GpuWeather>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GpuPixelCircle {
    pub(crate) center: PixelOffset,
    pub(crate) radius: u32,
}

/// 全屏天气覆盖实例：pattern 为天气类型码，frame 驱动粒子动画。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GpuWeather {
    pub(crate) pattern: u32,
    pub(crate) frame: u32,
}

impl GpuPixelImage {
    pub const fn new(bounds: PixelRect, resource: ResourceId, tint: Rgba8, z_index: i32) -> Self {
        Self {
            bounds,
            pixel_offset: PixelOffset::new(0, 0),
            corner_radii: [0; 4],
            resource,
            tint,
            z_index,
            circle: None,
            radar: None,
            weather: None,
        }
    }

    pub const fn with_pixel_offset(mut self, pixel_offset: PixelOffset) -> Self {
        self.pixel_offset = pixel_offset;
        self
    }

    /// Corner radii are ordered top-left, top-right, bottom-right, bottom-left.
    pub const fn with_corner_radii(mut self, corner_radii: [u32; 4]) -> Self {
        self.corner_radii = corner_radii;
        self
    }

    pub const fn with_circle(mut self, center: PixelOffset, radius: u32) -> Self {
        self.circle = Some(GpuPixelCircle { center, radius });
        self
    }

    pub const fn with_radar(mut self, radar: RadarInstanceData) -> Self {
        self.radar = Some(radar);
        self
    }

    pub const fn with_weather(mut self, pattern: u32, frame: u32) -> Self {
        self.weather = Some(GpuWeather { pattern, frame });
        self
    }
}

impl GpuImage {
    pub const fn new(bounds: GridRect, resource: ResourceId, tint: Rgba8, z_index: i32) -> Self {
        Self {
            bounds,
            pixel_offset: PixelOffset::new(0, 0),
            resource,
            tint,
            z_index,
        }
    }

    pub const fn with_pixel_offset(mut self, pixel_offset: PixelOffset) -> Self {
        self.pixel_offset = pixel_offset;
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Viewport {
    pub target_size: PixelSize,
    pub origin: PixelOffset,
    pub cell_size: PixelSize,
}

impl Viewport {
    pub fn new(
        target_size: PixelSize,
        origin: PixelOffset,
        cell_size: PixelSize,
    ) -> Result<Self, ViewportError> {
        if cell_size.is_empty() {
            return Err(ViewportError::EmptyCellSize { cell_size });
        }
        Ok(Self {
            target_size,
            origin,
            cell_size,
        })
    }

    pub const fn resized(self, target_size: PixelSize) -> Self {
        Self {
            target_size,
            ..self
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewportError {
    EmptyCellSize { cell_size: PixelSize },
}

impl fmt::Display for ViewportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCellSize { cell_size } => {
                write!(
                    formatter,
                    "GPU cell size must be non-empty, got {cell_size:?}"
                )
            }
        }
    }
}

impl Error for ViewportError {}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GpuClip {
    #[default]
    Surface,
    Rect(GridRect),
}
