use std::error::Error as _;

use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};
use punctum_gpu::{PixelRect, PixelSize, ResourceId, Rgba8};

use super::{AssetCatalog, DecodedImage, build_atlas, build_atlas_with_limit, decode_png};

#[test]
fn catalog_resolves_canonical_keys_without_exposing_paths_to_callers() {
    let catalog = AssetCatalog::from_json(
        br#"{
                "schema_version": 1,
                "assets": [{"key":"map/tile/0101","source":"source/map/tile/0101.png"}]
            }"#,
    )
    .unwrap();
    let key = super::AssetKey::new("map/tile/0101").unwrap();
    assert_eq!(
        catalog.get(&key).map(super::AssetDescriptor::source),
        Some("source/map/tile/0101.png")
    );
    assert_eq!(catalog.prefixed("map/tile/").count(), 1);
    assert!(
        AssetCatalog::from_json(
            br#"{"schema_version":1,"assets":[{"key":"map/tile/0101","source":"old/tile.png"}]}"#
        )
        .is_err()
    );
}

#[test]
fn stable_asset_keys_and_all_error_variants_are_observable() {
    let key = super::AssetKey::new("battle/player").unwrap();
    assert_eq!(key.as_str(), "battle/player");
    assert!(matches!(
        super::AssetKey::new("  "),
        Err(super::AssetError::EmptyAssetKey)
    ));

    let empty = DecodedImage {
        size: PixelSize::new(0, 0),
        rgba8: Vec::new(),
    };
    let wide = DecodedImage {
        size: PixelSize::new(2, 1),
        rgba8: Vec::new(),
    };
    let tall = DecodedImage {
        size: PixelSize::new(1, 5_000),
        rgba8: Vec::new(),
    };
    assert!(matches!(
        build_atlas(&[]),
        Err(super::AssetError::EmptyImageSet)
    ));
    assert!(matches!(
        build_atlas_with_limit(&[(ResourceId(1), &wide)], 0),
        Err(super::AssetError::AtlasDimensionsOverflow)
    ));
    assert!(matches!(
        build_atlas(&[(ResourceId(1), &empty)]),
        Err(super::AssetError::EmptyImage { .. })
    ));
    assert!(matches!(
        build_atlas_with_limit(&[(ResourceId(1), &wide)], 1),
        Err(super::AssetError::AtlasDimensionsOverflow)
    ));
    assert!(matches!(
        build_atlas_with_limit(&[(ResourceId(1), &tall), (ResourceId(2), &tall)], 1),
        Err(super::AssetError::AtlasDimensionsOverflow)
    ));
    let pixel = DecodedImage::solid(Rgba8::new(255, 255, 255, 255));
    assert!(matches!(
        build_atlas(&[(ResourceId(1), &pixel), (ResourceId(1), &pixel)]),
        Err(super::AssetError::InvalidAtlas(
            punctum_gpu::GpuAtlasError::DuplicateResource { .. }
        ))
    ));
    assert!(matches!(
        DecodedImage::from_rgba8(PixelSize::new(2, 2), vec![0; 3]),
        Err(super::AssetError::PixelLengthMismatch { .. })
    ));

    let invalid_atlas = super::AssetError::InvalidAtlas(punctum_gpu::GpuAtlasError::EmptyAtlas {
        size: PixelSize::new(0, 0),
    });
    for error in [
        super::AssetError::EmptyAssetKey,
        super::AssetError::InvalidCatalog("bad".into()),
        super::AssetError::InvalidPng("bad".into()),
        super::AssetError::EmptyImageSet,
        super::AssetError::EmptyImage { id: ResourceId(1) },
        super::AssetError::PixelLengthOverflow {
            size: PixelSize::new(u32::MAX, u32::MAX),
        },
        super::AssetError::PixelLengthMismatch {
            size: PixelSize::new(1, 1),
            expected: 4,
            actual: 0,
        },
        super::AssetError::AtlasDimensionsOverflow,
        invalid_atlas,
    ] {
        assert!(!error.to_string().is_empty());
        assert_eq!(
            error.source().is_some(),
            matches!(error, super::AssetError::InvalidAtlas(_))
        );
    }
}

#[test]
fn decodes_png_pixels_as_rgba8() {
    let pixels = [255, 0, 0, 255, 0, 128, 255, 64];
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(&pixels, 2, 1, ExtendedColorType::Rgba8)
        .unwrap();

    let image = decode_png(&png).unwrap();
    assert_eq!(image.size().width, 2);
    assert_eq!(image.size().height, 1);
    assert_eq!(image.rgba8(), pixels);
}

#[test]
fn packs_images_in_stable_horizontal_order() {
    let white = DecodedImage::solid(Rgba8::new(255, 255, 255, 255));
    let red = DecodedImage::solid(Rgba8::new(255, 0, 0, 128));
    let atlas = build_atlas(&[(ResourceId(1), &white), (ResourceId(2), &red)]).unwrap();

    assert_eq!(
        atlas.resource(ResourceId(1)),
        Some(PixelRect::new(0, 0, 1, 1))
    );
    assert_eq!(
        atlas.resource(ResourceId(2)),
        Some(PixelRect::new(1, 0, 1, 1))
    );
    assert_eq!(atlas.rgba8(), &[255, 255, 255, 255, 255, 0, 0, 128]);
}

#[test]
fn wraps_images_to_new_rows_before_the_texture_limit() {
    let white = DecodedImage::solid(Rgba8::new(255, 255, 255, 255));
    let red = DecodedImage::solid(Rgba8::new(255, 0, 0, 255));
    let blue = DecodedImage::solid(Rgba8::new(0, 0, 255, 255));
    let atlas = build_atlas_with_limit(
        &[
            (ResourceId(1), &white),
            (ResourceId(2), &red),
            (ResourceId(3), &blue),
        ],
        2,
    )
    .unwrap();

    assert_eq!(atlas.size(), PixelSize::new(2, 2));
    assert_eq!(
        atlas.resource(ResourceId(1)),
        Some(PixelRect::new(0, 0, 1, 1))
    );
    assert_eq!(
        atlas.resource(ResourceId(2)),
        Some(PixelRect::new(1, 0, 1, 1))
    );
    assert_eq!(
        atlas.resource(ResourceId(3)),
        Some(PixelRect::new(0, 1, 1, 1))
    );
}

#[test]
fn rejects_non_png_bytes() {
    assert!(decode_png(b"not a png").is_err());
}
