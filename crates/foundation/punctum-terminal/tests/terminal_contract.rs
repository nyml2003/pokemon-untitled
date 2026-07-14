use punctum_grid::{GridPos, GridSize, Surface, diff};
use punctum_terminal::{
    TerminalCell, TerminalCellError, TerminalColor, TerminalPlanError, TerminalTextError,
    plan_patch, resize_text_surface, write_text,
};

fn patch_with_one_changed_cell(
    size: GridSize,
    position: GridPos,
    cell: TerminalCell,
) -> punctum_grid::Patch<TerminalCell> {
    let previous = Surface::filled(size, TerminalCell::default()).unwrap();
    let mut next = previous.clone();
    next.set(position, cell).unwrap();
    diff(&previous, &next)
}

#[test]
fn terminal_cell_preserves_symbol_and_colors() {
    let cell = TerminalCell::new('x', TerminalColor::White, TerminalColor::Blue);

    assert_eq!(cell.grapheme(), Some("x"));
    assert_eq!(cell.foreground(), TerminalColor::White);
    assert_eq!(cell.background(), TerminalColor::Blue);
    assert!(!cell.is_continuation());
}

#[test]
fn terminal_cell_defaults_to_a_blank_with_default_colors() {
    assert_eq!(
        TerminalCell::default(),
        TerminalCell::new(' ', TerminalColor::Default, TerminalColor::Default)
    );
}

#[test]
fn terminal_cell_accepts_exactly_one_grapheme() {
    let combining =
        TerminalCell::from_grapheme("e\u{301}", TerminalColor::White, TerminalColor::Black)
            .unwrap();
    let emoji =
        TerminalCell::from_grapheme("👩‍💻", TerminalColor::White, TerminalColor::Black).unwrap();

    assert_eq!(combining.grapheme(), Some("e\u{301}"));
    assert_eq!(emoji.grapheme(), Some("👩‍💻"));
    assert_eq!(
        TerminalCell::from_grapheme("", TerminalColor::White, TerminalColor::Black),
        Err(TerminalCellError::EmptyGrapheme)
    );
    assert_eq!(
        TerminalCell::from_grapheme("ab", TerminalColor::White, TerminalColor::Black),
        Err(TerminalCellError::MultipleGraphemes)
    );
    assert!(
        TerminalCellError::EmptyGrapheme
            .to_string()
            .contains("empty")
    );
    assert!(
        TerminalCellError::MultipleGraphemes
            .to_string()
            .contains("one grapheme")
    );
}

#[test]
fn write_text_expands_unicode_into_lead_and_continuation_cells() {
    let mut surface = Surface::filled(GridSize::new(8, 1), TerminalCell::default()).unwrap();

    let cursor = write_text(
        &mut surface,
        GridPos::new(0, 0),
        "Ae\u{301}界👩‍💻",
        TerminalColor::White,
        TerminalColor::Black,
    )
    .unwrap();

    assert_eq!(cursor, GridPos::new(6, 0));
    assert_eq!(
        surface.get(GridPos::new(0, 0)).unwrap().grapheme(),
        Some("A")
    );
    assert_eq!(
        surface.get(GridPos::new(1, 0)).unwrap().grapheme(),
        Some("e\u{301}")
    );
    assert_eq!(
        surface.get(GridPos::new(2, 0)).unwrap().grapheme(),
        Some("界")
    );
    assert!(surface.get(GridPos::new(3, 0)).unwrap().is_continuation());
    assert_eq!(
        surface.get(GridPos::new(4, 0)).unwrap().grapheme(),
        Some("👩‍💻")
    );
    assert!(surface.get(GridPos::new(5, 0)).unwrap().is_continuation());
}

#[test]
fn overwriting_either_wide_grapheme_slot_clears_the_other_slot() {
    for overwrite_col in [0, 1] {
        let mut surface = Surface::filled(GridSize::new(3, 1), TerminalCell::default()).unwrap();
        write_text(
            &mut surface,
            GridPos::new(0, 0),
            "界",
            TerminalColor::White,
            TerminalColor::Black,
        )
        .unwrap();

        write_text(
            &mut surface,
            GridPos::new(overwrite_col, 0),
            "x",
            TerminalColor::Red,
            TerminalColor::Black,
        )
        .unwrap();

        let other_col = 1 - overwrite_col;
        assert_eq!(
            surface
                .get(GridPos::new(overwrite_col, 0))
                .unwrap()
                .grapheme(),
            Some("x")
        );
        assert_eq!(
            surface.get(GridPos::new(other_col, 0)).unwrap(),
            &TerminalCell::default()
        );
    }
}

#[test]
fn write_text_clips_a_wide_grapheme_as_a_whole() {
    let mut surface = Surface::filled(GridSize::new(3, 1), TerminalCell::default()).unwrap();

    let cursor = write_text(
        &mut surface,
        GridPos::new(2, 0),
        "界",
        TerminalColor::White,
        TerminalColor::Black,
    )
    .unwrap();

    assert_eq!(cursor, GridPos::new(3, 0));
    assert_eq!(
        surface.get(GridPos::new(2, 0)).unwrap(),
        &TerminalCell::default()
    );
}

#[test]
fn write_text_rejects_each_out_of_bounds_direction() {
    let size = GridSize::new(2, 2);
    for position in [
        GridPos::new(-1, 0),
        GridPos::new(0, -1),
        GridPos::new(2, 0),
        GridPos::new(0, 2),
    ] {
        let mut surface = Surface::filled(size, TerminalCell::default()).unwrap();
        let error = write_text(
            &mut surface,
            position,
            "x",
            TerminalColor::White,
            TerminalColor::Black,
        )
        .unwrap_err();

        assert_eq!(
            error,
            TerminalTextError::PositionOutOfBounds { position, size }
        );
        assert!(error.to_string().contains("outside"));
    }
}

#[test]
fn write_text_treats_empty_text_as_a_no_op() {
    let mut surface = Surface::filled(
        GridSize::new(2, 1),
        TerminalCell::new('x', TerminalColor::Red, TerminalColor::Blue),
    )
    .unwrap();
    let before = surface.clone();
    let position = GridPos::new(1, 0);

    let cursor = write_text(
        &mut surface,
        position,
        "",
        TerminalColor::White,
        TerminalColor::Black,
    )
    .unwrap();

    assert_eq!(cursor, position);
    assert_eq!(surface, before);
}

#[test]
fn write_text_ignores_zero_width_graphemes_and_stops_at_the_row_end() {
    let mut surface = Surface::filled(GridSize::new(1, 1), TerminalCell::default()).unwrap();

    let cursor = write_text(
        &mut surface,
        GridPos::new(0, 0),
        "\u{301}xy",
        TerminalColor::White,
        TerminalColor::Black,
    )
    .unwrap();

    assert_eq!(cursor, GridPos::new(1, 0));
    assert_eq!(surface.cells()[0].grapheme(), Some("x"));
}

#[test]
fn resize_never_keeps_half_of_a_wide_grapheme() {
    let mut surface = Surface::filled(GridSize::new(3, 1), TerminalCell::default()).unwrap();
    write_text(
        &mut surface,
        GridPos::new(1, 0),
        "界",
        TerminalColor::White,
        TerminalColor::Black,
    )
    .unwrap();

    let clipped = resize_text_surface(&surface, GridSize::new(2, 1)).unwrap();
    let expanded = resize_text_surface(&clipped, GridSize::new(4, 2)).unwrap();

    assert_eq!(
        clipped.cells(),
        &[TerminalCell::default(), TerminalCell::default()]
    );
    assert!(expanded.cells().iter().all(|cell| !cell.is_continuation()));
}

#[test]
fn resize_preserves_complete_pairs_and_cleans_orphan_continuations() {
    let mut surface = Surface::filled(GridSize::new(2, 1), TerminalCell::default()).unwrap();
    write_text(
        &mut surface,
        GridPos::new(0, 0),
        "界",
        TerminalColor::White,
        TerminalColor::Black,
    )
    .unwrap();

    let preserved = resize_text_surface(&surface, GridSize::new(3, 1)).unwrap();
    assert_eq!(
        preserved.get(GridPos::new(0, 0)).unwrap().grapheme(),
        Some("界")
    );
    assert!(preserved.get(GridPos::new(1, 0)).unwrap().is_continuation());

    surface
        .set(GridPos::new(0, 0), TerminalCell::default())
        .unwrap();
    let cleaned = resize_text_surface(&surface, GridSize::new(2, 1)).unwrap();
    assert!(cleaned.cells().iter().all(|cell| !cell.is_continuation()));
}

#[test]
fn resize_reports_surface_capacity_overflow() {
    let surface = Surface::filled(GridSize::new(0, 0), TerminalCell::default()).unwrap();

    assert!(resize_text_surface(&surface, GridSize::new(u32::MAX, u32::MAX)).is_err());
}

#[test]
fn plan_patch_scales_logical_columns_without_changing_rows() {
    let changed = TerminalCell::new('x', TerminalColor::Red, TerminalColor::Black);
    let patch =
        patch_with_one_changed_cell(GridSize::new(3, 2), GridPos::new(1, 1), changed.clone());

    let runs = plan_patch(&patch, 2).unwrap();

    assert_eq!(runs.runs().len(), 1);
    assert_eq!(runs.runs()[0].col(), 2);
    assert_eq!(runs.runs()[0].row(), 1);
    assert_eq!(runs.runs()[0].cells(), &[changed]);
    assert_eq!(runs.final_cursor(), (0, 0));
}

#[test]
fn plan_patch_keeps_replacement_rows_in_order() {
    let empty = Surface::filled(GridSize::new(0, 0), TerminalCell::default()).unwrap();
    let next = Surface::from_cells(
        GridSize::new(2, 2),
        vec![
            TerminalCell::new('a', TerminalColor::White, TerminalColor::Black),
            TerminalCell::new('b', TerminalColor::White, TerminalColor::Black),
            TerminalCell::new('c', TerminalColor::White, TerminalColor::Black),
            TerminalCell::new('d', TerminalColor::White, TerminalColor::Black),
        ],
    )
    .unwrap();

    let runs = plan_patch(&diff(&empty, &next), 1).unwrap();

    assert_eq!(runs.runs().len(), 2);
    assert_eq!((runs.runs()[0].col(), runs.runs()[0].row()), (0, 0));
    assert_eq!((runs.runs()[1].col(), runs.runs()[1].row()), (0, 1));
    assert_eq!(runs.runs()[0].cells()[0].grapheme(), Some("a"));
    assert_eq!(runs.runs()[1].cells()[1].grapheme(), Some("d"));
}

#[test]
fn plan_patch_preserves_complete_wide_pairs_and_replaces_unpaired_graphemes() {
    let empty = Surface::filled(GridSize::new(2, 1), TerminalCell::default()).unwrap();
    let mut paired = empty.clone();
    write_text(
        &mut paired,
        GridPos::new(0, 0),
        "界",
        TerminalColor::Yellow,
        TerminalColor::Blue,
    )
    .unwrap();
    let paired_plan = plan_patch(&diff(&empty, &paired), 1).unwrap();

    assert_eq!(paired_plan.runs()[0].cells()[0].grapheme(), Some("界"));
    assert!(paired_plan.runs()[0].cells()[1].is_continuation());

    let wide =
        TerminalCell::from_grapheme("界", TerminalColor::Yellow, TerminalColor::Blue).unwrap();
    let patch = patch_with_one_changed_cell(GridSize::new(1, 1), GridPos::new(0, 0), wide);

    let plan = plan_patch(&patch, 1).unwrap();
    let fallback = &plan.runs()[0].cells()[0];

    assert_eq!(fallback.grapheme(), Some("\u{fffd}"));
    assert_eq!(fallback.foreground(), TerminalColor::Yellow);
    assert_eq!(fallback.background(), TerminalColor::Blue);
}

#[test]
fn plan_patch_replaces_zero_width_and_orphan_continuation_cells() {
    let zero_width =
        TerminalCell::from_grapheme("\u{301}", TerminalColor::White, TerminalColor::Black).unwrap();
    let zero_width_patch =
        patch_with_one_changed_cell(GridSize::new(1, 1), GridPos::new(0, 0), zero_width);
    assert_eq!(
        plan_patch(&zero_width_patch, 1).unwrap().runs()[0].cells()[0].grapheme(),
        Some("\u{fffd}")
    );

    let mut next = Surface::filled(GridSize::new(2, 1), TerminalCell::default()).unwrap();
    write_text(
        &mut next,
        GridPos::new(0, 0),
        "界",
        TerminalColor::White,
        TerminalColor::Black,
    )
    .unwrap();
    let mut previous = next.clone();
    previous
        .set(GridPos::new(1, 0), TerminalCell::default())
        .unwrap();
    let plan = plan_patch(&diff(&previous, &next), 1).unwrap();

    assert_eq!(plan.runs()[0].cells()[0].grapheme(), Some("\u{fffd}"));
}

#[test]
fn plan_patch_rejects_zero_width_cells() {
    let patch = patch_with_one_changed_cell(
        GridSize::new(1, 1),
        GridPos::new(0, 0),
        TerminalCell::default(),
    );

    assert_eq!(plan_patch(&patch, 0), Err(TerminalPlanError::ZeroCellWidth));
}

#[test]
fn plan_patch_rejects_a_scaled_column_that_exceeds_terminal_coordinates() {
    let patch = patch_with_one_changed_cell(
        GridSize::new(32_769, 1),
        GridPos::new(32_768, 0),
        TerminalCell::new('x', TerminalColor::White, TerminalColor::Black),
    );

    assert_eq!(
        plan_patch(&patch, 2),
        Err(TerminalPlanError::CoordinateOverflow {
            col: 32_768,
            row: 0,
            cell_width: 2,
        })
    );
}

#[test]
fn plan_patch_rejects_a_row_that_exceeds_terminal_coordinates() {
    let patch = patch_with_one_changed_cell(
        GridSize::new(1, 65_537),
        GridPos::new(0, 65_536),
        TerminalCell::new('x', TerminalColor::White, TerminalColor::Black),
    );

    assert_eq!(
        plan_patch(&patch, 1),
        Err(TerminalPlanError::CoordinateOverflow {
            col: 0,
            row: 65_536,
            cell_width: 1,
        })
    );
}

#[test]
fn terminal_plan_errors_have_actionable_messages() {
    assert!(
        TerminalPlanError::ZeroCellWidth
            .to_string()
            .contains("width")
    );
    assert!(
        TerminalPlanError::CoordinateOverflow {
            col: 1,
            row: 2,
            cell_width: 3,
        }
        .to_string()
        .contains("1")
    );
}
