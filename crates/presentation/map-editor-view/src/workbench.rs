//! 纯地图编辑器工作台投影。

use std::{error::Error, fmt};

use game_assets::AssetKey;
use game_view::{GameView, LayerKind, TextLabel, TextRole, ViewCell, ViewImage, ViewLayer};
use map_editor_core::{EditorIntent, EditorMapViewport, EditorModel, EditorTool, layout};
use map_project::{Collision, MapEventKind, TilePosition};
use map_render::{AtomicTileCatalog, MapCamera, MapGridLayout, MapRenderInput, project_map};
use punctum_gpu::{PixelOffset, PixelSize, Rgba8, Viewport};
use punctum_grid::{GridPos, GridRect, GridSize, Surface, SurfaceError};
use punctum_ui::{
    CrossAlign, Dimension, FlexDirection, Insets, MainAlign, UiBuildError, UiColor, UiContent,
    UiContentId, UiId, UiNode as RawUiNode, UiStyle, UiTree,
};

const UI_BG: Rgba8 = Rgba8::new(22, 25, 29, 255);
const PANEL: Rgba8 = Rgba8::new(31, 35, 41, 255);
const BUTTON: Rgba8 = Rgba8::new(47, 53, 61, 255);
const SELECTED: Rgba8 = Rgba8::new(43, 119, 108, 255);
const BORDER: Rgba8 = Rgba8::new(70, 78, 88, 255);
const TEXT: Rgba8 = Rgba8::new(238, 241, 236, 255);
const MUTED: Rgba8 = Rgba8::new(163, 173, 176, 255);

const ASSET_SLOT_ID: u32 = 1_000;
const MATERIAL_SLOT_ID: u32 = 1_100;
const PREVIOUS_ASSETS_ID: u32 = 1_200;
const NEXT_ASSETS_ID: u32 = 1_201;
const PREVIOUS_MATERIALS_ID: u32 = 1_202;
const NEXT_MATERIALS_ID: u32 = 1_203;
const ADD_LAYER_ID: u32 = 1_210;
const REMOVE_LAYER_ID: u32 = 1_211;
const DELETE_MATERIAL_ID: u32 = 1_212;
const VISUAL_ID: u32 = 1_220;
const WALKABLE_ID: u32 = 1_221;
const BLOCKED_ID: u32 = 1_222;
const ENCOUNTER_ID: u32 = 1_223;
const CLEAR_EVENT_ID: u32 = 1_224;
const SAVE_ID: u32 = 1_230;
const UNDO_ID: u32 = 1_231;
const REDO_ID: u32 = 1_232;
const HELP_ID: u32 = 1_233;
const HELP_CLOSE_ID: u32 = 1_234;

#[derive(Debug)]
/// 编辑器模型的一帧投影结果。
/// `map` 包含地图和语义覆盖层，`chrome` 包含编辑工具的 Flex UI。
pub struct EditorFrame {
    pub map: GameView,
    pub chrome: UiTree<EditorChromeAction>,
    pub viewport: Viewport,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorChromeAction {
    Command(u32),
}

type UiNode = RawUiNode<EditorChromeAction>;

/// 将编辑器模型、资源目录和当前悬停位置投影为一帧工作台视图。
/// 地图渲染输入不合法时返回 `EditorViewError::Map`，UI 树无效时返回 `EditorViewError::Ui`。
pub fn project(
    model: &EditorModel,
    catalog: &AtomicTileCatalog,
    hover: Option<TilePosition>,
    target_size: PixelSize,
    map_viewport: EditorMapViewport,
) -> Result<EditorFrame, EditorViewError> {
    let viewport = editor_viewport(target_size);
    let scene = project_map(MapRenderInput {
        project: &model.project,
        catalog,
        camera: MapCamera::new(map_viewport.camera_col, map_viewport.camera_row),
        pixel_offset: PixelOffset::new(0, 0),
        viewport,
        layout: MapGridLayout::new(
            GridSize::new(layout::COLS, layout::ROWS),
            GridSize::new(map_viewport.tile_span, map_viewport.tile_span),
        ),
    })
    .map_err(|error| EditorViewError::Map(error.to_string()))?;
    let map = scene.into_layer();
    let mut images = Vec::new();
    project_semantics(&mut images, model, map_viewport);
    if let Some(position) = hover {
        images.push(image(
            GridPos::new(
                (i32::from(position.x()) - map_viewport.camera_col) * map_viewport.tile_span as i32,
                (i32::from(position.y()) - map_viewport.camera_row) * map_viewport.tile_span as i32,
            ),
            GridSize::new(map_viewport.tile_span, map_viewport.tile_span),
            white_asset(),
            Rgba8::new(255, 220, 78, 90),
            8,
        ));
    }
    let layers = vec![
        map,
        ViewLayer::new(LayerKind::Character),
        ViewLayer::new(LayerKind::Hud).with_images(images),
    ];
    let chrome = project_chrome_ui(model, catalog, hover).map_err(EditorViewError::Ui)?;
    Ok(EditorFrame {
        map: GameView::new(layers),
        chrome,
        viewport,
    })
}

/// 返回以地图中心为目标的编辑器相机视口。
///
/// # Panics
///
/// 当 `tile_span` 为零时 panic。
pub fn centered_map_viewport(model: &EditorModel, tile_span: u32) -> EditorMapViewport {
    assert!(tile_span > 0, "map tile span must be positive");
    let visible_cols = layout::MAP_RECT.size.cols / tile_span;
    let visible_rows = layout::MAP_RECT.size.rows / tile_span;
    let camera_col = i32::from(model.project.width.saturating_sub(visible_cols as u16) / 2);
    let camera_row = i32::from(model.project.height.saturating_sub(visible_rows as u16) / 2);
    EditorMapViewport::new(tile_span, camera_col, camera_row)
}

/// 在目标尺寸内居中放置整数缩放的固定编辑器网格。
/// 缩放比例至少为一个像素，即使目标尺寸小于网格也保持该比例。
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

/// 将 Flex UI 命中映射回现有编辑器 reducer 的意图。
/// 地图绘制刻意留在 `EditorController`，不会经过这里。
pub fn intent_for_ui_action(
    model: &EditorModel,
    action: EditorChromeAction,
) -> Option<EditorIntent> {
    let EditorChromeAction::Command(id) = action;
    match id {
        value
            if (ASSET_SLOT_ID..ASSET_SLOT_ID + layout::ASSET_PAGE_SIZE as u32).contains(&value) =>
        {
            let page = model.selected_atomic / layout::ASSET_PAGE_SIZE;
            let index = page * layout::ASSET_PAGE_SIZE + (value - ASSET_SLOT_ID) as usize;
            (index < model.atomic_ids.len()).then_some(EditorIntent::SelectAtomic(index))
        }
        value
            if (MATERIAL_SLOT_ID..MATERIAL_SLOT_ID + layout::MATERIAL_PAGE_SIZE as u32)
                .contains(&value) =>
        {
            let page = model.selected_material / layout::MATERIAL_PAGE_SIZE;
            let index = page * layout::MATERIAL_PAGE_SIZE + (value - MATERIAL_SLOT_ID) as usize;
            (index < model.project.materials.len()).then_some(EditorIntent::SelectMaterial(index))
        }
        PREVIOUS_ASSETS_ID => Some(EditorIntent::SelectAtomic(
            model
                .selected_atomic
                .saturating_sub(layout::ASSET_PAGE_SIZE),
        )),
        NEXT_ASSETS_ID => Some(EditorIntent::SelectAtomic(
            (model.selected_atomic / layout::ASSET_PAGE_SIZE + 1) * layout::ASSET_PAGE_SIZE,
        )),
        PREVIOUS_MATERIALS_ID => Some(EditorIntent::SelectMaterial(
            model
                .selected_material
                .saturating_sub(layout::MATERIAL_PAGE_SIZE),
        )),
        NEXT_MATERIALS_ID => Some(EditorIntent::SelectMaterial(
            (model.selected_material / layout::MATERIAL_PAGE_SIZE + 1) * layout::MATERIAL_PAGE_SIZE,
        )),
        ADD_LAYER_ID => Some(EditorIntent::AddLayer),
        REMOVE_LAYER_ID => Some(EditorIntent::RemoveLayer),
        DELETE_MATERIAL_ID => Some(EditorIntent::DeleteMaterial),
        VISUAL_ID => Some(EditorIntent::SelectTool(EditorTool::Visual)),
        WALKABLE_ID => Some(EditorIntent::SelectTool(EditorTool::Collision(
            Collision::Walkable,
        ))),
        BLOCKED_ID => Some(EditorIntent::SelectTool(EditorTool::Collision(
            Collision::Blocked,
        ))),
        ENCOUNTER_ID => Some(EditorIntent::SelectTool(EditorTool::Event(Some(
            MapEventKind::Encounter,
        )))),
        CLEAR_EVENT_ID => Some(EditorIntent::SelectTool(EditorTool::Event(None))),
        SAVE_ID => Some(EditorIntent::Save),
        UNDO_ID => Some(EditorIntent::Undo),
        REDO_ID => Some(EditorIntent::Redo),
        HELP_ID | HELP_CLOSE_ID => Some(EditorIntent::ToggleHelp),
        _ => None,
    }
}

#[derive(Default)]
struct UiIds {
    next: u32,
}

impl UiIds {
    fn next(&mut self) -> UiId {
        let id = UiId(self.next);
        self.next = self
            .next
            .checked_add(1)
            .expect("editor UI node id overflow");
        id
    }
}

fn project_chrome_ui(
    model: &EditorModel,
    catalog: &AtomicTileCatalog,
    hover: Option<TilePosition>,
) -> Result<UiTree<EditorChromeAction>, UiBuildError> {
    let mut ids = UiIds::default();
    let sidebar = editor_sidebar(&mut ids, model, catalog, hover);
    let materials = editor_materials(&mut ids, model, catalog);
    let top = UiNode::auto()
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Ratio {
                units: 16,
                base: 19,
            },
            direction: FlexDirection::Row,
            ..UiStyle::default()
        })
        .with_children([
            UiNode::auto().with_style(UiStyle {
                width: Dimension::Ratio { units: 3, base: 4 },
                height: Dimension::Fill,
                ..UiStyle::default()
            }),
            sidebar,
        ]);
    let bottom = UiNode::auto()
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Ratio { units: 3, base: 19 },
            direction: FlexDirection::Row,
            ..UiStyle::default()
        })
        .with_children([
            materials,
            UiNode::auto().with_style(UiStyle {
                width: Dimension::Ratio { units: 1, base: 4 },
                height: Dimension::Fill,
                ..UiStyle::default()
            }),
        ]);
    let mut children = vec![
        UiNode::auto()
            .with_style(UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                direction: FlexDirection::Column,
                ..UiStyle::default()
            })
            .with_children([top, bottom]),
    ];
    if model.show_help {
        children.push(editor_help_dialog(&mut ids));
    }
    UiTree::new(
        UiNode::auto()
            .with_style(UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                direction: FlexDirection::Stack,
                ..UiStyle::default()
            })
            .with_children(children),
    )
}

fn editor_sidebar(
    ids: &mut UiIds,
    model: &EditorModel,
    catalog: &AtomicTileCatalog,
    hover: Option<TilePosition>,
) -> UiNode {
    let asset_page = model.selected_atomic / layout::ASSET_PAGE_SIZE;
    let asset_pages = model
        .atomic_ids
        .len()
        .div_ceil(layout::ASSET_PAGE_SIZE)
        .max(1);
    let start = asset_page * layout::ASSET_PAGE_SIZE;
    let mut asset_rows = Vec::new();
    for row in 0..layout::ASSET_ROWS {
        let mut cards = Vec::new();
        for column in 0..layout::ASSET_COLS {
            let local = row * layout::ASSET_COLS + column;
            let index = start + local;
            cards.push(editor_asset_card(
                ids,
                ASSET_SLOT_ID + local as u32,
                model.atomic_ids.get(index),
                catalog,
                index == model.selected_atomic,
            ));
        }
        asset_rows.push(
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    direction: FlexDirection::Row,
                    gap: 6,
                    ..UiStyle::default()
                })
                .with_children(cards),
        );
    }
    let selected_atomic = model
        .atomic_ids
        .get(model.selected_atomic)
        .map_or("none", |id| id.as_str());
    let tool_buttons = [
        (VISUAL_ID, "贴图", matches!(model.tool, EditorTool::Visual)),
        (
            WALKABLE_ID,
            "可通行",
            matches!(model.tool, EditorTool::Collision(Collision::Walkable)),
        ),
        (
            BLOCKED_ID,
            "阻挡",
            matches!(model.tool, EditorTool::Collision(Collision::Blocked)),
        ),
        (
            ENCOUNTER_ID,
            "遭遇事件",
            matches!(model.tool, EditorTool::Event(Some(MapEventKind::Encounter))),
        ),
        (
            CLEAR_EVENT_ID,
            "清除事件",
            matches!(model.tool, EditorTool::Event(None)),
        ),
    ]
    .into_iter()
    .map(|(id, label, selected)| editor_button(ids, id, label, selected))
    .collect::<Vec<_>>();
    let coordinate = hover.map_or_else(String::new, |position| {
        format!(" | {}, {}", position.x(), position.y())
    });
    panel(
        ids.next(),
        UiStyle {
            width: Dimension::Ratio { units: 1, base: 4 },
            height: Dimension::Fill,
            direction: FlexDirection::Column,
            gap: 10,
            padding: Insets::all(14),
            border_radius: punctum_ui::UiBorderRadius::all(12),
            ..UiStyle::default()
        },
        PANEL,
        [
            ui_text(ids, "原子素材", TEXT, 20, Dimension::Fill),
            ui_text(
                ids,
                format!("{}  {}/{}", selected_atomic, asset_page + 1, asset_pages),
                MUTED,
                14,
                Dimension::Fill,
            ),
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(220),
                    direction: FlexDirection::Column,
                    gap: 6,
                    ..UiStyle::default()
                })
                .with_children(asset_rows),
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(32),
                    direction: FlexDirection::Row,
                    gap: 8,
                    ..UiStyle::default()
                })
                .with_children([
                    editor_button(ids, PREVIOUS_ASSETS_ID, "上一页", false),
                    editor_button(ids, NEXT_ASSETS_ID, "下一页", false),
                ]),
            ui_text(ids, "工具", TEXT, 18, Dimension::Fill),
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    direction: FlexDirection::Column,
                    gap: 6,
                    clip: true,
                    ..UiStyle::default()
                })
                .with_children(tool_buttons),
            ui_text(
                ids,
                format!("{}{}", model.status, coordinate),
                if model.status.starts_with("错误") {
                    Rgba8::new(255, 133, 116, 255)
                } else {
                    MUTED
                },
                14,
                Dimension::Fill,
            ),
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(32),
                    direction: FlexDirection::Row,
                    gap: 6,
                    ..UiStyle::default()
                })
                .with_children([
                    editor_button(ids, SAVE_ID, "保存", false),
                    editor_button(ids, UNDO_ID, "撤销", false),
                    editor_button(ids, REDO_ID, "重做", false),
                    editor_button(ids, HELP_ID, "帮助", false),
                ]),
        ],
    )
}

fn editor_materials(ids: &mut UiIds, model: &EditorModel, catalog: &AtomicTileCatalog) -> UiNode {
    let page = model.selected_material / layout::MATERIAL_PAGE_SIZE;
    let pages = model
        .project
        .materials
        .len()
        .div_ceil(layout::MATERIAL_PAGE_SIZE)
        .max(1);
    let start = page * layout::MATERIAL_PAGE_SIZE;
    let cards = (0..layout::MATERIAL_PAGE_SIZE)
        .map(|local| {
            let index = start + local;
            editor_material_card(
                ids,
                MATERIAL_SLOT_ID + local as u32,
                model.project.materials.get(index),
                catalog,
                index == model.selected_material,
            )
        })
        .collect::<Vec<_>>();
    let selected = model.project.materials.get(model.selected_material);
    panel(
        ids.next(),
        UiStyle {
            width: Dimension::Ratio { units: 3, base: 4 },
            height: Dimension::Fill,
            direction: FlexDirection::Row,
            gap: 14,
            padding: Insets::all(12),
            border_radius: punctum_ui::UiBorderRadius {
                top_left: 0,
                top_right: 12,
                bottom_right: 0,
                bottom_left: 0,
            },
            ..UiStyle::default()
        },
        UI_BG,
        [
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Ratio { units: 3, base: 5 },
                    height: Dimension::Fill,
                    direction: FlexDirection::Column,
                    gap: 8,
                    ..UiStyle::default()
                })
                .with_children([
                    ui_text(
                        ids,
                        format!("组合素材  {}/{}", page + 1, pages),
                        TEXT,
                        18,
                        Dimension::Fill,
                    ),
                    UiNode::auto()
                        .with_style(UiStyle {
                            width: Dimension::Fill,
                            height: Dimension::Fill,
                            direction: FlexDirection::Row,
                            gap: 8,
                            clip: true,
                            ..UiStyle::default()
                        })
                        .with_children(cards),
                    UiNode::auto()
                        .with_style(UiStyle {
                            width: Dimension::Fill,
                            height: Dimension::Px(30),
                            direction: FlexDirection::Row,
                            gap: 8,
                            ..UiStyle::default()
                        })
                        .with_children([
                            editor_button(ids, PREVIOUS_MATERIALS_ID, "上一页", false),
                            editor_button(ids, NEXT_MATERIALS_ID, "下一页", false),
                        ]),
                ]),
            panel(
                ids.next(),
                UiStyle {
                    width: Dimension::Ratio { units: 2, base: 5 },
                    height: Dimension::Fill,
                    direction: FlexDirection::Column,
                    gap: 6,
                    padding: Insets::all(10),
                    border_radius: punctum_ui::UiBorderRadius::all(8),
                    ..UiStyle::default()
                },
                PANEL,
                [
                    ui_text(ids, "当前组合", TEXT, 16, Dimension::Fill),
                    ui_text(
                        ids,
                        selected.map_or("none", |material| material.id.as_str()),
                        MUTED,
                        14,
                        Dimension::Fill,
                    ),
                    ui_text(
                        ids,
                        selected.map_or_else(
                            || "0 层".to_owned(),
                            |material| format!("{} 层", material.layers.len()),
                        ),
                        MUTED,
                        14,
                        Dimension::Fill,
                    ),
                    editor_button(ids, ADD_LAYER_ID, "添加一层", false),
                    editor_button(ids, REMOVE_LAYER_ID, "移除顶层", false),
                    editor_button(ids, DELETE_MATERIAL_ID, "删除当前组合", false),
                ],
            ),
        ],
    )
}

fn editor_asset_card(
    _ids: &mut UiIds,
    action_id: u32,
    atomic: Option<&map_project::AtomicTileId>,
    catalog: &AtomicTileCatalog,
    selected: bool,
) -> UiNode {
    let mut children = Vec::new();
    if let Some(atomic) = atomic {
        if let Some(asset) = catalog.asset(atomic) {
            children.push(
                UiNode::auto()
                    .with_style(UiStyle {
                        width: Dimension::Fill,
                        height: Dimension::Fill,
                        border_radius: punctum_ui::UiBorderRadius::all(5),
                        ..UiStyle::default()
                    })
                    .with_content(UiContent::Image(
                        UiContentId::new(asset.as_str()).expect("tile asset key is non-empty"),
                    )),
            );
        }
    }
    let node = UiNode::auto()
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            padding: Insets::all(4),
            border: punctum_ui::UiBorder {
                widths: Insets::all(1),
                color: ui_color(if selected { SELECTED } else { BORDER }),
            },
            border_radius: punctum_ui::UiBorderRadius::all(6),
            ..UiStyle::default()
        })
        .with_content(UiContent::Fill(ui_color(BUTTON)))
        .with_children(children);
    if atomic.is_some() {
        node.with_action(EditorChromeAction::Command(action_id))
    } else {
        node
    }
}

fn editor_material_card(
    _ids: &mut UiIds,
    action_id: u32,
    material: Option<&map_project::CompositeTile>,
    catalog: &AtomicTileCatalog,
    selected: bool,
) -> UiNode {
    let mut children = Vec::new();
    if let Some(material) = material {
        for layer in &material.layers {
            if let Some(asset) = catalog.asset(layer) {
                children.push(
                    UiNode::auto()
                        .with_style(UiStyle {
                            width: Dimension::Fill,
                            height: Dimension::Fill,
                            border_radius: punctum_ui::UiBorderRadius::all(6),
                            ..UiStyle::default()
                        })
                        .with_content(UiContent::Image(
                            UiContentId::new(asset.as_str()).expect("tile asset key is non-empty"),
                        )),
                );
            }
        }
    }
    let node = UiNode::auto()
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            border: punctum_ui::UiBorder {
                widths: Insets::all(1),
                color: ui_color(if selected { SELECTED } else { BORDER }),
            },
            border_radius: punctum_ui::UiBorderRadius::all(7),
            ..UiStyle::default()
        })
        .with_content(UiContent::Fill(ui_color(BUTTON)))
        .with_children(children);
    if material.is_some() {
        node.with_action(EditorChromeAction::Command(action_id))
    } else {
        node
    }
}

fn editor_button(ids: &mut UiIds, action_id: u32, label: &str, selected: bool) -> UiNode {
    UiNode::auto()
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            main_align: MainAlign::Center,
            cross_align: CrossAlign::Center,
            padding: Insets::symmetric(8, 4),
            border_radius: punctum_ui::UiBorderRadius::all(6),
            ..UiStyle::default()
        })
        .with_action(EditorChromeAction::Command(action_id))
        .with_content(UiContent::Fill(ui_color(if selected {
            SELECTED
        } else {
            BUTTON
        })))
        .with_children([ui_text(ids, label, TEXT, 14, Dimension::Fill)])
}

fn editor_help_dialog(ids: &mut UiIds) -> UiNode {
    let lines = [
        "地图编辑器使用说明",
        "左键绘制，右键擦除。",
        "选择原子素材后可添加组合层。",
        "贴图、碰撞和事件画笔互不修改彼此。",
        "Ctrl+S 保存，Ctrl+Z 撤销；Ctrl+滚轮缩放地图。",
        "点击此面板或“帮助”关闭。",
    ];
    UiNode::auto()
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            direction: FlexDirection::Column,
            main_align: MainAlign::Center,
            cross_align: CrossAlign::Center,
            ..UiStyle::default()
        })
        .with_action(EditorChromeAction::Command(HELP_CLOSE_ID))
        .with_content(UiContent::Fill(UiColor::new(17, 19, 22, 210)))
        .with_children([panel(
            ids.next(),
            UiStyle {
                width: Dimension::Px(660),
                height: Dimension::Px(310),
                direction: FlexDirection::Column,
                gap: 16,
                padding: Insets::all(28),
                border: punctum_ui::UiBorder {
                    widths: Insets::all(2),
                    color: ui_color(BORDER),
                },
                border_radius: punctum_ui::UiBorderRadius::all(14),
                ..UiStyle::default()
            },
            PANEL,
            lines.into_iter().enumerate().map(|(index, line)| {
                ui_text(
                    ids,
                    line,
                    if index == 0 { TEXT } else { MUTED },
                    if index == 0 { 24 } else { 17 },
                    Dimension::Fill,
                )
            }),
        )])
}

fn panel(
    _id: UiId,
    style: UiStyle,
    color: Rgba8,
    children: impl IntoIterator<Item = UiNode>,
) -> UiNode {
    UiNode::auto()
        .with_style(style)
        .with_content(UiContent::Fill(ui_color(color)))
        .with_children(children)
}

fn ui_text(
    _ids: &mut UiIds,
    content: impl Into<String>,
    color: Rgba8,
    font_size: u32,
    width: Dimension,
) -> UiNode {
    UiNode::auto()
        .with_style(UiStyle {
            width,
            height: Dimension::Px(font_size.saturating_add(5)),
            ..UiStyle::default()
        })
        .with_content(UiContent::Text {
            content: content.into(),
            color: ui_color(color),
            font_size,
        })
}

const fn ui_color(color: Rgba8) -> UiColor {
    UiColor::new(color.red, color.green, color.blue, color.alpha)
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

fn project_semantics(
    images: &mut Vec<ViewImage>,
    model: &EditorModel,
    map_viewport: EditorMapViewport,
) {
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
            let rect = GridRect::new(
                GridPos::new(
                    (i32::from(col) - map_viewport.camera_col) * map_viewport.tile_span as i32,
                    (i32::from(row) - map_viewport.camera_row) * map_viewport.tile_span as i32,
                ),
                GridSize::new(map_viewport.tile_span, map_viewport.tile_span),
            );
            if rect.intersection(layout::MAP_RECT).is_some() {
                images.push(image(rect.origin, rect.size, white_asset(), tint, 7));
            }
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
    Ui(UiBuildError),
}

impl fmt::Display for EditorViewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Map(error) => write!(formatter, "map projection failed: {error}"),
            Self::Surface(error) => write!(formatter, "workbench projection failed: {error}"),
            Self::Ui(error) => write!(formatter, "workbench UI projection failed: {error}"),
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
        let frame = project(
            &model,
            &catalog,
            None,
            PixelSize::new(1280, 720),
            EditorMapViewport::default(),
        )
        .unwrap();
        assert_eq!(frame.map.layers().len(), 3);
        assert!(frame.map.layers()[2].surface.is_none());
        let chrome = frame
            .chrome
            .resolve(punctum_ui::UiSize::new(1280, 720))
            .unwrap();
        assert!(chrome.commands().iter().any(|command| matches!(
            command, punctum_ui::UiDrawCommand::Text { content, .. } if content == "原子素材"
        )));
        assert!(chrome.commands().iter().any(|command| matches!(
            command, punctum_ui::UiDrawCommand::Text { content, .. } if content.starts_with("组合素材")
        )));
        assert!(chrome.action_hits().len() >= 10);
    }

    #[test]
    fn centered_viewport_tracks_the_map_center_at_each_zoom_level() {
        let tile = AtomicTileId::new("tile-0001").unwrap();
        let model = EditorModel::new(
            MapProject::blank(
                MapProjectId::new("map").unwrap(),
                72,
                56,
                Some(CompositeTile::new(
                    CompositeTileId::new("material-0000").unwrap(),
                    vec![tile.clone()],
                )),
            ),
            vec![tile],
        );
        assert_eq!(
            centered_map_viewport(&model, 1),
            EditorMapViewport::new(1, 12, 12)
        );
        assert_eq!(
            centered_map_viewport(&model, 2),
            EditorMapViewport::new(2, 24, 20)
        );
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

        let frame = project(
            &model,
            &catalog,
            Some(TilePosition::new(1, 2)),
            PixelSize::new(1280, 720),
            EditorMapViewport::default(),
        )
        .unwrap();

        assert_eq!(frame.map.layers().len(), 3);
        let chrome = frame
            .chrome
            .resolve(punctum_ui::UiSize::new(1280, 720))
            .unwrap();
        assert!(chrome.commands().iter().any(|command| matches!(
            command, punctum_ui::UiDrawCommand::Text { content, .. } if content == "地图编辑器使用说明"
        )));
        assert!(chrome.action_hits().len() >= 11);
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
            let frame = project(
                &model,
                &catalog,
                None,
                PixelSize::new(1280, 720),
                EditorMapViewport::default(),
            )
            .unwrap();
            assert_eq!(frame.map.layers().len(), 3);
            assert!(
                frame.map.layers()[2]
                    .images
                    .iter()
                    .any(|image| image.z_index == 7)
            );
            let chrome = frame
                .chrome
                .resolve(punctum_ui::UiSize::new(1280, 720))
                .unwrap();
            assert!(chrome.commands().iter().any(|command| matches!(
                command, punctum_ui::UiDrawCommand::Text { content, .. } if content == "错误：fixture"
            )));
        }

        let missing = AtomicTileCatalog::new([]).unwrap();
        let error = project(
            &model,
            &missing,
            None,
            PixelSize::new(1280, 720),
            EditorMapViewport::default(),
        )
        .unwrap_err();
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
