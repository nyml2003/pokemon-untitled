use std::{collections::BTreeMap, error::Error, fmt};

use game_assets::{AssetKey, DecodedImage};
use punctum_gpu::{GpuAtlas, PixelSize, ResourceId};

pub struct NativeAssets {
    pub(crate) atlas: GpuAtlas,
    resources: BTreeMap<AssetKey, ResourceId>,
}

impl NativeAssets {
    pub fn new(images: Vec<(AssetKey, DecodedImage)>) -> Result<Self, NativeAssetError> {
        let mut resources = BTreeMap::new();
        let mut numbered = Vec::with_capacity(images.len());
        for (index, (key, image)) in images.iter().enumerate() {
            let id = resource_id(index)?;
            if resources.insert(key.clone(), id).is_some() {
                return Err(NativeAssetError::DuplicateKey(key.clone()));
            }
            numbered.push((id, image));
        }
        let atlas = game_assets::build_atlas(&numbered)
            .map_err(|error| NativeAssetError::Atlas(error.to_string()))?;
        Ok(Self { atlas, resources })
    }

    pub fn resource(&self, key: &AssetKey) -> Option<ResourceId> {
        self.resources.get(key).copied()
    }

    pub const fn atlas_size(&self) -> PixelSize {
        self.atlas.size()
    }

    pub const fn atlas(&self) -> &GpuAtlas {
        &self.atlas
    }
}

pub(crate) fn resource_id(index: usize) -> Result<ResourceId, NativeAssetError> {
    Ok(ResourceId(
        u32::try_from(index)
            .map_err(|_| NativeAssetError::TooManyAssets)?
            .checked_add(1)
            .ok_or(NativeAssetError::TooManyAssets)?,
    ))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeAssetError {
    DuplicateKey(AssetKey),
    TooManyAssets,
    Atlas(String),
}

impl fmt::Display for NativeAssetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateKey(key) => write!(formatter, "duplicate asset key {}", key.as_str()),
            Self::TooManyAssets => formatter.write_str("native asset count exceeds u32"),
            Self::Atlas(message) => write!(formatter, "cannot build native atlas: {message}"),
        }
    }
}

impl Error for NativeAssetError {}
