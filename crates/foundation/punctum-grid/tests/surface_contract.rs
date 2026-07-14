use punctum_grid::{GridPos, GridRect, GridSize, Surface, SurfaceError};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

fn numbered_surface() -> Surface<u8> {
    Surface::from_cells(GridSize::new(3, 2), vec![0, 1, 2, 3, 4, 5]).unwrap()
}

#[test]
fn filled_surface_uses_dense_row_major_storage() {
    let surface = Surface::filled(GridSize::new(3, 2), 7_u8).unwrap();

    assert_eq!(surface.size(), GridSize::new(3, 2));
    assert_eq!(surface.cells(), &[7, 7, 7, 7, 7, 7]);
}

#[test]
fn from_cells_rejects_a_length_that_does_not_match_the_size() {
    let error = Surface::from_cells(GridSize::new(2, 2), vec![1_u8, 2, 3]).unwrap_err();

    assert_eq!(
        error,
        SurfaceError::LengthMismatch {
            size: GridSize::new(2, 2),
            expected: 4,
            actual: 3,
        }
    );
}

#[test]
fn constructors_reject_capacity_overflow_before_allocating() {
    let size = GridSize::new(u32::MAX, u32::MAX);

    assert_eq!(
        Surface::filled(size, 0_u8).unwrap_err(),
        SurfaceError::CapacityOverflow { size }
    );
    assert_eq!(
        Surface::<u8>::from_cells(size, Vec::new()).unwrap_err(),
        SurfaceError::CapacityOverflow { size }
    );
}

#[test]
fn get_reads_a_cell_by_signed_grid_position() {
    let surface = numbered_surface();

    assert_eq!(surface.get(GridPos::new(2, 1)), Ok(&5));
}

#[test]
fn set_replaces_a_cell_by_signed_grid_position() {
    let mut surface = numbered_surface();

    surface.set(GridPos::new(1, 1), 9).unwrap();

    assert_eq!(surface.cells(), &[0, 1, 2, 3, 9, 5]);
}

#[test]
fn cell_access_rejects_negative_and_high_positions() {
    let mut surface = numbered_surface();
    let negative = GridPos::new(-1, 0);
    let high = GridPos::new(3, 1);

    assert_eq!(
        surface.get(negative),
        Err(SurfaceError::PositionOutOfBounds {
            position: negative,
            size: GridSize::new(3, 2),
        })
    );
    assert_eq!(
        surface.set(high, 9),
        Err(SurfaceError::PositionOutOfBounds {
            position: high,
            size: GridSize::new(3, 2),
        })
    );
}

#[test]
fn fill_replaces_every_cell() {
    let mut surface = numbered_surface();

    surface.fill(8);

    assert_eq!(surface.cells(), &[8, 8, 8, 8, 8, 8]);
}

#[test]
fn strict_fill_rect_changes_only_the_requested_region() {
    let mut surface = Surface::filled(GridSize::new(4, 3), 0_u8).unwrap();

    surface
        .fill_rect(GridRect::new(GridPos::new(1, 1), GridSize::new(2, 2)), 5)
        .unwrap();

    assert_eq!(surface.cells(), &[0, 0, 0, 0, 0, 5, 5, 0, 0, 5, 5, 0]);
}

#[test]
fn strict_fill_rect_accepts_an_empty_rect_at_the_boundary() {
    let mut surface = numbered_surface();

    surface
        .fill_rect(GridRect::new(GridPos::new(3, 2), GridSize::new(0, 0)), 9)
        .unwrap();

    assert_eq!(surface, numbered_surface());
}

#[test]
fn strict_fill_rect_rejects_out_of_bounds_without_mutation() {
    let mut surface = numbered_surface();
    let before = surface.clone();
    let rect = GridRect::new(GridPos::new(-1, 0), GridSize::new(2, 1));

    assert_eq!(
        surface.fill_rect(rect, 9),
        Err(SurfaceError::RectOutOfBounds {
            rect,
            size: GridSize::new(3, 2),
        })
    );
    assert_eq!(surface, before);
}

#[test]
fn clipped_fill_rect_reports_and_changes_only_the_intersection() {
    let mut surface = Surface::filled(GridSize::new(3, 2), 0_u8).unwrap();
    let rect = GridRect::new(GridPos::new(-1, 1), GridSize::new(3, 2));

    let written = surface.fill_rect_clipped(rect, 4);

    assert_eq!(
        written,
        Some(GridRect::new(GridPos::new(0, 1), GridSize::new(2, 1)))
    );
    assert_eq!(surface.cells(), &[0, 0, 0, 4, 4, 0]);
}

#[test]
fn clipped_fill_rect_returns_none_when_disjoint() {
    let mut surface = numbered_surface();
    let before = surface.clone();

    let written =
        surface.fill_rect_clipped(GridRect::new(GridPos::new(5, 5), GridSize::new(2, 2)), 4);

    assert_eq!(written, None);
    assert_eq!(surface, before);
}

#[test]
fn strict_blit_copies_the_source_at_the_destination() {
    let mut target = Surface::filled(GridSize::new(4, 3), 0_u8).unwrap();
    let source = Surface::from_cells(GridSize::new(2, 2), vec![1, 2, 3, 4]).unwrap();

    target.blit(GridPos::new(1, 1), &source).unwrap();

    assert_eq!(target.cells(), &[0, 0, 0, 0, 0, 1, 2, 0, 0, 3, 4, 0]);
}

#[test]
fn strict_blit_rejects_out_of_bounds_without_mutation() {
    let mut target = numbered_surface();
    let before = target.clone();
    let source = Surface::filled(GridSize::new(2, 2), 9_u8).unwrap();
    let destination = GridPos::new(2, 1);
    let rect = GridRect::new(destination, source.size());

    assert_eq!(
        target.blit(destination, &source),
        Err(SurfaceError::RectOutOfBounds {
            rect,
            size: target.size(),
        })
    );
    assert_eq!(target, before);
}

#[test]
fn clipped_blit_handles_a_negative_destination() {
    let mut target = Surface::filled(GridSize::new(3, 2), 0_u8).unwrap();
    let source = Surface::from_cells(GridSize::new(3, 2), vec![1, 2, 3, 4, 5, 6]).unwrap();

    let written = target.blit_clipped(GridPos::new(-1, 0), &source);

    assert_eq!(
        written,
        Some(GridRect::new(GridPos::new(0, 0), GridSize::new(2, 2)))
    );
    assert_eq!(target.cells(), &[2, 3, 0, 5, 6, 0]);
}

#[test]
fn clipped_blit_returns_none_when_disjoint() {
    let mut target = numbered_surface();
    let before = target.clone();
    let source = Surface::filled(GridSize::new(2, 2), 9_u8).unwrap();

    assert_eq!(target.blit_clipped(GridPos::new(5, 5), &source), None);
    assert_eq!(target, before);
}

#[test]
fn zero_width_writes_handle_the_maximum_row_count() {
    let size = GridSize::new(0, u32::MAX);
    let mut target = Surface::<u8>::from_cells(size, Vec::new()).unwrap();
    let source = Surface::<u8>::from_cells(size, Vec::new()).unwrap();

    target
        .fill_rect(GridRect::new(GridPos::new(0, 0), size), 1)
        .unwrap();
    target.blit(GridPos::new(0, 0), &source).unwrap();

    assert_eq!(target, source);
}

#[test]
fn surface_errors_have_actionable_messages() {
    let size = GridSize::new(2, 2);
    let position = GridPos::new(-1, 0);
    let rect = GridRect::new(position, GridSize::new(1, 1));

    assert!(
        SurfaceError::CapacityOverflow { size }
            .to_string()
            .contains("capacity")
    );
    assert!(
        SurfaceError::LengthMismatch {
            size,
            expected: 4,
            actual: 3,
        }
        .to_string()
        .contains("3")
    );
    assert!(
        SurfaceError::PositionOutOfBounds { position, size }
            .to_string()
            .contains("outside")
    );
    assert!(
        SurfaceError::RectOutOfBounds { rect, size }
            .to_string()
            .contains("outside")
    );
}

#[derive(Debug)]
struct CloneProbe(Arc<AtomicUsize>);

impl Clone for CloneProbe {
    fn clone(&self) -> Self {
        self.0.fetch_add(1, Ordering::Relaxed);
        Self(Arc::clone(&self.0))
    }
}

#[test]
fn clone_probe_surface_rejects_a_length_mismatch() {
    let error = Surface::<CloneProbe>::from_cells(GridSize::new(1, 1), Vec::new()).unwrap_err();

    assert_eq!(
        error,
        SurfaceError::LengthMismatch {
            size: GridSize::new(1, 1),
            expected: 1,
            actual: 0,
        }
    );
}

#[test]
fn clone_probe_surface_rejects_capacity_overflow() {
    let size = GridSize::new(u32::MAX, u32::MAX);
    let error = Surface::<CloneProbe>::from_cells(size, Vec::new()).unwrap_err();

    assert_eq!(error, SurfaceError::CapacityOverflow { size });
}

#[test]
fn clone_probe_surface_fills_a_nonempty_rect() {
    let initial = Arc::new(AtomicUsize::new(0));
    let replacement = Arc::new(AtomicUsize::new(0));
    let mut surface =
        Surface::from_cells(GridSize::new(1, 1), vec![CloneProbe(Arc::clone(&initial))]).unwrap();

    surface
        .fill_rect(
            GridRect::new(GridPos::new(0, 0), GridSize::new(1, 1)),
            CloneProbe(Arc::clone(&replacement)),
        )
        .unwrap();

    assert!(Arc::ptr_eq(&surface.cells()[0].0, &replacement));
}

#[test]
fn clone_probe_zero_area_fill_still_rejects_out_of_bounds() {
    let clones = Arc::new(AtomicUsize::new(0));
    let mut surface = Surface::<CloneProbe>::from_cells(GridSize::new(0, 3), Vec::new()).unwrap();
    let rect = GridRect::new(GridPos::new(-1, 0), GridSize::new(0, 3));

    let error = surface
        .fill_rect(rect, CloneProbe(Arc::clone(&clones)))
        .unwrap_err();

    assert_eq!(
        error,
        SurfaceError::RectOutOfBounds {
            rect,
            size: GridSize::new(0, 3),
        }
    );
    assert_eq!(clones.load(Ordering::Relaxed), 0);
}

#[test]
fn strict_fill_rect_does_not_clone_for_zero_area() {
    let clones = Arc::new(AtomicUsize::new(0));
    let mut surface = Surface::<CloneProbe>::from_cells(GridSize::new(0, 3), Vec::new()).unwrap();

    surface
        .fill_rect(
            GridRect::new(GridPos::new(0, 0), GridSize::new(0, 3)),
            CloneProbe(Arc::clone(&clones)),
        )
        .unwrap();

    assert_eq!(clones.load(Ordering::Relaxed), 0);
}
