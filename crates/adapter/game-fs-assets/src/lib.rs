//! Filesystem adapter for deterministic asset plans.

#![forbid(unsafe_code)]

use std::{fs, io, path::Path};

use game_asset_plan::{AssetBytes, AssetRequest};
use game_assets::AssetCatalog;
use map_assets::TileSource;

pub fn load_catalog(root: &Path) -> io::Result<AssetCatalog> {
    let path = root.join("catalog/assets.v1.json");
    let bytes = fs::read(&path).map_err(|error| at_path(&path, error))?;
    AssetCatalog::from_json(&bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))
}

pub fn read_tile_sources(root: &Path, catalog: &AssetCatalog) -> io::Result<Vec<TileSource>> {
    catalog
        .prefixed("map/tile/")
        .map(|(key, descriptor)| {
            let tile = key.as_str().strip_prefix("map/tile/").ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidData, "invalid map tile key")
            })?;
            let path = root.join(descriptor.source());
            let bytes = fs::read(&path).map_err(|error| at_path(&path, error))?;
            Ok(TileSource {
                name: format!("tile-{tile}"),
                bytes,
            })
        })
        .collect()
}

pub fn read_asset_requests(
    root: &Path,
    catalog: &AssetCatalog,
    requests: Vec<AssetRequest>,
) -> io::Result<Vec<AssetBytes>> {
    requests
        .into_iter()
        .map(|request| {
            let descriptor = catalog.get(&request.asset_key).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("asset not found in catalog: {}", request.asset_key.as_str()),
                )
            })?;
            let path = root.join(descriptor.source());
            let bytes = fs::read(&path).map_err(|error| at_path(&path, error))?;
            Ok(AssetBytes { request, bytes })
        })
        .collect()
}

pub fn read_optional_text(path: &Path) -> io::Result<Option<String>> {
    path.is_file()
        .then(|| fs::read_to_string(path).map_err(|error| at_path(path, error)))
        .transpose()
}

fn at_path(path: &Path, error: io::Error) -> io::Error {
    io::Error::new(error.kind(), format!("{}: {error}", path.display()))
}
