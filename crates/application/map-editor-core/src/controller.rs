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
#[path = "../tests/unit/controller.rs"]
mod tests;
