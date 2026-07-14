//! Pure map editor workbench projection.

#![forbid(unsafe_code)]

use std::{error::Error, fmt};

use game_assets::AssetKey;
use game_view::{GameView, LayerKind, TextLabel, TextRole, ViewCell, ViewImage, ViewLayer};
use map_editor_core::{EditorModel, EditorTool, layout};
use map_project::{Collision, MapEventKind, TilePosition};
use map_render::{AtomicTileCatalog, MapCamera, MapGridLayout, MapRenderInput, project_map};
use punctum_gpu::{PixelOffset, PixelSize, Rgba8, Viewport};
use punctum_grid::{GridPos, GridRect, GridSize, Surface, SurfaceError};

const UI_BG: Rgba8 = Rgba8::new(22, 25, 29, 255);
const PANEL: Rgba8 = Rgba8::new(31, 35, 41, 255);
const BUTTON: Rgba8 = Rgba8::new(47, 53, 61, 255);
const SELECTED: Rgba8 = Rgba8::new(43, 119, 108, 255);
const BORDER: Rgba8 = Rgba8::new(70, 78, 88, 255);
const TEXT: Rgba8 = Rgba8::new(238, 241, 236, 255);
const MUTED: Rgba8 = Rgba8::new(163, 173, 176, 255);

pub fn project(
    model: &EditorModel,
    catalog: &AtomicTileCatalog,
    hover: Option<TilePosition>,
    target_size: PixelSize,
) -> Result<(GameView, Viewport), EditorViewError> {
    let viewport = editor_viewport(target_size);
    let scene = project_map(MapRenderInput {
        project: &model.project,
        catalog,
        camera: MapCamera::default(),
        pixel_offset: PixelOffset::new(0, 0),
        viewport,
        layout: MapGridLayout::new(
            GridSize::new(layout::COLS, layout::ROWS),
            GridSize::new(layout::MAP_TILE_SPAN, layout::MAP_TILE_SPAN),
        ),
    })
    .map_err(|error| EditorViewError::Map(error.to_string()))?;
    let map = scene.into_layer();
    let mut surface = Surface::filled(GridSize::new(layout::COLS, layout::ROWS), ViewCell::Empty)
        .map_err(EditorViewError::Surface)?;
    project_chrome(&mut surface, model).map_err(EditorViewError::Surface)?;
    let mut images = Vec::new();
    project_assets(&mut images, model, catalog);
    project_materials(&mut images, model, catalog);
    project_semantics(&mut images, model);
    if let Some(position) = hover {
        images.push(image(
            GridPos::new(
                i32::from(position.x()) * layout::MAP_TILE_SPAN as i32,
                i32::from(position.y()) * layout::MAP_TILE_SPAN as i32,
            ),
            GridSize::new(layout::MAP_TILE_SPAN, layout::MAP_TILE_SPAN),
            white_asset(),
            Rgba8::new(255, 220, 78, 90),
            8,
        ));
    }
    let labels = project_labels(model, hover);
    let help = if model.show_help {
        let ui = layout::workbench();
        Some(
            ViewLayer::new(LayerKind::Console)
                .with_images(vec![image(
                    ui.help_panel.origin,
                    ui.help_panel.size,
                    white_asset(),
                    Rgba8::new(24, 28, 33, 248),
                    100,
                )])
                .with_labels(project_help_labels(ui.help_panel)),
        )
    } else {
        None
    };
    let mut layers = vec![
        map,
        ViewLayer::new(LayerKind::Character),
        ViewLayer::new(LayerKind::Hud)
            .with_surface(surface)
            .with_images(images)
            .with_labels(labels),
    ];
    if let Some(help) = help {
        layers.push(help);
    }
    Ok((GameView::new(layers), viewport))
}

pub fn editor_viewport(target_size: PixelSize) -> Viewport {
    let cell_size = (target_size.width / layout::COLS)
        .min(target_size.height / layout::ROWS)
        .max(1);
    let width = i64::from(layout::COLS) * i64::from(cell_size);
    let height = i64::from(layout::ROWS) * i64::from(cell_size);
    Viewport::new(
        target_size,
        PixelOffset::new(
            ((i64::from(target_size.width) - width) / 2) as i32,
            ((i64::from(target_size.height) - height) / 2) as i32,
        ),
        PixelSize::new(cell_size, cell_size),
    )
    .expect("editor viewport cell size is positive")
}

fn project_chrome(
    surface: &mut Surface<ViewCell>,
    model: &EditorModel,
) -> Result<(), SurfaceError> {
    let ui = layout::workbench();
    surface.fill(sprite(UI_BG));
    fill(surface, ui.right_panel, PANEL)?;
    fill(surface, ui.material_panel, PANEL)?;
    fill(surface, ui.divider, BORDER)?;
    for rect in [
        ui.previous_assets,
        ui.next_assets,
        ui.previous_materials,
        ui.next_materials,
        ui.add_layer,
        ui.remove_layer,
        ui.delete_material,
        ui.save,
        ui.undo,
        ui.redo,
        ui.help,
        ui.visual,
        ui.walkable,
        ui.blocked,
        ui.encounter,
        ui.clear_event,
    ] {
        fill(surface, rect, BUTTON)?;
    }
    if model.selected_atomic < layout::ASSET_PAGE_SIZE {
        fill(surface, ui.previous_assets, PANEL)?;
    }
    let last_asset_page = model.atomic_ids.len().saturating_sub(1) / layout::ASSET_PAGE_SIZE;
    if model.selected_atomic / layout::ASSET_PAGE_SIZE >= last_asset_page {
        fill(surface, ui.next_assets, PANEL)?;
    }
    if model.selected_material < layout::MATERIAL_PAGE_SIZE {
        fill(surface, ui.previous_materials, PANEL)?;
    }
    let last_material_page =
        model.project.materials.len().saturating_sub(1) / layout::MATERIAL_PAGE_SIZE;
    if model.selected_material / layout::MATERIAL_PAGE_SIZE >= last_material_page {
        fill(surface, ui.next_materials, PANEL)?;
    }
    let selected = match model.tool {
        EditorTool::Visual => ui.visual,
        EditorTool::Collision(Collision::Walkable) => ui.walkable,
        EditorTool::Collision(Collision::Blocked) => ui.blocked,
        EditorTool::Event(Some(MapEventKind::Encounter)) => ui.encounter,
        EditorTool::Event(None) => ui.clear_event,
    };
    fill(surface, selected, SELECTED)?;
    Ok(())
}

fn project_assets(images: &mut Vec<ViewImage>, model: &EditorModel, catalog: &AtomicTileCatalog) {
    let ui = layout::workbench();
    let page_start = (model.selected_atomic / layout::ASSET_PAGE_SIZE) * layout::ASSET_PAGE_SIZE;
    for (index, id) in model
        .atomic_ids
        .iter()
        .enumerate()
        .skip(page_start)
        .take(layout::ASSET_PAGE_SIZE)
    {
        let position = ui.asset_slots[index - page_start].origin;
        if let Some(asset) = catalog.asset(id) {
            images.push(image(
                position,
                GridSize::new(2, 2),
                asset.clone(),
                Rgba8::new(255, 255, 255, 255),
                3,
            ));
        }
        if index == model.selected_atomic {
            images.push(image(
                position,
                GridSize::new(2, 2),
                white_asset(),
                Rgba8::new(55, 205, 181, 72),
                4,
            ));
        }
    }
}

fn project_materials(
    images: &mut Vec<ViewImage>,
    model: &EditorModel,
    catalog: &AtomicTileCatalog,
) {
    let ui = layout::workbench();
    let page_start =
        (model.selected_material / layout::MATERIAL_PAGE_SIZE) * layout::MATERIAL_PAGE_SIZE;
    for (index, material) in model
        .project
        .materials
        .iter()
        .enumerate()
        .skip(page_start)
        .take(layout::MATERIAL_PAGE_SIZE)
    {
        let position = ui.material_slots[index - page_start].origin;
        for layer in &material.layers {
            if let Some(asset) = catalog.asset(layer) {
                images.push(image(
                    position,
                    GridSize::new(3, 3),
                    asset.clone(),
                    Rgba8::new(255, 255, 255, 255),
                    3,
                ));
            }
        }
        if index == model.selected_material {
            images.push(image(
                position,
                GridSize::new(3, 3),
                white_asset(),
                Rgba8::new(55, 205, 181, 72),
                4,
            ));
        }
    }
}

fn project_semantics(images: &mut Vec<ViewImage>, model: &EditorModel) {
    for row in 0..model.project.height {
        for col in 0..model.project.width {
            let index = usize::from(row) * usize::from(model.project.width) + usize::from(col);
            let tint = match model.tool {
                EditorTool::Collision(_) => match model.project.collision_cells[index] {
                    Collision::Walkable => Rgba8::new(53, 190, 105, 35),
                    Collision::Blocked => Rgba8::new(230, 72, 72, 118),
                },
                EditorTool::Event(_) => match model.project.event_cells[index] {
                    Some(MapEventKind::Encounter) => Rgba8::new(244, 187, 58, 118),
                    None => continue,
                },
                EditorTool::Visual => continue,
            };
            images.push(image(
                GridPos::new(
                    i32::from(col) * layout::MAP_TILE_SPAN as i32,
                    i32::from(row) * layout::MAP_TILE_SPAN as i32,
                ),
                GridSize::new(layout::MAP_TILE_SPAN, layout::MAP_TILE_SPAN),
                white_asset(),
                tint,
                7,
            ));
        }
    }
}

fn project_labels(model: &EditorModel, hover: Option<TilePosition>) -> Vec<TextLabel> {
    let ui = layout::workbench();
    let asset_page = model.selected_atomic / layout::ASSET_PAGE_SIZE;
    let asset_pages = model.atomic_ids.len().div_ceil(layout::ASSET_PAGE_SIZE);
    let selected_atomic = model
        .atomic_ids
        .get(model.selected_atomic)
        .map_or("none", |id| id.as_str());
    let selected_material = model.project.materials.get(model.selected_material);
    let material_page = model.selected_material / layout::MATERIAL_PAGE_SIZE;
    let material_pages = model
        .project
        .materials
        .len()
        .div_ceil(layout::MATERIAL_PAGE_SIZE)
        .max(1);
    let mut labels = vec![
        label_rect(ui.asset_title, "原子素材", TEXT),
        label_rect(
            ui.asset_summary,
            &format!("{}  {}/{}", selected_atomic, asset_page + 1, asset_pages),
            MUTED,
        ),
        label_rect(ui.previous_assets, "上一页", TEXT),
        label_rect(ui.next_assets, "下一页", TEXT),
        label_rect(
            ui.material_title,
            &format!("组合素材  {}/{}", material_page + 1, material_pages),
            TEXT,
        ),
        label_rect(ui.previous_materials, "上一页", TEXT),
        label_rect(ui.next_materials, "下一页", TEXT),
        label_rect(ui.visual, "贴图", TEXT),
        label_rect(ui.walkable, "可通行", TEXT),
        label_rect(ui.blocked, "阻挡", TEXT),
        label_rect(ui.encounter, "遭遇事件", TEXT),
        label_rect(ui.clear_event, "清除事件", TEXT),
        label_rect(ui.composition_title, "当前组合", TEXT),
        label_rect(ui.add_layer, "添加一层", TEXT),
        label_rect(ui.remove_layer, "移除顶层", TEXT),
        label_rect(ui.delete_material, "删除当前组合", TEXT),
        label_rect(ui.tool_title, "工具", TEXT),
        label_rect(ui.save, "保存", TEXT),
        label_rect(ui.undo, "撤销", TEXT),
        label_rect(ui.redo, "重做", TEXT),
        label_rect(ui.help, "帮助", TEXT),
    ];
    let asset_start = asset_page * layout::ASSET_PAGE_SIZE;
    for (index, id) in model
        .atomic_ids
        .iter()
        .enumerate()
        .skip(asset_start)
        .take(layout::ASSET_PAGE_SIZE)
    {
        let slot = ui.asset_slots[index - asset_start];
        labels.push(label(
            slot.origin.col as u32,
            (slot.origin.row + 2) as u32,
            slot.size.cols,
            id.as_str().strip_prefix("tile-").unwrap_or(id.as_str()),
            MUTED,
        ));
    }
    let material_start =
        (model.selected_material / layout::MATERIAL_PAGE_SIZE) * layout::MATERIAL_PAGE_SIZE;
    for (index, material) in model
        .project
        .materials
        .iter()
        .enumerate()
        .skip(material_start)
        .take(layout::MATERIAL_PAGE_SIZE)
    {
        let slot = ui.material_slots[index - material_start];
        labels.push(label(
            slot.origin.col as u32,
            (slot.origin.row + 3) as u32,
            slot.size.cols,
            material
                .id
                .as_str()
                .strip_prefix("material-")
                .unwrap_or(material.id.as_str()),
            MUTED,
        ));
    }
    if let Some(material) = selected_material {
        labels.push(label_rect(
            ui.composition_summary,
            material.id.as_str(),
            MUTED,
        ));
        labels.push(label_rect(
            ui.layer_summary,
            &format!("{} 层", material.layers.len()),
            MUTED,
        ));
    }
    let coordinate = hover.map_or_else(String::new, |position| {
        format!(" | {}, {}", position.x(), position.y())
    });
    labels.push(label_rect(
        ui.status,
        &format!("{}{}", model.status, coordinate),
        if model.status.starts_with("错误") {
            Rgba8::new(255, 133, 116, 255)
        } else {
            MUTED
        },
    ));
    labels
}

fn project_help_labels(panel: GridRect) -> Vec<TextLabel> {
    let col = panel.origin.col as u32 + 2;
    let width = panel.size.cols.saturating_sub(4);
    let row = panel.origin.row as u32;
    vec![
        label(col, row + 1, width, "地图编辑器使用说明", TEXT),
        label(
            col,
            row + 3,
            width,
            "1. 在右侧“原子素材”中点击一张 16x16 素材。",
            TEXT,
        ),
        label(
            col,
            row + 5,
            width,
            "2. 在底部“组合素材”中选择画笔，左键点击地图绘制。",
            TEXT,
        ),
        label(
            col,
            row + 7,
            width,
            "3. 右键点击地图可擦除当前图层内容。",
            TEXT,
        ),
        label(
            col,
            row + 9,
            width,
            "4. 点击“添加一层”，会用当前原子素材创建新组合。",
            TEXT,
        ),
        label(
            col,
            row + 11,
            width,
            "5. “删除当前组合”只能删除未被地图使用的素材。",
            TEXT,
        ),
        label(
            col,
            row + 13,
            width,
            "6. 切换“可通行 / 阻挡 / 遭遇事件”后再点击地图。",
            TEXT,
        ),
        label(
            col,
            row + 15,
            width,
            "7. “贴图”只改画面；碰撞和事件不会修改贴图。",
            TEXT,
        ),
        label(
            col,
            row + 17,
            width,
            "8. 保存后，游戏会读取同一份地图文件。",
            TEXT,
        ),
        label(
            col,
            row + 19,
            width,
            "快捷键：Delete 删除组合，Ctrl+S 保存，Ctrl+Z 撤销。",
            MUTED,
        ),
        label(
            col,
            row + 21,
            width,
            "再次点击右下角“帮助”关闭本说明。",
            MUTED,
        ),
    ]
}

fn label_rect(rect: GridRect, content: &str, color: Rgba8) -> TextLabel {
    label(
        rect.origin.col as u32,
        rect.origin.row as u32,
        rect.size.cols,
        content,
        color,
    )
}

fn label(col: u32, row: u32, width: u32, content: &str, color: Rgba8) -> TextLabel {
    TextLabel {
        role: TextRole::Editor,
        col,
        row,
        width,
        height: 1,
        content: content.into(),
        color,
    }
}

fn image(
    position: GridPos,
    size: GridSize,
    asset: AssetKey,
    tint: Rgba8,
    z_index: i32,
) -> ViewImage {
    ViewImage::new(GridRect::new(position, size), asset, tint, z_index as u16)
}

const fn sprite(color: Rgba8) -> ViewCell {
    ViewCell::Fill(color)
}

fn fill(surface: &mut Surface<ViewCell>, rect: GridRect, color: Rgba8) -> Result<(), SurfaceError> {
    surface.fill_rect(rect, sprite(color))
}

fn white_asset() -> AssetKey {
    AssetKey::new("solid/white").expect("the white asset key is valid")
}

#[derive(Debug)]
pub enum EditorViewError {
    Map(String),
    Surface(SurfaceError),
}

impl fmt::Display for EditorViewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Map(error) => write!(formatter, "map projection failed: {error}"),
            Self::Surface(error) => write!(formatter, "workbench projection failed: {error}"),
        }
    }
}

impl Error for EditorViewError {}

#[cfg(test)]
mod tests {
    use game_assets::AssetKey;
    use map_project::{AtomicTileId, CompositeTile, CompositeTileId, MapProject, MapProjectId};
    use map_render::AtomicTileAsset;

    use super::*;

    #[test]
    fn projects_a_fixed_workbench_with_readable_labels() {
        let tile = AtomicTileId::new("tile-0001").unwrap();
        let model = EditorModel::new(
            MapProject::blank(
                MapProjectId::new("map").unwrap(),
                16,
                10,
                Some(CompositeTile::new(
                    CompositeTileId::new("material-0000").unwrap(),
                    vec![tile.clone()],
                )),
            ),
            vec![tile.clone()],
        );
        let catalog = AtomicTileCatalog::new([AtomicTileAsset {
            id: tile,
            asset: AssetKey::new("map/tile/tile-0001").unwrap(),
        }])
        .unwrap();
        let (view, _) = project(&model, &catalog, None, PixelSize::new(1280, 720)).unwrap();
        assert_eq!(
            view.layers()[2].surface.as_ref().unwrap().size(),
            GridSize::new(64, 38)
        );
        assert!(view.labels().any(|label| label.content == "原子素材"));
        assert!(
            view.labels()
                .any(|label| label.content.starts_with("组合素材"))
        );
        assert!(view.labels().any(|label| label.content == "删除当前组合"));
    }

    #[test]
    fn help_is_an_explicit_layer_and_does_not_steal_hud_labels() {
        let tile = AtomicTileId::new("tile-0001").unwrap();
        let mut model = EditorModel::new(
            MapProject::blank(
                MapProjectId::new("map").unwrap(),
                16,
                10,
                Some(CompositeTile::new(
                    CompositeTileId::new("material-0000").unwrap(),
                    vec![tile.clone()],
                )),
            ),
            vec![tile.clone()],
        );
        model.show_help = true;
        let catalog = AtomicTileCatalog::new([AtomicTileAsset {
            id: tile,
            asset: AssetKey::new("map/tile/tile-0001").unwrap(),
        }])
        .unwrap();

        let (view, _) = project(
            &model,
            &catalog,
            Some(TilePosition::new(1, 2)),
            PixelSize::new(1280, 720),
        )
        .unwrap();

        assert_eq!(view.layers().len(), 4);
        assert_eq!(view.layers()[3].kind, LayerKind::Console);
        assert_eq!(view.layers()[3].labels.len(), 11);
        assert!(
            view.layers()[2]
                .labels
                .iter()
                .any(|label| label.content == "就绪 | 1, 2")
        );
    }

    #[test]
    fn projects_every_tool_page_and_error_state() {
        let tile = AtomicTileId::new("tile-0001").unwrap();
        let mut model = EditorModel::new(
            MapProject::blank(
                MapProjectId::new("map").unwrap(),
                16,
                10,
                Some(CompositeTile::new(
                    CompositeTileId::new("material-0000").unwrap(),
                    vec![tile.clone()],
                )),
            ),
            (0..25)
                .map(|index| AtomicTileId::new(format!("tile-{index:04}")).unwrap())
                .collect(),
        );
        let catalog = AtomicTileCatalog::new([AtomicTileAsset {
            id: tile.clone(),
            asset: AssetKey::new("map/tile/tile-0001").unwrap(),
        }])
        .unwrap();
        model.project.collision_cells[1] = Collision::Blocked;
        model.project.event_cells[0] = Some(MapEventKind::Encounter);
        model.selected_atomic = 24;
        for index in 1..9 {
            model.project.materials.push(CompositeTile::new(
                CompositeTileId::new(format!("material-{index:04}")).unwrap(),
                vec![tile.clone()],
            ));
        }
        model.selected_material = 8;
        model.status = "错误：fixture".into();

        for tool in [
            EditorTool::Collision(Collision::Walkable),
            EditorTool::Collision(Collision::Blocked),
            EditorTool::Event(Some(MapEventKind::Encounter)),
            EditorTool::Event(None),
        ] {
            model.tool = tool;
            let (view, _) = project(&model, &catalog, None, PixelSize::new(1, 1)).unwrap();
            assert_eq!(view.layers().len(), 3);
            assert!(
                view.layers()[2]
                    .images
                    .iter()
                    .any(|image| image.z_index == 7)
            );
            assert!(view.labels().any(|label| label.content == "错误：fixture"));
        }

        let missing = AtomicTileCatalog::new([]).unwrap();
        let error = project(&model, &missing, None, PixelSize::new(1280, 720)).unwrap_err();
        assert!(error.to_string().starts_with("map projection failed:"));
        let surface_error =
            Surface::<ViewCell>::from_cells(GridSize::new(1, 1), vec![]).unwrap_err();
        assert!(
            EditorViewError::Surface(surface_error)
                .to_string()
                .starts_with("workbench projection failed:")
        );
    }
}
