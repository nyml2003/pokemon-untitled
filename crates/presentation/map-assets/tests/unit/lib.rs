use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};

use super::*;

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes)
        .write_image(
            &vec![255; (width * height * 4) as usize],
            width,
            height,
            ExtendedColorType::Rgba8,
        )
        .unwrap();
    bytes
}

fn source(name: &str, bytes: Vec<u8>) -> TileSource {
    TileSource {
        name: name.into(),
        bytes,
    }
}

#[test]
fn tile_assembly_is_sorted_and_validated() {
    let assets = build_tile_assets(vec![
        source("tile-0102", png(16, 16)),
        source("tile-0101", png(16, 16)),
    ])
    .unwrap();
    assert_eq!(assets.ids[0].as_str(), "tile-0101");
    assert_eq!(assets.ids[1].as_str(), "tile-0102");
    assert_eq!(assets.images[0].1.size(), PixelSize::new(16, 16));
    assert_eq!(assets.images[0].0.as_str(), "map/tile/0101");
    assert_eq!(assets.catalog.len(), 2);

    for error in [
        build_tile_assets(Vec::new()).unwrap_err(),
        build_tile_assets(vec![source("bad", b"not png".to_vec())]).unwrap_err(),
        build_tile_assets(vec![source("bad", png(1, 1))]).unwrap_err(),
        build_tile_assets(vec![source(" ", png(16, 16))]).unwrap_err(),
        build_tile_assets(vec![
            source("duplicate", png(16, 16)),
            source("duplicate", png(16, 16)),
        ])
        .unwrap_err(),
    ] {
        assert!(!error.to_string().is_empty());
    }
}

#[test]
fn project_loading_uses_the_same_known_tile_set_for_defaults_and_json() {
    let assets = build_tile_assets(vec![source("tile-0102", png(16, 16))]).unwrap();
    let project = project_from_json_or_default(None, &assets.ids).unwrap();
    assert_eq!((project.width, project.height), (24, 16));

    let known = assets.ids.iter().cloned().collect();
    let json = project.to_json_pretty(&known).unwrap();
    assert_eq!(
        project_from_json_or_default(Some(&json), &assets.ids)
            .unwrap()
            .id,
        project.id
    );
    assert!(project_from_json_or_default(None, &[]).is_err());
    let error = project_from_json_or_default(Some("{}"), &assets.ids).unwrap_err();
    assert!(!error.to_string().is_empty());
}
