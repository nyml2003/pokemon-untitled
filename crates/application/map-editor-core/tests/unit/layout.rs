use super::*;

#[test]
fn row_and_column_allocate_without_overlap() {
    let mut row = Row::new(rect(0, 0, 10, 2), 1);
    assert_eq!(row.take(3), rect(0, 0, 3, 2));
    assert_eq!(row.take(2), rect(4, 0, 2, 2));

    let mut column = Column::new(rect(0, 0, 3, 10), 1);
    assert_eq!(column.take(2), rect(0, 0, 3, 2));
    assert_eq!(column.take(3), rect(0, 3, 3, 3));
}

#[test]
fn interactive_workbench_regions_do_not_overlap() {
    let ui = workbench();
    let mut regions = vec![
        ui.previous_assets,
        ui.next_assets,
        ui.previous_materials,
        ui.next_materials,
        ui.add_layer,
        ui.remove_layer,
        ui.delete_material,
        ui.visual,
        ui.walkable,
        ui.blocked,
        ui.encounter,
        ui.clear_event,
        ui.save,
        ui.undo,
        ui.redo,
        ui.help,
    ];
    regions.extend(ui.asset_slots);
    regions.extend(ui.material_slots);
    let surface = rect(0, 0, COLS, ROWS);
    for (index, region) in regions.iter().enumerate() {
        assert_eq!(region.intersection(surface), Some(*region));
        for other in regions.iter().skip(index + 1) {
            assert!(
                region.intersection(*other).is_none(),
                "{region:?} overlaps {other:?}"
            );
        }
    }
}
