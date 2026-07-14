use punctum_gpu::{
    GpuAtlas, GpuCell, GpuClip, GpuImage, GpuPlanError, GpuResource, InstanceData, PixelOffset,
    PixelRect, PixelSize, ResourceId, Rgba8, SubmissionMode, Viewport, plan_composite, plan_patch,
    plan_surface,
};
use punctum_grid::{GridPos, GridRect, GridSize, Surface, diff};

fn atlas() -> GpuAtlas {
    GpuAtlas::new(
        PixelSize::new(4, 2),
        vec![255; 32],
        &[
            GpuResource::new(ResourceId(1), PixelRect::new(0, 0, 2, 2)),
            GpuResource::new(ResourceId(2), PixelRect::new(2, 0, 2, 2)),
        ],
    )
    .unwrap()
}

fn viewport() -> Viewport {
    Viewport::new(
        PixelSize::new(100, 80),
        PixelOffset::new(-5, 10),
        PixelSize::new(10, 8),
    )
    .unwrap()
}

fn sprite(id: u32, tint: Rgba8) -> GpuCell {
    GpuCell::Sprite {
        resource: ResourceId(id),
        tint,
    }
}

#[test]
fn surface_plan_resolves_resources_and_preserves_row_major_slots() {
    let surface = Surface::from_cells(
        GridSize::new(3, 2),
        vec![
            sprite(1, Rgba8::new(255, 0, 0, 128)),
            GpuCell::Empty,
            sprite(2, Rgba8::new(0, 255, 0, 255)),
            GpuCell::Empty,
            sprite(1, Rgba8::new(0, 255, 0, 255)),
            GpuCell::Empty,
        ],
    )
    .unwrap();

    let plan = plan_surface(&surface, &atlas(), u32::MAX, viewport(), GpuClip::Surface).unwrap();

    assert_eq!(plan.grid_size, GridSize::new(3, 2));
    assert_eq!(plan.mode, SubmissionMode::Replace);
    assert_eq!(plan.viewport, viewport());
    assert_eq!(plan.scissor, Some(PixelRect::new(0, 10, 25, 16)));
    assert_eq!(plan.instance_count, 6);
    assert_eq!(plan.uploads.len(), 1);
    assert_eq!(plan.uploads[0].first_slot, 0);
    assert_eq!(plan.uploads[0].instances.len(), 6);
    assert_eq!(
        plan.uploads[0].instances[0],
        InstanceData {
            grid_position: [0, 0],
            grid_span: [1, 1],
            pixel_offset: [0, 0],
            atlas_rect: [0, 0, 2, 2],
            tint: [255, 0, 0, 128],
            visible: 1,
        }
    );
    assert_eq!(
        plan.uploads[0].instances[1],
        InstanceData {
            grid_position: [1, 0],
            grid_span: [1, 1],
            pixel_offset: [0, 0],
            atlas_rect: [0; 4],
            tint: [0; 4],
            visible: 0,
        }
    );
    assert_eq!(plan.uploads[0].instances[2].grid_position, [2, 0]);
    assert_eq!(plan.uploads[0].instances[2].atlas_rect, [2, 0, 2, 2]);
    assert_eq!(plan.uploads[0].instances[4].grid_position, [1, 1]);
}

#[test]
fn composite_plan_draws_cells_then_images_in_stable_z_order() {
    let surface = Surface::filled(GridSize::new(4, 3), GpuCell::Empty).unwrap();
    let images = [
        GpuImage::new(
            GridRect::new(GridPos::new(2, 1), GridSize::new(2, 2)),
            ResourceId(2),
            Rgba8::new(10, 20, 30, 255),
            5,
        )
        .with_pixel_offset(PixelOffset::new(-3, 4)),
        GpuImage::new(
            GridRect::new(GridPos::new(0, 0), GridSize::new(1, 2)),
            ResourceId(1),
            Rgba8::new(255, 255, 255, 255),
            -1,
        ),
    ];

    let plan = plan_composite(
        &surface,
        &images,
        &atlas(),
        u32::MAX,
        viewport(),
        GpuClip::Surface,
    )
    .unwrap();
    let instances = &plan.uploads[0].instances;

    assert_eq!(plan.instance_count, 14);
    assert_eq!(instances[11].grid_span, [1, 1]);
    assert_eq!(instances[12].grid_position, [0, 0]);
    assert_eq!(instances[12].grid_span, [1, 2]);
    assert_eq!(instances[12].atlas_rect, [0, 0, 2, 2]);
    assert_eq!(instances[13].grid_position, [2, 1]);
    assert_eq!(instances[13].grid_span, [2, 2]);
    assert_eq!(instances[13].pixel_offset, [-3, 4]);
    assert_eq!(instances[13].atlas_rect, [2, 0, 2, 2]);
}

#[test]
fn composite_plan_rejects_images_outside_the_grid() {
    let surface = Surface::filled(GridSize::new(4, 3), GpuCell::Empty).unwrap();
    let bounds = GridRect::new(GridPos::new(3, 2), GridSize::new(2, 2));
    let image = GpuImage::new(bounds, ResourceId(1), Rgba8::new(255, 255, 255, 255), 0);

    assert_eq!(
        plan_composite(
            &surface,
            &[image],
            &atlas(),
            u32::MAX,
            viewport(),
            GpuClip::Surface,
        )
        .unwrap_err(),
        GpuPlanError::ImageOutOfBounds {
            bounds,
            grid_size: GridSize::new(4, 3),
        }
    );
}

#[test]
fn patch_plan_uploads_only_changed_spans_at_stable_slots() {
    let size = GridSize::new(4, 2);
    let previous = Surface::filled(size, GpuCell::Empty).unwrap();
    let next = Surface::from_cells(
        size,
        vec![
            GpuCell::Empty,
            sprite(1, Rgba8::new(1, 2, 3, 4)),
            sprite(2, Rgba8::new(5, 6, 7, 8)),
            GpuCell::Empty,
            GpuCell::Empty,
            GpuCell::Empty,
            GpuCell::Empty,
            sprite(1, Rgba8::new(9, 10, 11, 12)),
        ],
    )
    .unwrap();

    let plan = plan_patch(
        &diff(&previous, &next),
        &atlas(),
        u32::MAX,
        viewport(),
        GpuClip::Surface,
    )
    .unwrap();

    assert_eq!(plan.mode, SubmissionMode::Delta);
    assert_eq!(plan.instance_count, 8);
    assert_eq!(plan.uploads.len(), 2);
    assert_eq!(plan.uploads[0].first_slot, 1);
    assert_eq!(plan.uploads[0].instances.len(), 2);
    assert_eq!(plan.uploads[0].instances[0].grid_position, [1, 0]);
    assert_eq!(plan.uploads[0].instances[1].grid_position, [2, 0]);
    assert_eq!(plan.uploads[1].first_slot, 7);
    assert_eq!(plan.uploads[1].instances[0].grid_position, [3, 1]);
}

#[test]
fn replacement_patch_rebuilds_all_slots_after_grid_resize() {
    let previous = Surface::filled(GridSize::new(2, 1), GpuCell::Empty).unwrap();
    let next = Surface::from_cells(
        GridSize::new(1, 2),
        vec![
            sprite(1, Rgba8::new(1, 1, 1, 255)),
            sprite(2, Rgba8::new(2, 2, 2, 255)),
        ],
    )
    .unwrap();

    let plan = plan_patch(
        &diff(&previous, &next),
        &atlas(),
        u32::MAX,
        viewport(),
        GpuClip::Surface,
    )
    .unwrap();

    assert_eq!(plan.mode, SubmissionMode::Replace);
    assert_eq!(plan.grid_size, GridSize::new(1, 2));
    assert_eq!(plan.instance_count, 2);
    assert_eq!(plan.uploads.len(), 2);
    assert_eq!(plan.uploads[0].first_slot, 0);
    assert_eq!(plan.uploads[1].first_slot, 1);
}

#[test]
fn clip_intersects_grid_and_target_in_pixel_coordinates() {
    let surface = Surface::filled(GridSize::new(4, 3), GpuCell::Empty).unwrap();
    let clip = GpuClip::Rect(GridRect::new(GridPos::new(-1, 1), GridSize::new(4, 3)));

    let plan = plan_surface(&surface, &atlas(), u32::MAX, viewport(), clip).unwrap();
    assert_eq!(plan.scissor, Some(PixelRect::new(0, 18, 25, 16)));

    let outside = GpuClip::Rect(GridRect::new(GridPos::new(20, 20), GridSize::new(1, 1)));
    assert_eq!(
        plan_surface(&surface, &atlas(), u32::MAX, viewport(), outside)
            .unwrap()
            .scissor,
        None
    );
}

#[test]
fn resize_reclamps_scissor_and_minimize_suspends_drawing() {
    let surface = Surface::filled(GridSize::new(4, 3), GpuCell::Empty).unwrap();
    let smaller = viewport().resized(PixelSize::new(12, 20));
    let minimized_width = smaller.resized(PixelSize::new(0, 20));
    let minimized_height = smaller.resized(PixelSize::new(20, 0));

    assert_eq!(
        plan_surface(&surface, &atlas(), u32::MAX, smaller, GpuClip::Surface)
            .unwrap()
            .scissor,
        Some(PixelRect::new(0, 10, 12, 10))
    );
    for minimized in [minimized_width, minimized_height] {
        assert_eq!(
            plan_surface(&surface, &atlas(), u32::MAX, minimized, GpuClip::Surface,)
                .unwrap()
                .scissor,
            None
        );
    }
}

#[test]
fn empty_surfaces_produce_no_instances_or_scissor() {
    for size in [GridSize::new(0, 3), GridSize::new(3, 0)] {
        let surface = Surface::from_cells(size, Vec::new()).unwrap();
        let plan =
            plan_surface(&surface, &atlas(), u32::MAX, viewport(), GpuClip::Surface).unwrap();
        assert_eq!(plan.instance_count, 0);
        assert!(plan.uploads.is_empty());
        assert_eq!(plan.scissor, None);
    }
}

#[test]
fn planner_reports_missing_resources_with_their_coordinate() {
    let previous = Surface::filled(GridSize::new(2, 1), GpuCell::Empty).unwrap();
    let surface = Surface::from_cells(
        GridSize::new(2, 1),
        vec![GpuCell::Empty, sprite(99, Rgba8::new(1, 2, 3, 4))],
    )
    .unwrap();
    let error =
        plan_surface(&surface, &atlas(), u32::MAX, viewport(), GpuClip::Surface).unwrap_err();

    assert_eq!(
        error,
        GpuPlanError::MissingResource {
            position: GridPos::new(1, 0),
            resource: ResourceId(99),
        }
    );
    assert!(error.to_string().contains("99"));
    assert_eq!(
        plan_patch(
            &diff(&previous, &surface),
            &atlas(),
            u32::MAX,
            viewport(),
            GpuClip::Surface,
        )
        .unwrap_err(),
        error
    );
}

#[test]
fn planner_enforces_the_supplied_device_instance_limit() {
    let previous = Surface::filled(GridSize::new(2, 1), GpuCell::Empty).unwrap();
    let next = Surface::from_cells(
        GridSize::new(2, 1),
        vec![GpuCell::Empty, sprite(1, Rgba8::new(1, 2, 3, 4))],
    )
    .unwrap();
    let expected = GpuPlanError::InstanceCountOverflow {
        size: GridSize::new(2, 1),
        maximum: 1,
    };

    assert_eq!(
        plan_surface(&next, &atlas(), 1, viewport(), GpuClip::Surface).unwrap_err(),
        expected
    );
    assert_eq!(
        plan_patch(
            &diff(&previous, &next),
            &atlas(),
            1,
            viewport(),
            GpuClip::Surface,
        )
        .unwrap_err(),
        expected
    );
    assert!(expected.to_string().contains("limit 1"));
}

#[test]
fn clip_returns_none_when_mapped_grid_is_outside_the_framebuffer() {
    let surface = Surface::filled(GridSize::new(1, 1), GpuCell::Empty).unwrap();
    for origin in [PixelOffset::new(200, 0), PixelOffset::new(0, 200)] {
        let viewport =
            Viewport::new(PixelSize::new(100, 100), origin, PixelSize::new(10, 10)).unwrap();
        assert_eq!(
            plan_surface(&surface, &atlas(), u32::MAX, viewport, GpuClip::Surface,)
                .unwrap()
                .scissor,
            None
        );
    }
}
