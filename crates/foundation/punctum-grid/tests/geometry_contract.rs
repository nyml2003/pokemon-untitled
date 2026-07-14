use punctum_grid::{GridPos, GridRect, GridSize};

#[test]
fn empty_size_reports_empty_when_either_axis_is_zero() {
    assert!(GridSize::new(0, 4).is_empty());
    assert!(GridSize::new(4, 0).is_empty());
    assert!(!GridSize::new(4, 3).is_empty());
}

#[test]
fn rect_contains_left_top_but_excludes_right_bottom() {
    let rect = GridRect::new(GridPos::new(-2, 3), GridSize::new(4, 2));

    assert!(rect.contains(GridPos::new(-2, 3)));
    assert!(rect.contains(GridPos::new(1, 4)));
    assert!(!rect.contains(GridPos::new(2, 4)));
    assert!(!rect.contains(GridPos::new(1, 5)));
}

#[test]
fn rect_intersection_returns_the_shared_region() {
    let left = GridRect::new(GridPos::new(-2, 1), GridSize::new(6, 4));
    let right = GridRect::new(GridPos::new(1, -1), GridSize::new(4, 5));

    assert_eq!(
        left.intersection(right),
        Some(GridRect::new(GridPos::new(1, 1), GridSize::new(3, 3)))
    );
}

#[test]
fn rect_intersection_returns_none_without_positive_area() {
    let rect = GridRect::new(GridPos::new(0, 0), GridSize::new(2, 2));
    let touching = GridRect::new(GridPos::new(2, 0), GridSize::new(2, 2));
    let empty = GridRect::new(GridPos::new(0, 0), GridSize::new(0, 2));

    assert_eq!(rect.intersection(touching), None);
    assert_eq!(rect.intersection(empty), None);
}

#[test]
fn rect_clips_negative_origin_to_grid_bounds() {
    let rect = GridRect::new(GridPos::new(-2, -1), GridSize::new(5, 4));

    assert_eq!(
        rect.clip_to(GridSize::new(4, 2)),
        Some(GridRect::new(GridPos::new(0, 0), GridSize::new(3, 2)))
    );
}

#[test]
fn rect_outside_grid_clips_to_none() {
    let rect = GridRect::new(GridPos::new(5, 5), GridSize::new(2, 2));

    assert_eq!(rect.clip_to(GridSize::new(4, 4)), None);
}
