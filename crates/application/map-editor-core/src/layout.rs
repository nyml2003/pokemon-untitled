use punctum_grid::{GridPos, GridRect, GridSize};

pub const COLS: u32 = 64;
pub const ROWS: u32 = 38;
pub const MAP_TILE_SPAN: u32 = 2;
pub const MAP_RECT: GridRect = rect(0, 0, 48, 32);
pub const ASSET_COLS: usize = 5;
pub const ASSET_ROWS: usize = 3;
pub const ASSET_PAGE_SIZE: usize = ASSET_COLS * ASSET_ROWS;
pub const MATERIAL_PAGE_SIZE: usize = 5;

#[derive(Clone, Copy, Debug)]
struct Row {
    bounds: GridRect,
    cursor: i32,
    gap: u32,
}

impl Row {
    const fn new(bounds: GridRect, gap: u32) -> Self {
        Self {
            bounds,
            cursor: bounds.origin.col,
            gap,
        }
    }

    fn take(&mut self, width: u32) -> GridRect {
        let item = rect(
            self.cursor,
            self.bounds.origin.row,
            width,
            self.bounds.size.rows,
        );
        assert!(
            item_right(item) <= item_right(self.bounds),
            "row item exceeds bounds"
        );
        self.cursor += width as i32 + self.gap as i32;
        item
    }
}

#[derive(Clone, Copy, Debug)]
struct Column {
    bounds: GridRect,
    cursor: i32,
    gap: u32,
}

impl Column {
    const fn new(bounds: GridRect, gap: u32) -> Self {
        Self {
            bounds,
            cursor: bounds.origin.row,
            gap,
        }
    }

    fn take(&mut self, height: u32) -> GridRect {
        let item = rect(
            self.bounds.origin.col,
            self.cursor,
            self.bounds.size.cols,
            height,
        );
        assert!(
            item_bottom(item) <= item_bottom(self.bounds),
            "column item exceeds bounds"
        );
        self.cursor += height as i32 + self.gap as i32;
        item
    }

    fn skip(&mut self, height: u32) {
        self.cursor += height as i32;
        assert!(
            self.cursor <= item_bottom(self.bounds),
            "column spacer exceeds bounds"
        );
    }
}

#[derive(Clone, Debug)]
pub struct WorkbenchLayout {
    pub right_panel: GridRect,
    pub material_panel: GridRect,
    pub divider: GridRect,
    pub asset_title: GridRect,
    pub asset_summary: GridRect,
    pub asset_slots: [GridRect; ASSET_PAGE_SIZE],
    pub previous_assets: GridRect,
    pub next_assets: GridRect,
    pub composition_title: GridRect,
    pub composition_summary: GridRect,
    pub add_layer: GridRect,
    pub remove_layer: GridRect,
    pub layer_summary: GridRect,
    pub delete_material: GridRect,
    pub tool_title: GridRect,
    pub visual: GridRect,
    pub walkable: GridRect,
    pub blocked: GridRect,
    pub encounter: GridRect,
    pub clear_event: GridRect,
    pub status: GridRect,
    pub save: GridRect,
    pub undo: GridRect,
    pub redo: GridRect,
    pub help: GridRect,
    pub help_panel: GridRect,
    pub material_title: GridRect,
    pub previous_materials: GridRect,
    pub next_materials: GridRect,
    pub material_slots: [GridRect; MATERIAL_PAGE_SIZE],
}

pub fn workbench() -> WorkbenchLayout {
    let right_panel = rect(48, 0, 16, 38);
    let material_panel = rect(0, 32, 48, 6);
    let divider = rect(48, 0, 1, 38);

    let mut right = Column::new(rect(49, 0, 14, 38), 0);
    let asset_title = right.take(1);
    let asset_summary = right.take(1);
    let asset_grid = right.take(9);
    let mut asset_slots = [GridRect::default(); ASSET_PAGE_SIZE];
    let mut asset_rows = Column::new(asset_grid, 0);
    for row_index in 0..ASSET_ROWS {
        let mut row = Row::new(asset_rows.take(3), 1);
        for col_index in 0..ASSET_COLS {
            asset_slots[row_index * ASSET_COLS + col_index] = row.take(2);
        }
    }

    let mut asset_pager = Row::new(right.take(1), 2);
    let previous_assets = asset_pager.take(6);
    let next_assets = asset_pager.take(6);
    right.skip(1);
    let composition_title = right.take(1);
    let composition_summary = right.take(1);
    let mut layer_actions = Row::new(right.take(1), 2);
    let add_layer = layer_actions.take(6);
    let remove_layer = layer_actions.take(6);
    let layer_summary = right.take(1);
    let delete_material = right.take(1);
    right.skip(9);
    let tool_title = right.take(1);
    let visual = right.take(1);
    right.skip(1);
    let mut collision_modes = Row::new(right.take(1), 2);
    let walkable = collision_modes.take(6);
    let blocked = collision_modes.take(6);
    right.skip(1);
    let mut event_modes = Row::new(right.take(1), 2);
    let encounter = event_modes.take(6);
    let clear_event = event_modes.take(6);
    right.skip(1);
    let status = right.take(1);
    right.skip(2);
    let mut history_actions = Row::new(right.take(1), 0);
    let save = history_actions.take(3);
    let undo = history_actions.take(3);
    let redo = history_actions.take(3);
    let help = history_actions.take(3);
    let help_panel = rect(5, 4, 38, 23);

    let mut materials = Column::new(rect(1, 32, 46, 6), 0);
    let material_title = materials.take(1);
    let mut material_pager = Row::new(materials.take(1), 34);
    let previous_materials = material_pager.take(6);
    let next_materials = material_pager.take(6);
    let material_grid = materials.take(4);
    let mut material_row = Row::new(material_grid, 6);
    let mut material_slots = [GridRect::default(); MATERIAL_PAGE_SIZE];
    for slot in &mut material_slots {
        *slot = material_row.take(3);
    }

    WorkbenchLayout {
        right_panel,
        material_panel,
        divider,
        asset_title,
        asset_summary,
        asset_slots,
        previous_assets,
        next_assets,
        composition_title,
        composition_summary,
        add_layer,
        remove_layer,
        layer_summary,
        delete_material,
        tool_title,
        visual,
        walkable,
        blocked,
        encounter,
        clear_event,
        status,
        save,
        undo,
        redo,
        help,
        help_panel,
        material_title,
        previous_materials,
        next_materials,
        material_slots,
    }
}

pub const fn rect(col: i32, row: i32, cols: u32, rows: u32) -> GridRect {
    GridRect::new(GridPos::new(col, row), GridSize::new(cols, rows))
}

const fn item_right(rect: GridRect) -> i32 {
    rect.origin.col + rect.size.cols as i32
}

const fn item_bottom(rect: GridRect) -> i32 {
    rect.origin.row + rect.size.rows as i32
}

#[cfg(test)]
#[path = "../tests/unit/layout.rs"]
mod tests;
