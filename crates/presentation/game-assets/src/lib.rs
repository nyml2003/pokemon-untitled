//! PNG decoding and deterministic GPU atlas assembly.

#![forbid(unsafe_code)]

use std::{collections::BTreeMap, error::Error, fmt};

use image::ImageFormat;
use punctum_gpu::{GpuAtlas, GpuAtlasError, GpuResource, PixelRect, PixelSize, ResourceId, Rgba8};
use serde::Deserialize;

const MAX_ATLAS_DIMENSION: u32 = 8_192;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetKey(String);

impl AssetKey {
    pub fn new(value: impl Into<String>) -> Result<Self, AssetError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(AssetError::EmptyAssetKey);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Builds a key from a fixed resource template that guarantees a non-empty path.
    ///
    /// This is intentionally separate from [`Self::new`]: external asset
    /// catalog input must still be validated by the fallible constructor.
    pub fn from_resource_template(value: String) -> Self {
        Self(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssetDescriptor {
    source: String,
}

impl AssetDescriptor {
    pub fn source(&self) -> &str {
        &self.source
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AssetCatalog {
    assets: BTreeMap<AssetKey, AssetDescriptor>,
}

impl AssetCatalog {
    pub fn from_json(bytes: &[u8]) -> Result<Self, AssetError> {
        let document: CatalogDocument = serde_json::from_slice(bytes)
            .map_err(|error| AssetError::InvalidCatalog(error.to_string()))?;
        if document.schema_version != 1 {
            return Err(AssetError::InvalidCatalog(format!(
                "unsupported schema version {}",
                document.schema_version
            )));
        }
        let mut assets = BTreeMap::new();
        for entry in document.assets {
            let key = AssetKey::new(entry.key)?;
            if !entry.source.starts_with("source/") || entry.source.contains('\\') {
                return Err(AssetError::InvalidCatalog(format!(
                    "{key:?} has an invalid source path"
                )));
            }
            if assets
                .insert(
                    key.clone(),
                    AssetDescriptor {
                        source: entry.source,
                    },
                )
                .is_some()
            {
                return Err(AssetError::InvalidCatalog(format!("duplicate key {key:?}")));
            }
        }
        Ok(Self { assets })
    }

    pub fn get(&self, key: &AssetKey) -> Option<&AssetDescriptor> {
        self.assets.get(key)
    }

    pub fn prefixed(&self, prefix: &str) -> impl Iterator<Item = (&AssetKey, &AssetDescriptor)> {
        self.assets
            .iter()
            .filter(move |(key, _)| key.as_str().starts_with(prefix))
    }
}

#[derive(Deserialize)]
struct CatalogDocument {
    schema_version: u32,
    assets: Vec<CatalogEntry>,
}

#[derive(Deserialize)]
struct CatalogEntry {
    key: String,
    source: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedImage {
    size: PixelSize,
    rgba8: Vec<u8>,
}

impl DecodedImage {
    pub fn solid(color: Rgba8) -> Self {
        Self {
            size: PixelSize::new(1, 1),
            rgba8: color.to_array().to_vec(),
        }
    }

    pub fn from_rgba8(size: PixelSize, rgba8: Vec<u8>) -> Result<Self, AssetError> {
        let expected = usize::try_from(u64::from(size.width) * u64::from(size.height) * 4)
            .map_err(|_| AssetError::PixelLengthOverflow { size })?;
        if rgba8.len() != expected {
            return Err(AssetError::PixelLengthMismatch {
                size,
                expected,
                actual: rgba8.len(),
            });
        }
        Ok(Self { size, rgba8 })
    }

    pub const fn size(&self) -> PixelSize {
        self.size
    }

    pub fn rgba8(&self) -> &[u8] {
        &self.rgba8
    }
}

pub fn decode_png(bytes: &[u8]) -> Result<DecodedImage, AssetError> {
    let rgba = image::load_from_memory_with_format(bytes, ImageFormat::Png)
        .map_err(|error| AssetError::InvalidPng(error.to_string()))?
        .to_rgba8();
    Ok(DecodedImage {
        size: PixelSize::new(rgba.width(), rgba.height()),
        rgba8: rgba.into_raw(),
    })
}

pub fn build_atlas(images: &[(ResourceId, &DecodedImage)]) -> Result<GpuAtlas, AssetError> {
    build_atlas_with_limit(images, MAX_ATLAS_DIMENSION)
}

fn build_atlas_with_limit(
    images: &[(ResourceId, &DecodedImage)],
    maximum_width: u32,
) -> Result<GpuAtlas, AssetError> {
    if images.is_empty() {
        return Err(AssetError::EmptyImageSet);
    }
    if maximum_width == 0 {
        return Err(AssetError::AtlasDimensionsOverflow);
    }

    let mut placements = Vec::with_capacity(images.len());
    let mut x = 0_u32;
    let mut y = 0_u32;
    let mut row_height = 0_u32;
    let mut width = 0_u32;
    for &(id, image) in images {
        if image.size.is_empty() {
            return Err(AssetError::EmptyImage { id });
        }
        if image.size.width > maximum_width || image.size.height > MAX_ATLAS_DIMENSION {
            return Err(AssetError::AtlasDimensionsOverflow);
        }
        if x > 0 && x.saturating_add(image.size.width) > maximum_width {
            y = y
                .checked_add(row_height)
                .ok_or(AssetError::AtlasDimensionsOverflow)?;
            x = 0;
            row_height = 0;
        }
        placements.push((id, image, x, y));
        x = x
            .checked_add(image.size.width)
            .ok_or(AssetError::AtlasDimensionsOverflow)?;
        row_height = row_height.max(image.size.height);
        width = width.max(x);
    }
    let height = y
        .checked_add(row_height)
        .ok_or(AssetError::AtlasDimensionsOverflow)?;
    if height > MAX_ATLAS_DIMENSION {
        return Err(AssetError::AtlasDimensionsOverflow);
    }
    let size = PixelSize::new(width, height);
    let byte_len = u64::from(width) * u64::from(height) * 4;
    let byte_len = usize::try_from(byte_len).map_err(|_| AssetError::AtlasDimensionsOverflow)?;
    let mut rgba8 = vec![0; byte_len];
    let mut resources = Vec::with_capacity(images.len());
    let atlas_row_bytes = width as usize * 4;

    for (id, image, x, y) in placements {
        let image_row_bytes = image.size.width as usize * 4;
        for row in 0..image.size.height as usize {
            let source = row * image_row_bytes;
            let target = (y as usize + row) * atlas_row_bytes + x as usize * 4;
            rgba8[target..target + image_row_bytes]
                .copy_from_slice(&image.rgba8[source..source + image_row_bytes]);
        }
        resources.push(GpuResource::new(
            id,
            PixelRect::new(x, y, image.size.width, image.size.height),
        ));
    }

    GpuAtlas::new(size, rgba8, &resources).map_err(AssetError::InvalidAtlas)
}

#[derive(Debug)]
pub enum AssetError {
    EmptyAssetKey,
    InvalidCatalog(String),
    InvalidPng(String),
    EmptyImageSet,
    EmptyImage {
        id: ResourceId,
    },
    PixelLengthOverflow {
        size: PixelSize,
    },
    PixelLengthMismatch {
        size: PixelSize,
        expected: usize,
        actual: usize,
    },
    AtlasDimensionsOverflow,
    InvalidAtlas(GpuAtlasError),
}

impl fmt::Display for AssetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAssetKey => formatter.write_str("asset key must not be empty"),
            Self::InvalidCatalog(error) => write!(formatter, "invalid asset catalog: {error}"),
            Self::InvalidPng(error) => write!(formatter, "invalid PNG: {error}"),
            Self::EmptyImageSet => formatter.write_str("an atlas requires at least one image"),
            Self::EmptyImage { id } => write!(formatter, "image {id:?} is empty"),
            Self::PixelLengthOverflow { size } => {
                write!(formatter, "RGBA8 byte length overflows for image {size:?}")
            }
            Self::PixelLengthMismatch {
                size,
                expected,
                actual,
            } => write!(
                formatter,
                "image {size:?} requires {expected} RGBA8 bytes, received {actual}"
            ),
            Self::AtlasDimensionsOverflow => formatter.write_str("atlas dimensions overflow"),
            Self::InvalidAtlas(error) => write!(formatter, "invalid GPU atlas: {error}"),
        }
    }
}

impl Error for AssetError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidAtlas(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
