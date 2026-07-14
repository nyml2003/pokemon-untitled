use punctum_grid::{GridSize, PatchApplyError, PatchKind, Surface, apply_patch, diff};

fn surface(size: GridSize, cells: &[u8]) -> Surface<u8> {
    Surface::from_cells(size, cells.to_vec()).unwrap()
}

#[test]
fn unchanged_surfaces_produce_an_empty_delta() {
    let frame = surface(GridSize::new(3, 2), &[0, 1, 2, 3, 4, 5]);
    let patch = diff(&frame, &frame);

    assert_eq!(patch.kind(), PatchKind::Delta);
    assert_eq!(patch.size(), frame.size());
    assert!(patch.spans().is_empty());
    assert_eq!(patch.changed_cell_count(), 0);
}

#[test]
fn diff_groups_contiguous_changes_into_sorted_non_overlapping_spans() {
    let previous = surface(GridSize::new(5, 2), &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let next = surface(GridSize::new(5, 2), &[1, 2, 0, 3, 0, 0, 4, 5, 0, 6]);
    let patch = diff(&previous, &next);
    let spans = patch.spans();

    assert_eq!(patch.kind(), PatchKind::Delta);
    assert_eq!(spans.len(), 4);
    assert_eq!(
        (spans[0].row(), spans[0].start_col(), spans[0].cells()),
        (0, 0, &[1, 2][..])
    );
    assert_eq!(
        (spans[1].row(), spans[1].start_col(), spans[1].cells()),
        (0, 3, &[3][..])
    );
    assert_eq!(
        (spans[2].row(), spans[2].start_col(), spans[2].cells()),
        (1, 1, &[4, 5][..])
    );
    assert_eq!(
        (spans[3].row(), spans[3].start_col(), spans[3].cells()),
        (1, 4, &[6][..])
    );
    assert_eq!(patch.changed_cell_count(), 6);
}

#[test]
fn size_change_produces_a_full_replacement_patch() {
    let previous = surface(GridSize::new(2, 1), &[1, 2]);
    let next = surface(GridSize::new(3, 2), &[3, 4, 5, 6, 7, 8]);
    let patch = diff(&previous, &next);

    assert_eq!(patch.kind(), PatchKind::Replace);
    assert_eq!(patch.size(), next.size());
    assert_eq!(patch.spans().len(), 2);
    assert_eq!(patch.spans()[0].cells(), &[3, 4, 5]);
    assert_eq!(patch.spans()[1].cells(), &[6, 7, 8]);
    assert_eq!(patch.changed_cell_count(), 6);
}

#[test]
fn applying_a_delta_reconstructs_the_next_surface() {
    let mut previous = surface(GridSize::new(3, 2), &[0, 1, 2, 3, 4, 5]);
    let next = surface(GridSize::new(3, 2), &[0, 8, 9, 3, 4, 7]);
    let patch = diff(&previous, &next);

    apply_patch(&mut previous, &patch).unwrap();

    assert_eq!(previous, next);
}

#[test]
fn applying_a_delta_to_the_wrong_size_is_rejected_without_mutation() {
    let old = surface(GridSize::new(2, 1), &[0, 0]);
    let next = surface(GridSize::new(2, 1), &[1, 0]);
    let patch = diff(&old, &next);
    let mut wrong_target = surface(GridSize::new(1, 1), &[9]);
    let before = wrong_target.clone();

    let error = apply_patch(&mut wrong_target, &patch).unwrap_err();

    assert_eq!(
        error,
        PatchApplyError::SizeMismatch {
            surface_size: GridSize::new(1, 1),
            patch_size: GridSize::new(2, 1),
        }
    );
    assert_eq!(wrong_target, before);
    assert!(error.to_string().contains("does not match"));
}

#[test]
fn applying_a_replacement_changes_surface_size_and_cells() {
    let mut previous = surface(GridSize::new(1, 2), &[1, 2]);
    let next = surface(GridSize::new(3, 1), &[4, 5, 6]);
    let patch = diff(&previous, &next);

    apply_patch(&mut previous, &patch).unwrap();

    assert_eq!(previous, next);
}

#[test]
fn replacing_with_an_empty_surface_is_supported() {
    let mut previous = surface(GridSize::new(1, 1), &[1]);
    let next = surface(GridSize::new(0, 3), &[]);
    let patch = diff(&previous, &next);

    apply_patch(&mut previous, &patch).unwrap();

    assert_eq!(patch.kind(), PatchKind::Replace);
    assert!(patch.spans().is_empty());
    assert_eq!(previous, next);
}

#[test]
fn diff_apply_matches_a_scalar_reference_across_many_frames() {
    let sizes = [
        GridSize::new(0, 0),
        GridSize::new(1, 1),
        GridSize::new(4, 3),
        GridSize::new(7, 5),
    ];
    let mut seed = 0x5eed_u64;

    for previous_size in sizes {
        for next_size in sizes {
            for _ in 0..32 {
                let mut make_cells = |size: GridSize| {
                    (0..usize::try_from(u64::from(size.cols) * u64::from(size.rows)).unwrap())
                        .map(|_| {
                            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                            (seed >> 32) as u8
                        })
                        .collect::<Vec<_>>()
                };
                let mut previous =
                    Surface::from_cells(previous_size, make_cells(previous_size)).unwrap();
                let next = Surface::from_cells(next_size, make_cells(next_size)).unwrap();
                let patch = diff(&previous, &next);

                apply_patch(&mut previous, &patch).unwrap();

                assert_eq!(previous, next);
            }
        }
    }
}

#[test]
fn zero_width_diff_handles_the_maximum_row_count() {
    let size = GridSize::new(0, u32::MAX);
    let previous = Surface::<u8>::from_cells(size, Vec::new()).unwrap();
    let next = Surface::<u8>::from_cells(size, Vec::new()).unwrap();
    let patch = diff(&previous, &next);

    assert_eq!(patch.kind(), PatchKind::Delta);
    assert!(patch.spans().is_empty());
}
