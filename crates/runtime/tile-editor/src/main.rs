#![forbid(unsafe_code)]

use std::{collections::BTreeSet, error::Error, fs, path::PathBuf, sync::Arc};

use game_assets::{AssetKey, DecodedImage};
use game_fs_assets::{load_catalog, read_tile_sources};
use game_native_target::{FramePlan, NativeAssets, NativeTarget, PresentOutcome, TextScale};
use map_assets::build_tile_assets;
use map_project::AtomicTileId;
use map_render::AtomicTileCatalog;
use map_tile_semantics::{Direction8, TileSemanticsCatalog};
use punctum_gpu::{PixelSize, Rgba8};
use punctum_ui::{
    CrossAlign, Dimension, FlexDirection, Insets, MainAlign, UiBorder, UiBorderRadius, UiColor,
    UiContent, UiContentId, UiFrame, UiId, UiNode as RawUiNode, UiSize, UiStyle, UiTree,
};
use tile_editor_core::{
    NeighbourPreview, NeighbourRuleKind, StackControl, TileEditorAction, TileEditorSnapshot,
    TileSemanticsEditor,
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

const CLEAR_COLOR: Rgba8 = Rgba8::new(18, 21, 25, 255);
const TEXT_SCALE: TextScale = TextScale::new(10, 18, 10, 20);
const PALETTE_COLUMNS: usize = 10;
const PALETTE_ROWS: usize = 6;
const PAGE_SIZE: usize = PALETTE_COLUMNS * PALETTE_ROWS;
const PALETTE_ID: u32 = 1_000;
const PREVIOUS_PAGE_ID: u32 = 2_000;
const NEXT_PAGE_ID: u32 = 2_001;
const TOGGLE_APPROVED_ID: u32 = 2_100;
const TOGGLE_MEADOW_ID: u32 = 2_101;
const TOGGLE_BASE_ID: u32 = 2_102;
const TOGGLE_BELOW_ID: u32 = 2_103;
const NEIGHBOUR_ID: u32 = 2_200;
const SAVE_ID: u32 = 2_300;

const BACKGROUND: UiColor = UiColor::new(18, 21, 25, 255);
const PANEL: UiColor = UiColor::new(29, 34, 40, 255);
const BUTTON: UiColor = UiColor::new(45, 53, 62, 255);
const SELECTED: UiColor = UiColor::new(53, 190, 165, 255);
const TEXT: UiColor = UiColor::new(236, 241, 244, 255);
const MUTED: UiColor = UiColor::new(155, 170, 182, 255);
const REQUIRED: UiColor = UiColor::new(52, 184, 111, 255);
const FORBIDDEN: UiColor = UiColor::new(220, 83, 79, 255);
const BORDER: UiColor = UiColor::new(79, 93, 105, 255);

struct TileEditorAssets {
    native: NativeAssets,
    catalog: AtomicTileCatalog,
    ids: Vec<AtomicTileId>,
    semantics: TileSemanticsCatalog,
    semantics_path: PathBuf,
}

struct TileEditorApp {
    editor: TileSemanticsEditor,
    assets: NativeAssets,
    catalog: AtomicTileCatalog,
    semantics_path: PathBuf,
    page: usize,
    status: String,
    frame: Option<UiFrame<TileEditorUiAction>>,
    cursor: Option<PhysicalPosition<f64>>,
    window: Option<Arc<Window>>,
    runtime: Option<NativeTarget<'static>>,
}

#[derive(Clone)]
enum TileEditorUiAction {
    Command(u32),
}

type UiNode = RawUiNode<TileEditorUiAction>;

impl TileEditorApp {
    fn new() -> Result<Self, Box<dyn Error>> {
        let loaded = load_assets()?;
        let mut editor = TileSemanticsEditor::new(loaded.semantics, loaded.ids)?;
        editor.apply(TileEditorAction::Select(AtomicTileId::new("tile-0102")?))?;
        Ok(Self {
            editor,
            assets: loaded.native,
            catalog: loaded.catalog,
            semantics_path: loaded.semantics_path,
            page: 0,
            status: "已加载目录".into(),
            frame: None,
            cursor: None,
            window: None,
            runtime: None,
        })
    }

    fn initialize(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn Error>> {
        let window = Arc::new(
            event_loop.create_window(
                Window::default_attributes()
                    .with_title("瓦片编辑器")
                    .with_inner_size(LogicalSize::new(1480.0, 920.0)),
            )?,
        );
        let size = pixel_size(window.inner_size());
        self.runtime = Some(NativeTarget::new(
            window.clone(),
            size,
            &self.assets,
            CLEAR_COLOR,
        )?);
        self.window = Some(window);
        self.update_title();
        self.request_redraw();
        Ok(())
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let Some(size) = self.runtime.as_ref().map(NativeTarget::surface_size) else {
            return;
        };
        let tree = match project_ui(&self.editor, &self.catalog, self.page, &self.status) {
            Ok(tree) => tree,
            Err(error) => {
                self.status = format!("界面错误：{error}");
                self.update_title();
                return;
            }
        };
        let frame = match tree.resolve(UiSize::new(size.width, size.height)) {
            Ok(frame) => frame,
            Err(error) => {
                self.status = format!("布局错误：{error}");
                self.update_title();
                return;
            }
        };
        let plan = match FramePlan::from_ui_frame(&frame, &self.assets, TEXT_SCALE) {
            Ok(plan) => plan,
            Err(error) => {
                self.status = format!("渲染错误：{error}");
                self.update_title();
                return;
            }
        };
        self.frame = Some(frame);
        let (Some(window), Some(runtime)) = (&self.window, &mut self.runtime) else {
            return;
        };
        match runtime.present(&plan) {
            Ok(PresentOutcome::Reconfigured | PresentOutcome::SurfaceLost) => {
                runtime.resize(runtime.surface_size());
                window.request_redraw();
            }
            Ok(_) => {}
            Err(error) => {
                eprintln!("tile editor presentation failed: {error}");
                event_loop.exit();
            }
        }
    }

    fn click(&mut self) {
        let Some(cursor) = self.cursor else {
            return;
        };
        let Some(hit) = self
            .frame
            .as_ref()
            .and_then(|frame| {
                frame.action_hit_at(cursor.x.max(0.0) as u32, cursor.y.max(0.0) as u32)
            })
            .map(|hit| hit.action.clone())
        else {
            return;
        };
        let TileEditorUiAction::Command(hit) = hit;
        match hit {
            id if (PALETTE_ID..PALETTE_ID + PAGE_SIZE as u32).contains(&id) => {
                let index = self.page * PAGE_SIZE + (id - PALETTE_ID) as usize;
                if let Some(tile) = self.editor.ids().get(index).cloned() {
                    self.apply(TileEditorAction::Select(tile));
                }
            }
            PREVIOUS_PAGE_ID => {
                self.page = self
                    .page
                    .checked_sub(1)
                    .unwrap_or_else(|| self.page_count() - 1);
                self.request_redraw();
            }
            NEXT_PAGE_ID => {
                self.page = (self.page + 1) % self.page_count();
                self.request_redraw();
            }
            TOGGLE_APPROVED_ID => self.apply(TileEditorAction::ToggleApproved),
            TOGGLE_MEADOW_ID => self.apply(TileEditorAction::ToggleMeadowTag),
            TOGGLE_BASE_ID => self.apply(TileEditorAction::ToggleStack(StackControl::MustBeBase)),
            TOGGLE_BELOW_ID => self.apply(TileEditorAction::ToggleStack(
                StackControl::RequiresMeadowBelow,
            )),
            id if (NEIGHBOUR_ID..NEIGHBOUR_ID + 8).contains(&id) => {
                if let Some(direction) = direction_at(id - NEIGHBOUR_ID) {
                    self.apply(TileEditorAction::CycleNeighbour(direction));
                }
            }
            SAVE_ID => self.save(),
            _ => {}
        }
    }

    fn apply(&mut self, action: TileEditorAction) {
        match self.editor.apply(action) {
            Ok(()) => {
                self.page = self.editor.selected_index() / PAGE_SIZE;
                self.status = "已修改目录".into();
            }
            Err(error) => self.status = format!("错误：{error}"),
        }
        self.update_title();
        self.request_redraw();
    }

    fn save(&mut self) {
        let result = self.editor.catalog_json().and_then(|json| {
            fs::write(&self.semantics_path, json).map_err(|error| {
                tile_editor_core::TileEditorError::Serialization(error.to_string())
            })
        });
        match result {
            Ok(()) => {
                self.editor.mark_saved();
                self.status = "已保存目录".into();
            }
            Err(error) => self.status = format!("错误：{error}"),
        }
        self.update_title();
        self.request_redraw();
    }

    fn page_count(&self) -> usize {
        self.editor.ids().len().div_ceil(PAGE_SIZE).max(1)
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if let Some(runtime) = &mut self.runtime {
            runtime.resize(pixel_size(size));
        }
        self.request_redraw();
    }

    fn update_title(&self) {
        if let Some(window) = &self.window {
            let dirty = if self.editor.is_dirty() { " *" } else { "" };
            window.set_title(&format!("瓦片编辑器{dirty}"));
        }
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl ApplicationHandler for TileEditorApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none()
            && let Err(error) = self.initialize(event_loop)
        {
            eprintln!("tile editor initialization failed: {error}");
            event_loop.exit();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.window.as_ref().map(|window| window.id()) != Some(window_id) {
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            WindowEvent::Resized(size) => self.resize(size),
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(window) = &self.window {
                    self.resize(window.inner_size());
                }
            }
            WindowEvent::CursorMoved { position, .. } => self.cursor = Some(position),
            WindowEvent::CursorLeft { .. } => self.cursor = None,
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => self.click(),
            _ => {}
        }
    }
}

fn load_assets() -> Result<TileEditorAssets, Box<dyn Error>> {
    let root = asset_root();
    let source_catalog = load_catalog(&root)?;
    let assets = build_tile_assets(read_tile_sources(&root, &source_catalog)?)?;
    let mut images = vec![(
        AssetKey::new("solid/white")?,
        DecodedImage::solid(Rgba8::new(255, 255, 255, 255)),
    )];
    images.extend(assets.images);
    let known = assets.ids.iter().cloned().collect::<BTreeSet<_>>();
    let semantics_path = root.join("source/map/tile/tile-semantics-v1.json");
    Ok(TileEditorAssets {
        native: NativeAssets::new(images)?,
        catalog: assets.catalog,
        ids: assets.ids,
        semantics: TileSemanticsCatalog::from_json(&fs::read_to_string(&semantics_path)?, &known)?,
        semantics_path,
    })
}

fn asset_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../assets")
}

fn project_ui(
    editor: &TileSemanticsEditor,
    catalog: &AtomicTileCatalog,
    page: usize,
    status: &str,
) -> Result<UiTree<TileEditorUiAction>, Box<dyn Error>> {
    let snapshot = editor.snapshot()?;
    let mut ids = UiIds::default();
    let palette = palette_panel(&mut ids, editor, catalog, page);
    let inspector = inspector_panel(&mut ids, &snapshot, catalog, status);
    UiTree::new(
        UiNode::auto()
            .with_style(UiStyle {
                width: Dimension::Fill,
                height: Dimension::Fill,
                direction: FlexDirection::Row,
                padding: Insets::all(16),
                gap: 16,
                ..UiStyle::default()
            })
            .with_content(UiContent::Fill(BACKGROUND))
            .with_children([palette, inspector]),
    )
    .map_err(Into::into)
}

fn palette_panel(
    ids: &mut UiIds,
    editor: &TileSemanticsEditor,
    catalog: &AtomicTileCatalog,
    page: usize,
) -> UiNode {
    let start = page * PAGE_SIZE;
    let mut rows = Vec::new();
    for row in 0..PALETTE_ROWS {
        let cards = (0..PALETTE_COLUMNS)
            .map(|column| {
                let local = row * PALETTE_COLUMNS + column;
                let index = start + local;
                tile_card(
                    ids,
                    PALETTE_ID + local as u32,
                    editor.ids().get(index),
                    catalog,
                    index == editor.selected_index(),
                    UiColor::new(255, 255, 255, 255),
                )
            })
            .collect::<Vec<_>>();
        rows.push(
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(58),
                    direction: FlexDirection::Row,
                    gap: 6,
                    ..UiStyle::default()
                })
                .with_children(cards),
        );
    }
    panel(
        ids.next(),
        UiStyle {
            width: Dimension::Px(660),
            height: Dimension::Fill,
            direction: FlexDirection::Column,
            gap: 10,
            padding: Insets::all(14),
            ..UiStyle::default()
        },
        [
            text(ids, "瓦片", TEXT, 20),
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(410),
                    direction: FlexDirection::Column,
                    gap: 8,
                    ..UiStyle::default()
                })
                .with_children(rows),
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(38),
                    direction: FlexDirection::Row,
                    gap: 8,
                    ..UiStyle::default()
                })
                .with_children([
                    button(ids, PREVIOUS_PAGE_ID, "<", false),
                    text(
                        ids,
                        format!("{}/{}", page + 1, editor.ids().len().div_ceil(PAGE_SIZE)),
                        MUTED,
                        15,
                    ),
                    button(ids, NEXT_PAGE_ID, ">", false),
                ]),
        ],
    )
}

fn inspector_panel(
    ids: &mut UiIds,
    snapshot: &TileEditorSnapshot,
    catalog: &AtomicTileCatalog,
    status: &str,
) -> UiNode {
    let controls = [
        (
            TOGGLE_APPROVED_ID,
            if snapshot.approved {
                "已批准"
            } else {
                "已禁止"
            },
            snapshot.approved,
        ),
        (TOGGLE_MEADOW_ID, "草地", snapshot.meadow),
        (TOGGLE_BASE_ID, "底层", snapshot.must_be_base),
        (TOGGLE_BELOW_ID, "下方草地", snapshot.requires_meadow_below),
    ]
    .into_iter()
    .map(|(id, label, selected)| button(ids, id, label, selected))
    .collect::<Vec<_>>();
    panel(
        ids.next(),
        UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            direction: FlexDirection::Column,
            gap: 12,
            padding: Insets::all(18),
            ..UiStyle::default()
        },
        [
            text(ids, snapshot.id.as_str(), TEXT, 22),
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(40),
                    direction: FlexDirection::Row,
                    gap: 8,
                    ..UiStyle::default()
                })
                .with_children(controls),
            text(ids, "邻域可放置瓦片", TEXT, 18),
            neighbour_grid(ids, snapshot, catalog),
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Px(42),
                    direction: FlexDirection::Row,
                    gap: 8,
                    ..UiStyle::default()
                })
                .with_children([button(ids, SAVE_ID, "保存", false)]),
            text(ids, status, MUTED, 15),
        ],
    )
}

fn neighbour_grid(
    ids: &mut UiIds,
    snapshot: &TileEditorSnapshot,
    catalog: &AtomicTileCatalog,
) -> UiNode {
    let center = NeighbourPreview {
        kind: NeighbourRuleKind::Any,
        accepted_tiles: vec![snapshot.id.clone()],
        locked_by_pattern: false,
    };
    let previews = [
        (&snapshot.neighbours.north_west, Some(Direction8::NorthWest)),
        (&snapshot.neighbours.north, Some(Direction8::North)),
        (&snapshot.neighbours.north_east, Some(Direction8::NorthEast)),
        (&snapshot.neighbours.west, Some(Direction8::West)),
        (&center, None),
        (&snapshot.neighbours.east, Some(Direction8::East)),
        (&snapshot.neighbours.south_west, Some(Direction8::SouthWest)),
        (&snapshot.neighbours.south, Some(Direction8::South)),
        (&snapshot.neighbours.south_east, Some(Direction8::SouthEast)),
    ];
    let mut rows = Vec::new();
    for row in 0..3 {
        let cells = (0..3)
            .map(|column| {
                let (preview, direction) = previews[row * 3 + column];
                neighbour_cell(ids, preview, direction, catalog)
            })
            .collect::<Vec<_>>();
        rows.push(
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Px(516),
                    height: Dimension::Px(164),
                    direction: FlexDirection::Row,
                    gap: 6,
                    ..UiStyle::default()
                })
                .with_children(cells),
        );
    }
    UiNode::auto()
        .with_style(UiStyle {
            width: Dimension::Px(516),
            height: Dimension::Px(516),
            direction: FlexDirection::Column,
            gap: 6,
            ..UiStyle::default()
        })
        .with_children(rows)
}

fn neighbour_cell(
    ids: &mut UiIds,
    preview: &NeighbourPreview,
    direction: Option<Direction8>,
    catalog: &AtomicTileCatalog,
) -> UiNode {
    let action_id = direction.map_or(0, |direction| NEIGHBOUR_ID + direction_index(direction));
    let selected = direction.is_none();
    let color = match preview.kind {
        NeighbourRuleKind::Any => BORDER,
        NeighbourRuleKind::Requires => REQUIRED,
        NeighbourRuleKind::Forbids => FORBIDDEN,
    };
    let label = match direction {
        None => "当前瓦片".to_owned(),
        Some(direction) => format!(
            "{} · {} · {} 个",
            direction_name(direction),
            if preview.locked_by_pattern {
                "图样必须"
            } else {
                rule_kind_name(preview.kind)
            },
            preview.accepted_tiles.len()
        ),
    };
    let node = UiNode::auto()
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            direction: FlexDirection::Column,
            gap: 4,
            padding: Insets::all(5),
            border: UiBorder {
                widths: Insets::all(if selected { 3 } else { 2 }),
                color: if selected { SELECTED } else { color },
            },
            border_radius: UiBorderRadius::all(4),
            ..UiStyle::default()
        })
        .with_content(UiContent::Fill(BUTTON))
        .with_children([
            text(ids, label, TEXT, 12),
            candidate_tiles(ids, &preview.accepted_tiles, catalog),
        ]);
    if !selected && !preview.locked_by_pattern {
        node.with_action(TileEditorUiAction::Command(action_id))
    } else {
        node
    }
}

fn candidate_tiles(ids: &mut UiIds, tiles: &[AtomicTileId], catalog: &AtomicTileCatalog) -> UiNode {
    if tiles.is_empty() {
        return text(ids, "无可放置瓦片", FORBIDDEN, 12);
    }
    let rows = tiles
        .iter()
        .take(4)
        .collect::<Vec<_>>()
        .chunks(2)
        .map(|row| {
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    direction: FlexDirection::Row,
                    gap: 4,
                    ..UiStyle::default()
                })
                .with_children(row.iter().map(|tile| tile_thumbnail(ids, tile, catalog)))
        })
        .collect::<Vec<_>>();
    UiNode::auto()
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            direction: FlexDirection::Column,
            gap: 4,
            ..UiStyle::default()
        })
        .with_children(rows)
}

fn tile_thumbnail(_ids: &mut UiIds, tile: &AtomicTileId, catalog: &AtomicTileCatalog) -> UiNode {
    let mut children = Vec::new();
    if let Some(asset) = catalog.asset(tile) {
        children.push(
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    ..UiStyle::default()
                })
                .with_content(UiContent::Image(UiContentId::from_resource_key(
                    asset.as_str(),
                ))),
        );
    }
    UiNode::auto()
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            ..UiStyle::default()
        })
        .with_content(UiContent::Fill(PANEL))
        .with_children(children)
}

fn rule_kind_name(kind: NeighbourRuleKind) -> &'static str {
    match kind {
        NeighbourRuleKind::Any => "任意",
        NeighbourRuleKind::Requires => "必须",
        NeighbourRuleKind::Forbids => "排除",
    }
}

fn direction_name(direction: Direction8) -> &'static str {
    match direction {
        Direction8::North => "上",
        Direction8::NorthEast => "右上",
        Direction8::East => "右",
        Direction8::SouthEast => "右下",
        Direction8::South => "下",
        Direction8::SouthWest => "左下",
        Direction8::West => "左",
        Direction8::NorthWest => "左上",
    }
}

fn tile_card(
    _ids: &mut UiIds,
    action_id: u32,
    tile: Option<&AtomicTileId>,
    catalog: &AtomicTileCatalog,
    selected: bool,
    border_color: UiColor,
) -> UiNode {
    let mut children = Vec::new();
    if let Some(tile) = tile
        && let Some(asset) = catalog.asset(tile)
    {
        children.push(
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    ..UiStyle::default()
                })
                .with_content(UiContent::Image(UiContentId::from_resource_key(
                    asset.as_str(),
                ))),
        );
    }
    let node = UiNode::auto()
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            padding: Insets::all(4),
            border: UiBorder {
                widths: Insets::all(if selected { 3 } else { 2 }),
                color: if selected { SELECTED } else { border_color },
            },
            border_radius: UiBorderRadius::all(4),
            ..UiStyle::default()
        })
        .with_content(UiContent::Fill(BUTTON))
        .with_children(children);
    if action_id != 0 && tile.is_some() {
        node.with_action(TileEditorUiAction::Command(action_id))
    } else {
        node
    }
}

fn panel(_id: UiId, style: UiStyle, children: impl IntoIterator<Item = UiNode>) -> UiNode {
    UiNode::auto()
        .with_style(UiStyle {
            border: UiBorder {
                widths: Insets::all(1),
                color: BORDER,
            },
            border_radius: UiBorderRadius::all(6),
            ..style
        })
        .with_content(UiContent::Fill(PANEL))
        .with_children(children)
}

fn button(ids: &mut UiIds, id: u32, label: &str, selected: bool) -> UiNode {
    UiNode::auto()
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Fill,
            direction: FlexDirection::Stack,
            main_align: MainAlign::Center,
            cross_align: CrossAlign::Center,
            padding: Insets::symmetric(8, 4),
            border_radius: UiBorderRadius::all(4),
            ..UiStyle::default()
        })
        .with_action(TileEditorUiAction::Command(id))
        .with_content(UiContent::Fill(if selected { SELECTED } else { BUTTON }))
        .with_children([text(ids, label, TEXT, 14)])
}

fn text(_ids: &mut UiIds, content: impl Into<String>, color: UiColor, font_size: u32) -> UiNode {
    UiNode::auto()
        .with_style(UiStyle {
            width: Dimension::Fill,
            height: Dimension::Px(font_size.saturating_add(8)),
            main_align: MainAlign::Center,
            cross_align: CrossAlign::Center,
            ..UiStyle::default()
        })
        .with_content(UiContent::Text {
            content: content.into(),
            color,
            font_size,
        })
}

fn direction_index(direction: Direction8) -> u32 {
    match direction {
        Direction8::North => 0,
        Direction8::NorthEast => 1,
        Direction8::East => 2,
        Direction8::SouthEast => 3,
        Direction8::South => 4,
        Direction8::SouthWest => 5,
        Direction8::West => 6,
        Direction8::NorthWest => 7,
    }
}

fn direction_at(index: u32) -> Option<Direction8> {
    match index {
        0 => Some(Direction8::North),
        1 => Some(Direction8::NorthEast),
        2 => Some(Direction8::East),
        3 => Some(Direction8::SouthEast),
        4 => Some(Direction8::South),
        5 => Some(Direction8::SouthWest),
        6 => Some(Direction8::West),
        7 => Some(Direction8::NorthWest),
        _ => None,
    }
}

#[derive(Default)]
struct UiIds {
    next: u32,
}

impl UiIds {
    fn next(&mut self) -> UiId {
        self.next += 1;
        UiId(10_000 + self.next)
    }
}

fn pixel_size(size: PhysicalSize<u32>) -> PixelSize {
    PixelSize::new(size.width, size.height)
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = TileEditorApp::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}
