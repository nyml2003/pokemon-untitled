use punctum_gpu::{
    GpuAtlas, GpuAtlasError, GpuCell, GpuClip, GpuResource, PixelOffset, PixelRect, PixelSize,
    ResourceId, Rgba8, Viewport, ViewportError,
};

fn resource(id: u32, rect: PixelRect) -> GpuResource {
    GpuResource::new(ResourceId(id), rect)
}

#[test]
fn atlas_exposes_validated_pixels_resources_and_geometry() {
    let atlas = GpuAtlas::new(
        PixelSize::new(2, 2),
        vec![255; 16],
        &[resource(7, PixelRect::new(0, 0, 1, 2))],
    )
    .unwrap();

    assert_eq!(atlas.size(), PixelSize::new(2, 2));
    assert_eq!(atlas.rgba8(), &[255; 16]);
    assert_eq!(
        atlas.resource(ResourceId(7)),
        Some(PixelRect::new(0, 0, 1, 2))
    );
    assert_eq!(atlas.resource(ResourceId(8)), None);
    assert_eq!(PixelRect::new(1, 2, 3, 4).size(), PixelSize::new(3, 4));
}

#[test]
fn atlas_rejects_empty_size_and_invalid_pixel_storage() {
    for size in [PixelSize::new(0, 2), PixelSize::new(2, 0)] {
        let error = GpuAtlas::new(size, Vec::new(), &[]).unwrap_err();
        assert_eq!(error, GpuAtlasError::EmptyAtlas { size });
        assert!(error.to_string().contains("non-empty"));
    }

    let mismatch = GpuAtlas::new(PixelSize::new(2, 2), vec![0; 15], &[]).unwrap_err();
    assert_eq!(
        mismatch,
        GpuAtlasError::PixelLengthMismatch {
            size: PixelSize::new(2, 2),
            expected: 16,
            actual: 15,
        }
    );
    assert!(mismatch.to_string().contains("16"));

    let row_size = PixelSize::new(u32::MAX, 1);
    let row_overflow = GpuAtlas::new(row_size, Vec::new(), &[]).unwrap_err();
    assert_eq!(
        row_overflow,
        GpuAtlasError::RowByteLengthOverflow { size: row_size }
    );
    assert!(row_overflow.to_string().contains("row byte"));

    let size = PixelSize::new(65_536, 65_536);
    let overflow = GpuAtlas::new(size, Vec::new(), &[]).unwrap_err();
    assert_eq!(overflow, GpuAtlasError::PixelLengthOverflow { size });
    assert!(overflow.to_string().contains("overflows"));
}

#[test]
fn atlas_rejects_empty_out_of_bounds_and_duplicate_resources() {
    let size = PixelSize::new(2, 2);
    let pixels = || vec![0; 16];

    for rect in [PixelRect::new(0, 0, 0, 1), PixelRect::new(0, 0, 1, 0)] {
        let error = GpuAtlas::new(size, pixels(), &[resource(1, rect)]).unwrap_err();
        assert_eq!(error, GpuAtlasError::EmptyResource { id: ResourceId(1) });
        assert!(error.to_string().contains("empty"));
    }

    for rect in [PixelRect::new(1, 0, 2, 1), PixelRect::new(0, 1, 1, 2)] {
        let error = GpuAtlas::new(size, pixels(), &[resource(2, rect)]).unwrap_err();
        assert_eq!(
            error,
            GpuAtlasError::ResourceOutOfBounds {
                id: ResourceId(2),
                rect,
                atlas_size: size,
            }
        );
        assert!(error.to_string().contains("outside"));
    }

    let duplicate = GpuAtlas::new(
        size,
        pixels(),
        &[
            resource(3, PixelRect::new(0, 0, 1, 1)),
            resource(3, PixelRect::new(1, 1, 1, 1)),
        ],
    )
    .unwrap_err();
    assert_eq!(
        duplicate,
        GpuAtlasError::DuplicateResource { id: ResourceId(3) }
    );
    assert!(duplicate.to_string().contains("more than once"));
}

#[test]
fn viewport_rejects_empty_cells_and_keeps_layout_on_resize() {
    for cell_size in [PixelSize::new(0, 8), PixelSize::new(8, 0)] {
        let error =
            Viewport::new(PixelSize::new(100, 100), PixelOffset::new(0, 0), cell_size).unwrap_err();
        assert_eq!(error, ViewportError::EmptyCellSize { cell_size });
        assert!(error.to_string().contains("cell size"));
    }

    let viewport = Viewport::new(
        PixelSize::new(100, 80),
        PixelOffset::new(-4, 9),
        PixelSize::new(8, 6),
    )
    .unwrap();
    assert_eq!(
        viewport.resized(PixelSize::new(40, 30)),
        Viewport {
            target_size: PixelSize::new(40, 30),
            origin: PixelOffset::new(-4, 9),
            cell_size: PixelSize::new(8, 6),
        }
    );
}

#[test]
fn model_defaults_are_transparent_zero_or_unbounded() {
    assert_eq!(Rgba8::default(), Rgba8::new(0, 0, 0, 0));
    assert_eq!(PixelSize::default(), PixelSize::new(0, 0));
    assert_eq!(PixelOffset::default(), PixelOffset::new(0, 0));
    assert_eq!(PixelRect::default(), PixelRect::new(0, 0, 0, 0));
    assert_eq!(GpuCell::default(), GpuCell::Empty);
    assert_eq!(GpuClip::default(), GpuClip::Surface);
}
