use map_project::{Collision, MapEventKind, TilePosition};
use punctum_gpu::Viewport;
use punctum_grid::GridPos;

use crate::{
    layout,
    model::{EditorIntent, EditorModel, EditorTool},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerButton {
    Primary,
    Secondary,
}

/// Pure map-to-workbench coordinate transform shared by the renderer and
/// pointer controller. It deliberately contains no window or GPU concepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditorMapViewport {
    pub tile_span: u32,
    pub camera_col: i32,
    pub camera_row: i32,
}

impl EditorMapViewport {
    pub const fn new(tile_span: u32, camera_col: i32, camera_row: i32) -> Self {
        assert!(tile_span > 0, "map tile span must be positive");
        Self {
            tile_span,
            camera_col,
            camera_row,
        }
    }
}

impl Default for EditorMapViewport {
    fn default() -> Self {
        Self::new(layout::MAP_TILE_SPAN, 0, 0)
    }
}

#[derive(Clone, Default)]
pub struct EditorController {
    pub hover: Option<TilePosition>,
    cursor: Option<GridPos>,
    pressed: Option<PointerButton>,
    last_painted: Option<TilePosition>,
}

impl EditorController {
    pub fn move_cursor(
        mut self,
        x: f64,
        y: f64,
        viewport: Viewport,
        map_viewport: EditorMapViewport,
        model: &EditorModel,
    ) -> (Self, Option<EditorIntent>) {
        self.cursor = grid_position(x, y, viewport);
        if model.show_help {
            self.hover = None;
            return (self, None);
        }
        self.hover = self
            .cursor
            .and_then(|position| map_position(position, map_viewport, model));
        let Some(position) = self.hover else {
            return (self, None);
        };
        let Some(button) = self.pressed else {
            return (self, None);
        };
        if self.last_painted == Some(position) {
            return (self, None);
        }
        self.last_painted = Some(position);
        (
            self,
            Some(EditorIntent::Paint {
                position,
                erase: button == PointerButton::Secondary,
            }),
        )
    }

    pub fn press(
        mut self,
        button: PointerButton,
        map_viewport: EditorMapViewport,
        model: &EditorModel,
    ) -> (Self, Option<EditorIntent>) {
        let Some(grid) = self.cursor else {
            return (self, None);
        };
        if model.show_help {
            let ui = layout::workbench();
            let intent = (button == PointerButton::Primary && ui.help.contains(grid))
                .then_some(EditorIntent::ToggleHelp);
            return (self, intent);
        }
        if let Some(position) = map_position(grid, map_viewport, model) {
            self.pressed = Some(button);
            self.last_painted = Some(position);
            return (
                self,
                Some(EditorIntent::Paint {
                    position,
                    erase: button == PointerButton::Secondary,
                }),
            );
        }
        if button == PointerButton::Secondary {
            return (self, None);
        }
        self.pressed = None;
        self.last_painted = None;
        let intent = click_intent(grid, model);
        (self, intent)
    }

    pub fn release(mut self, button: PointerButton) -> Self {
        if self.pressed == Some(button) {
            self.pressed = None;
            self.last_painted = None;
        }
        self
    }

    pub fn leave(mut self) -> Self {
        self.hover = None;
        self.cursor = None;
        self.pressed = None;
        self.last_painted = None;
        self
    }
}

fn click_intent(position: GridPos, model: &EditorModel) -> Option<EditorIntent> {
    let ui = layout::workbench();
    let asset_page = model.selected_atomic / layout::ASSET_PAGE_SIZE;
    for local in 0..layout::ASSET_PAGE_SIZE {
        if ui.asset_slots[local].contains(position) {
            let index = asset_page * layout::ASSET_PAGE_SIZE + local;
            return (index < model.atomic_ids.len()).then_some(EditorIntent::SelectAtomic(index));
        }
    }
    let material_page = model.selected_material / layout::MATERIAL_PAGE_SIZE;
    for local in 0..layout::MATERIAL_PAGE_SIZE {
        if ui.material_slots[local].contains(position) {
            let index = material_page * layout::MATERIAL_PAGE_SIZE + local;
            return (index < model.project.materials.len())
                .then_some(EditorIntent::SelectMaterial(index));
        }
    }
    if ui.previous_assets.contains(position) {
        let page = asset_page.saturating_sub(1);
        return Some(EditorIntent::SelectAtomic(page * layout::ASSET_PAGE_SIZE));
    }
    if ui.next_assets.contains(position) {
        let maximum_page = model.atomic_ids.len().saturating_sub(1) / layout::ASSET_PAGE_SIZE;
        let page = (asset_page + 1).min(maximum_page);
        return Some(EditorIntent::SelectAtomic(page * layout::ASSET_PAGE_SIZE));
    }
    if ui.previous_materials.contains(position) {
        let page = material_page.saturating_sub(1);
        return Some(EditorIntent::SelectMaterial(
            page * layout::MATERIAL_PAGE_SIZE,
        ));
    }
    if ui.next_materials.contains(position) {
        let maximum_page =
            model.project.materials.len().saturating_sub(1) / layout::MATERIAL_PAGE_SIZE;
        let page = (material_page + 1).min(maximum_page);
        return Some(EditorIntent::SelectMaterial(
            page * layout::MATERIAL_PAGE_SIZE,
        ));
    }
    if ui.add_layer.contains(position) {
        return Some(EditorIntent::AddLayer);
    }
    if ui.remove_layer.contains(position) {
        return Some(EditorIntent::RemoveLayer);
    }
    if ui.delete_material.contains(position) {
        return Some(EditorIntent::DeleteMaterial);
    }
    if ui.save.contains(position) {
        return Some(EditorIntent::Save);
    }
    if ui.undo.contains(position) {
        return Some(EditorIntent::Undo);
    }
    if ui.redo.contains(position) {
        return Some(EditorIntent::Redo);
    }
    if ui.help.contains(position) {
        return Some(EditorIntent::ToggleHelp);
    }
    if ui.visual.contains(position) {
        return Some(EditorIntent::SelectTool(EditorTool::Visual));
    }
    if ui.walkable.contains(position) {
        return Some(EditorIntent::SelectTool(EditorTool::Collision(
            Collision::Walkable,
        )));
    }
    if ui.blocked.contains(position) {
        return Some(EditorIntent::SelectTool(EditorTool::Collision(
            Collision::Blocked,
        )));
    }
    if ui.encounter.contains(position) {
        return Some(EditorIntent::SelectTool(EditorTool::Event(Some(
            MapEventKind::Encounter,
        ))));
    }
    if ui.clear_event.contains(position) {
        return Some(EditorIntent::SelectTool(EditorTool::Event(None)));
    }
    None
}

fn grid_position(x: f64, y: f64, viewport: Viewport) -> Option<GridPos> {
    let x = x - f64::from(viewport.origin.x);
    let y = y - f64::from(viewport.origin.y);
    if x < 0.0 || y < 0.0 {
        return None;
    }
    let col = (x / f64::from(viewport.cell_size.width)).floor() as i32;
    let row = (y / f64::from(viewport.cell_size.height)).floor() as i32;
    (col < layout::COLS as i32 && row < layout::ROWS as i32).then_some(GridPos::new(col, row))
}

fn map_position(
    position: GridPos,
    map_viewport: EditorMapViewport,
    model: &EditorModel,
) -> Option<TilePosition> {
    let col = position.col / map_viewport.tile_span as i32 + map_viewport.camera_col;
    let row = position.row / map_viewport.tile_span as i32 + map_viewport.camera_row;
    (layout::MAP_RECT.contains(position)
        && col < i32::from(model.project.width)
        && row < i32::from(model.project.height))
    .then(|| TilePosition::new(col as u16, row as u16))
}

#[cfg(test)]
mod tests {
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
        let (controller, intent) =
            controller.move_cursor(-1.0, -1.0, viewport, map_viewport(), &state);
        assert_eq!(intent, None);
        let (controller, _) = controller.move_cursor(80.0, 120.0, viewport, map_viewport(), &state);
        let (controller, intent) =
            controller.press(PointerButton::Secondary, map_viewport(), &state);
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
        let (controller, intent) =
            controller.press(PointerButton::Secondary, map_viewport(), &help);
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
}
