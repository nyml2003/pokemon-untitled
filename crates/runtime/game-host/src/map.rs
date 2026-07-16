use std::{error::Error, path::PathBuf};

use game_assets::{AssetKey, DecodedImage};
use game_fs_assets::{load_catalog, read_optional_text, read_tile_sources};
use map_assets::{build_tile_assets, project_from_json_or_default};
use map_project::MapProject;
use map_render::AtomicTileCatalog;

pub struct LoadedMap {
    pub project: MapProject,
    pub catalog: AtomicTileCatalog,
    pub images: Vec<(AssetKey, DecodedImage)>,
}

pub fn load_map() -> Result<LoadedMap, Box<dyn Error>> {
    let root = asset_root();
    let catalog = load_catalog(&root)?;
    let assets = build_tile_assets(read_tile_sources(&root, &catalog)?)?;
    let json = read_optional_text(&project_path())?;
    let project = project_from_json_or_default(json.as_deref(), &assets.ids)?;
    Ok(LoadedMap {
        project,
        catalog: assets.catalog,
        images: assets.images,
    })
}

fn asset_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../assets")
}

fn project_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../maps/demo-map.json")
}

#[cfg(test)]
mod tests {
    use super::load_map;

    #[test]
    fn checked_in_demo_map_loads_with_static_actors() {
        let map = load_map().unwrap();
        assert_eq!(map.project.format_version, map_project::FORMAT_VERSION);
        assert_eq!(map.project.actors.len(), 4);
        assert_eq!(
            map.project
                .actors
                .iter()
                .map(|actor| (actor.id.as_str(), actor.appearance.as_str()))
                .collect::<Vec<_>>(),
            [
                ("forest-guide", "dppt/000"),
                ("forest-scout", "dppt/001"),
                ("forest-ranger", "dppt/002"),
                ("forest-collector", "dppt/003"),
            ]
        );
    }
}
