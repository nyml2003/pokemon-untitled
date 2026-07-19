use map_project::{AtomicTileId, CompositeTile, CompositeTileId, MapProject, MapProjectId};
use punctum_gpu::{PixelOffset, PixelSize};

use super::*;

fn model() -> EditorModel {
    let tile = AtomicTileId::new("tile").unwrap();
    EditorModel::new(
        MapProject::blank(
            MapProjectId::new("map").unwrap(),
            16,
            10,
            Some(CompositeTile::new(
                CompositeTileId::new("base").unwrap(),
                vec![tile.clone()],
            )),
        ),
        vec![tile; 20],
    )
}

fn map_viewport() -> EditorMapViewport {
    EditorMapViewport::default()
}

#[test]
fn canvas_and_palette_clicks_produce_intents_without_mutating_the_model() {
    let state = model();
    let viewport = Viewport::new(
        PixelSize::new(1920, 1040),
        PixelOffset::new(0, 0),
        PixelSize::new(40, 40),
    )
    .unwrap();
    let mut controller = EditorController::default();
    (controller, _) = controller.move_cursor(80.0, 120.0, viewport, map_viewport(), &state);
    let (next, intent) = controller.press(PointerButton::Primary, map_viewport(), &state);
    controller = next;
    assert!(matches!(
        intent,
        Some(EditorIntent::Paint {
            position: TilePosition(1, 1),
            ..
        })
    ));
    controller = controller.release(PointerButton::Primary);
    (controller, _) =
        controller.move_cursor(50.5 * 40.0, 2.5 * 40.0, viewport, map_viewport(), &state);
    let (next, intent) = controller.press(PointerButton::Primary, map_viewport(), &state);
    controller = next;
    assert_eq!(intent, Some(EditorIntent::SelectAtomic(0)));
    let (_, intent) = controller.move_cursor(40.0, 40.0, viewport, map_viewport(), &state);
    assert_eq!(
        intent, None,
        "dragging from a UI control must not paint the canvas"
    );
}

#[test]
fn material_pages_are_clickable() {
    let mut model = model();
    let base = model.project.materials[0].clone();
    for index in 1..8 {
        let mut material = base.clone();
        material.id = CompositeTileId::new(format!("material-{index:04}")).unwrap();
        model.project.materials.push(material);
    }
    let viewport = Viewport::new(
        PixelSize::new(1920, 1040),
        PixelOffset::new(0, 0),
        PixelSize::new(40, 40),
    )
    .unwrap();
    let mut controller = EditorController::default();
    (controller, _) =
        controller.move_cursor(41.5 * 40.0, 33.5 * 40.0, viewport, map_viewport(), &model);
    let (_, intent) = controller.press(PointerButton::Primary, map_viewport(), &model);
    assert_eq!(intent, Some(EditorIntent::SelectMaterial(5)));
}

#[test]
fn cursor_mapping_applies_zoom_and_camera_offset() {
    let state = model();
    let viewport = Viewport::new(
        PixelSize::new(1920, 1040),
        PixelOffset::new(0, 0),
        PixelSize::new(40, 40),
    )
    .unwrap();
    let map_viewport = EditorMapViewport::new(1, 4, 2);
    let (controller, _) = EditorController::default().move_cursor(
        3.5 * 40.0,
        4.5 * 40.0,
        viewport,
        map_viewport,
        &state,
    );
    assert_eq!(controller.hover, Some(TilePosition::new(7, 6)));
}

#[test]
fn delete_material_button_produces_delete_intent() {
    let state = model();
    let button = layout::workbench().delete_material;

    assert_eq!(
        click_intent(button.origin, &state),
        Some(EditorIntent::DeleteMaterial)
    );
}

#[test]
fn every_workbench_control_maps_to_one_intent() {
    let state = model();
    let ui = layout::workbench();
    let cases = [
        (ui.previous_assets, EditorIntent::SelectAtomic(0)),
        (ui.next_assets, EditorIntent::SelectAtomic(15)),
        (ui.previous_materials, EditorIntent::SelectMaterial(0)),
        (ui.next_materials, EditorIntent::SelectMaterial(0)),
        (ui.add_layer, EditorIntent::AddLayer),
        (ui.remove_layer, EditorIntent::RemoveLayer),
        (ui.delete_material, EditorIntent::DeleteMaterial),
        (ui.save, EditorIntent::Save),
        (ui.undo, EditorIntent::Undo),
        (ui.redo, EditorIntent::Redo),
        (ui.help, EditorIntent::ToggleHelp),
        (ui.visual, EditorIntent::SelectTool(EditorTool::Visual)),
        (
            ui.walkable,
            EditorIntent::SelectTool(EditorTool::Collision(Collision::Walkable)),
        ),
        (
            ui.blocked,
            EditorIntent::SelectTool(EditorTool::Collision(Collision::Blocked)),
        ),
        (
            ui.encounter,
            EditorIntent::SelectTool(EditorTool::Event(Some(MapEventKind::Encounter))),
        ),
        (
            ui.clear_event,
            EditorIntent::SelectTool(EditorTool::Event(None)),
        ),
    ];
    for (rect, expected) in cases {
        assert_eq!(click_intent(rect.origin, &state), Some(expected));
    }
    assert_eq!(click_intent(GridPos::new(63, 37), &state), None);

    let mut empty = model();
    empty.atomic_ids.clear();
    empty.project.materials.clear();
    assert_eq!(click_intent(ui.asset_slots[0].origin, &empty), None);
    assert_eq!(click_intent(ui.material_slots[0].origin, &empty), None);
}

#[test]
fn pointer_reducer_covers_drag_help_release_and_leave() {
    let viewport = Viewport::new(
        PixelSize::new(1920, 1040),
        PixelOffset::new(0, 0),
        PixelSize::new(40, 40),
    )
    .unwrap();
    let state = model();
    let (controller, intent) =
        EditorController::default().press(PointerButton::Primary, map_viewport(), &state);
    assert_eq!(intent, None);
    let (controller, intent) = controller.move_cursor(-1.0, -1.0, viewport, map_viewport(), &state);
    assert_eq!(intent, None);
    let (controller, _) = controller.move_cursor(80.0, 120.0, viewport, map_viewport(), &state);
    let (controller, intent) = controller.press(PointerButton::Secondary, map_viewport(), &state);
    assert!(matches!(
        intent,
        Some(EditorIntent::Paint { erase: true, .. })
    ));
    let (controller, intent) =
        controller.move_cursor(80.0, 120.0, viewport, map_viewport(), &state);
    assert_eq!(intent, None);
    let (controller, intent) =
        controller.move_cursor(160.0, 120.0, viewport, map_viewport(), &state);
    assert!(matches!(
        intent,
        Some(EditorIntent::Paint { erase: true, .. })
    ));
    let controller = controller.release(PointerButton::Primary);
    let controller = controller.release(PointerButton::Secondary);
    assert_eq!(controller.clone().leave().hover, None);

    let mut help = model();
    help.show_help = true;
    let help_button = layout::workbench().help.origin;
    let x = f64::from(help_button.col * 40 + 1);
    let y = f64::from(help_button.row * 40 + 1);
    let (controller, intent) =
        EditorController::default().move_cursor(x, y, viewport, map_viewport(), &help);
    assert_eq!(intent, None);
    let (controller, intent) = controller.press(PointerButton::Secondary, map_viewport(), &help);
    assert_eq!(intent, None);
    let (_, intent) = controller.press(PointerButton::Primary, map_viewport(), &help);
    assert_eq!(intent, Some(EditorIntent::ToggleHelp));

    let (controller, _) = EditorController::default().move_cursor(
        60.0 * 40.0,
        10.0 * 40.0,
        viewport,
        map_viewport(),
        &state,
    );
    let (_, intent) = controller.press(PointerButton::Secondary, map_viewport(), &state);
    assert_eq!(intent, None);
}
