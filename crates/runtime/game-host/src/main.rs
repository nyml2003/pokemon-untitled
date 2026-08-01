//! 真实产品会话驱动的玩家页面原生宿主。

#![forbid(unsafe_code)]

mod map;
mod narrative;
mod sprites;
mod thin_slice;
mod trainer_content;

use std::{
    error::Error,
    sync::Arc,
    time::{Duration, Instant},
};

use game_data::CurrentDataSet;
use game_foundation::{ContentPackage, ContentPackageDocument, SaveEnvelope, ThinSliceContent};
use game_native_target::{
    FramePlan, NativeAssets, NativeTarget, PresentOutcome, TextScale, WinitCommittedTextSnapshot,
    WinitKeyEventSnapshot, instance_for_event_loop, normalize_committed_text, normalize_key_event,
};
use game_page_model::{PageEffect, PageIntent, PageModel, PageState, PlayerRoute, project_page};
use game_scene_view::{SceneFrame, SceneViewInput, game_viewport, project_scene};
use game_session::{DEBUG_PRESETS, GameScene, ProductCommand, ProductSession};
use game_ui::{GameControl, GameRuntime, PageUiOutcome, PageUiState};
use game_view::project_page_model_with_visual_state;
use map::load_map;
use map_project::MapProject;
use map_render::AtomicTileCatalog;
use narrative::load_narrative_scripts;
use punctum_gpu::{PixelSize, Rgba8};
use punctum_input::{KeyPhase, PhysicalKeyCode};
use punctum_ui::{
    Dimension, Insets, Position, UiBorderRadius, UiColor, UiContent, UiFrame, UiInteraction,
    UiInteractionTarget, UiNode, UiSize, UiStyle, UiTree,
};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::ModifiersState,
    window::{Window, WindowId},
};

const CLEAR_COLOR: Rgba8 = Rgba8::new(14, 18, 24, 255);
const GAME_TEXT_SCALE: TextScale = TextScale::new(3, 5, 10, 28);
const PAGE_TEXT_SCALE: TextScale = TextScale::new(1, 1, 16, 28);
const FOUNDATION_SAVE_PATH: &str = "target/foundation-page.save.json";

struct CreatureGameApp {
    product: Option<ProductSession>,
    game: GameRuntime,
    content: ThinSliceContent,
    page_state: PageState,
    page_ui: PageUiState,
    pokedex: game_data::PokedexData,
    map_project: MapProject,
    map_catalog: AtomicTileCatalog,
    status: Option<String>,
    assets: NativeAssets,
    modifiers: ModifiersState,
    cursor: Option<PhysicalPosition<f64>>,
    page_frame: Option<UiFrame<PageIntent>>,
    interaction: UiInteraction,
    interaction_targets: Vec<UiInteractionTarget>,
    last_real_instant: Instant,
    last_ui_instant: Instant,
    window: Option<Arc<Window>>,
    runtime: Option<NativeTarget<'static>>,
    debug_preset_index: usize,
    debug_preset_active: bool,
}

/// 以系统时钟与进程号派生每次启动不同的队伍随机种子。
fn random_roster_seed() -> u64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    (now ^ u128::from(std::process::id())) as u64
}

impl CreatureGameApp {
    fn new() -> Result<Self, Box<dyn Error>> {
        let package = load_product_content_package()?;
        let content = package.content().clone();
        let product = ProductSession::from_package(CurrentDataSet::embedded()?, package)
            .map_err(|error| std::io::Error::other(format!("product session: {error:?}")))?;
        let loaded_map = load_map()?;
        let world = world_application::WorldApplication::from_map_project_with_scripts(
            &loaded_map.project,
            load_narrative_scripts()?,
        )
        .map_err(|error| std::io::Error::other(format!("map world: {error:?}")))?;
        let game = GameRuntime::new(CurrentDataSet::embedded()?, world, random_roster_seed())
            .map_err(|error| std::io::Error::other(format!("map game: {error:?}")))?;
        let pokedex = game_data::PokedexData::embedded_gen3()?;
        let assets = sprites::load_host_assets(
            &game
                .sprite_manifest()
                .ok_or("game runtime unavailable")?
                .map_err(|error| std::io::Error::other(format!("sprite manifest: {error:?}")))?,
            &pokedex,
            game.snapshot().ok_or("game runtime unavailable")?.world(),
            loaded_map.images,
        )?;
        let now = Instant::now();
        Ok(Self {
            product: Some(product),
            game,
            content,
            page_state: PageState::world(),
            page_ui: PageUiState::default(),
            pokedex,
            map_project: loaded_map.project,
            map_catalog: loaded_map.catalog,
            status: None,
            assets,
            modifiers: ModifiersState::empty(),
            cursor: None,
            page_frame: None,
            interaction: UiInteraction::default(),
            interaction_targets: Vec::new(),
            last_real_instant: now,
            last_ui_instant: now,
            window: None,
            runtime: None,
            debug_preset_index: 0,
            debug_preset_active: false,
        })
    }

    fn page_model(&self) -> Result<PageModel, String> {
        let product = self
            .product
            .as_ref()
            .ok_or_else(|| String::from("product session is unavailable"))?;
        project_page(&self.content, &product.snapshot(), self.page_state.route())
            .map_err(|error| format!("page model: {error}"))
    }

    fn submit_product(&mut self, command: ProductCommand) -> Result<(), String> {
        let product = self
            .product
            .take()
            .ok_or_else(|| String::from("product session is unavailable"))?;
        let (product, result) = product.transition(command);
        self.product = Some(product);
        result
            .map(|_| ())
            .map_err(|error| format!("product command rejected: {error:?}"))
    }

    fn save_product(&mut self) -> Result<(), String> {
        let product = self
            .product
            .as_ref()
            .ok_or_else(|| String::from("product session is unavailable"))?;
        let save = product
            .save()
            .map_err(|error| format!("product save: {error:?}"))?;
        let bytes = save
            .to_json()
            .map_err(|error| format!("save encode: {error:?}"))?;
        let loaded = SaveEnvelope::from_json(&self.content, &bytes)
            .map_err(|error| format!("save reload: {error:?}"))?;
        let data = CurrentDataSet::embedded().map_err(|error| format!("data reload: {error:?}"))?;
        let product = ProductSession::from_save(data, self.content.clone(), loaded)
            .map_err(|error| format!("product reload: {error:?}"))?;
        write_foundation_save(&bytes)?;
        self.product = Some(product);
        Ok(())
    }

    fn initialize(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn Error>> {
        let window = Arc::new(
            event_loop.create_window(
                Window::default_attributes()
                    .with_title("宝可梦：还没想好名字")
                    .with_inner_size(LogicalSize::new(960.0, 720.0)),
            )?,
        );
        let instance = instance_for_event_loop(event_loop);
        let runtime = NativeTarget::new(
            &instance,
            window.clone(),
            pixel_size(window.inner_size()),
            &self.assets,
            CLEAR_COLOR,
        )?;
        window.set_ime_allowed(true);
        window.request_redraw();
        self.window = Some(window);
        self.runtime = Some(runtime);
        Ok(())
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        self.advance_ui(Instant::now());
        let Some(surface_size) = self.runtime.as_ref().map(NativeTarget::surface_size) else {
            return;
        };
        if matches!(self.page_state.route(), PlayerRoute::World) {
            self.redraw_world(event_loop, surface_size);
            return;
        }
        let model = match self.page_model() {
            Ok(model) => model,
            Err(error) => {
                eprintln!("page model construction failed: {error}");
                event_loop.exit();
                return;
            }
        };
        self.page_ui.sync(&model);
        let tree = match project_page_model_with_visual_state(
            &model,
            self.status.as_deref(),
            Some(self.page_ui.pokedex_visual_state()),
            UiSize::new(surface_size.width, surface_size.height),
        ) {
            Ok(tree) => tree,
            Err(error) => {
                eprintln!("page tree construction failed: {error}");
                event_loop.exit();
                return;
            }
        };
        let frame = match tree.resolve(UiSize::new(surface_size.width, surface_size.height)) {
            Ok(frame) => frame,
            Err(error) => {
                eprintln!("page layout failed: {error}");
                event_loop.exit();
                return;
            }
        };
        self.interaction_targets = frame.interaction_targets().to_vec();
        self.interaction.reconcile(&self.interaction_targets);
        self.sync_keyboard_focus(&model, &frame);
        self.sync_pointer();
        let frame = frame.with_interaction(self.interaction.snapshot());
        let plan = match FramePlan::from_ui_frame(&frame, &self.assets, PAGE_TEXT_SCALE) {
            Ok(plan) => plan,
            Err(error) => {
                eprintln!("page GPU planning failed: {error}");
                event_loop.exit();
                return;
            }
        };
        self.page_frame = Some(frame);
        let (Some(window), Some(runtime)) = (&self.window, &mut self.runtime) else {
            return;
        };
        match runtime.present(&plan) {
            Ok(PresentOutcome::Reconfigured | PresentOutcome::SurfaceLost) => {
                runtime.resize(runtime.surface_size());
                window.request_redraw();
            }
            Ok(
                PresentOutcome::Presented
                | PresentOutcome::PresentedAndReconfigured
                | PresentOutcome::SkippedMinimized
                | PresentOutcome::SkippedTimeout
                | PresentOutcome::SkippedOccluded,
            ) => {}
            Err(error) => {
                eprintln!("page presentation failed: {error}");
                event_loop.exit();
            }
        }
    }

    fn redraw_world(&mut self, event_loop: &ActiveEventLoop, surface_size: PixelSize) {
        self.advance_presentation(Instant::now());
        let Some(snapshot) = self.game.snapshot() else {
            event_loop.exit();
            return;
        };
        let viewport = game_viewport(surface_size);
        let Some(presentation) = self.game.presentation_snapshot(viewport.cell_size) else {
            event_loop.exit();
            return;
        };
        let projected = match project_scene(SceneViewInput {
            game: &snapshot,
            presentation,
            console: None,
            pokedex: &self.pokedex,
            map_project: &self.map_project,
            map_catalog: &self.map_catalog,
            viewport,
        }) {
            Ok(frame) => frame,
            Err(error) => {
                eprintln!("map scene projection failed: {error}");
                event_loop.exit();
                return;
            }
        };
        let plan = match projected.frame {
            SceneFrame::Grid(view) => {
                FramePlan::from_game_view(&view, &self.assets, viewport, GAME_TEXT_SCALE)
            }
            SceneFrame::Ui(frame) => {
                FramePlan::from_ui_frame(&frame, &self.assets, PAGE_TEXT_SCALE)
            }
            SceneFrame::Pokedex(frame) => {
                FramePlan::from_ui_frame(&frame, &self.assets, PAGE_TEXT_SCALE)
            }
            SceneFrame::GridWithUi { base, overlay } => {
                let base =
                    FramePlan::from_game_view(&base, &self.assets, viewport, GAME_TEXT_SCALE);
                let overlay = FramePlan::from_ui_frame(&overlay, &self.assets, PAGE_TEXT_SCALE);
                match (base, overlay) {
                    (Ok(base), Ok(overlay)) => Ok(FramePlan::compose(base, overlay)),
                    (Err(error), _) | (_, Err(error)) => Err(error),
                }
            }
            SceneFrame::UiWithUi { base, overlay } => {
                let base = FramePlan::from_ui_frame(&base, &self.assets, PAGE_TEXT_SCALE);
                let overlay = FramePlan::from_ui_frame(&overlay, &self.assets, PAGE_TEXT_SCALE);
                match (base, overlay) {
                    (Ok(base), Ok(overlay)) => Ok(FramePlan::compose(base, overlay)),
                    (Err(error), _) | (_, Err(error)) => Err(error),
                }
            }
            SceneFrame::PokedexWithUi { base, overlay } => {
                let base = FramePlan::from_ui_frame(&base, &self.assets, PAGE_TEXT_SCALE);
                let overlay = FramePlan::from_ui_frame(&overlay, &self.assets, PAGE_TEXT_SCALE);
                match (base, overlay) {
                    (Ok(base), Ok(overlay)) => Ok(FramePlan::compose(base, overlay)),
                    (Err(error), _) | (_, Err(error)) => Err(error),
                }
            }
        };
        let plan = match plan {
            Ok(plan) => plan,
            Err(error) => {
                eprintln!("map GPU planning failed: {error}");
                event_loop.exit();
                return;
            }
        };
        let plan = match self.debug_badge_frame(surface_size) {
            Some(badge) => {
                let badge = FramePlan::from_ui_frame(&badge, &self.assets, PAGE_TEXT_SCALE);
                match (Ok(plan), badge) {
                    (Ok(plan), Ok(badge)) => Ok(FramePlan::compose(plan, badge)),
                    (Err(error), _) | (_, Err(error)) => Err(error),
                }
            }
            None => Ok(plan),
        };
        let plan = match plan {
            Ok(plan) => plan,
            Err(error) => {
                eprintln!("debug badge planning failed: {error}");
                event_loop.exit();
                return;
            }
        };
        let (Some(window), Some(runtime)) = (&self.window, &mut self.runtime) else {
            return;
        };
        if let Err(error) = runtime.update_assets(&self.assets) {
            eprintln!("atlas refresh failed: {error}");
            event_loop.exit();
            return;
        }
        match runtime.present(&plan) {
            Ok(PresentOutcome::Reconfigured | PresentOutcome::SurfaceLost) => {
                runtime.resize(runtime.surface_size());
                window.request_redraw();
            }
            Ok(_) => {}
            Err(error) => {
                eprintln!("map presentation failed: {error}");
                event_loop.exit();
            }
        }
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if let Some(runtime) = &mut self.runtime {
            runtime.resize(pixel_size(size));
        }
        self.request_redraw();
    }

    /// 用调试预置队伍重建游戏与精灵资源；只应在世界场景调用。
    fn restart_with_debug_preset(&mut self, preset_index: usize) -> Result<(), String> {
        let preset = DEBUG_PRESETS.get(preset_index).ok_or("调试队伍索引越界")?;
        let data = CurrentDataSet::embedded().map_err(|error| format!("数据加载失败：{error}"))?;
        let loaded_map = load_map().map_err(|error| format!("地图加载失败：{error}"))?;
        let scripts = load_narrative_scripts().map_err(|error| format!("脚本加载失败：{error}"))?;
        let world = world_application::WorldApplication::from_map_project_with_scripts(
            &loaded_map.project,
            scripts,
        )
        .map_err(|error| format!("世界构建失败：{error:?}"))?;
        let game = GameRuntime::new_with_debug_preset(data, world, preset)
            .map_err(|error| format!("游戏构建失败：{error:?}"))?;
        let manifest = game
            .sprite_manifest()
            .ok_or("游戏运行时不可用")?
            .map_err(|error| format!("精灵清单失败：{error:?}"))?;
        let assets = sprites::load_host_assets(
            &manifest,
            &self.pokedex,
            game.snapshot().ok_or("游戏运行时不可用")?.world(),
            loaded_map.images,
        )
        .map_err(|error| format!("精灵资源加载失败：{error}"))?;
        self.game = game;
        self.assets = assets;
        self.map_project = loaded_map.project;
        self.map_catalog = loaded_map.catalog;
        self.debug_preset_index = preset_index;
        self.debug_preset_active = true;
        self.status = Some(format!("调试队伍 {}：{}", preset_index + 1, preset.name));
        self.request_redraw();
        Ok(())
    }

    /// 在角落叠加调试提示：未切换时提示按键，切换后显示当前队伍。
    fn debug_badge_frame(&self, size: PixelSize) -> Option<UiFrame> {
        let label = if self.debug_preset_active {
            let preset = DEBUG_PRESETS.get(self.debug_preset_index)?;
            format!(
                "调试队伍 {}：{}（F5 切换）",
                self.debug_preset_index + 1,
                preset.name
            )
        } else {
            "按 F5 切换调试队伍".to_owned()
        };
        let tree = UiTree::new(
            UiNode::auto()
                .with_style(UiStyle {
                    width: Dimension::Fill,
                    height: Dimension::Fill,
                    ..UiStyle::default()
                })
                .with_children([UiNode::auto()
                    .with_style(UiStyle {
                        position: Position::Absolute { left: 12, top: 12 },
                        padding: Insets::all(8),
                        border_radius: UiBorderRadius::all(8),
                        ..UiStyle::default()
                    })
                    .with_content(UiContent::Fill(UiColor::new(18, 24, 32, 220)))
                    .with_children([UiNode::auto()
                        .with_style(UiStyle {
                            padding: Insets::all(2),
                            ..UiStyle::default()
                        })
                        .with_content(UiContent::Text {
                            content: label,
                            color: UiColor::new(240, 220, 140, 255),
                            font_size: 16,
                        })])]),
        )
        .ok()?;
        tree.resolve(UiSize::new(size.width, size.height)).ok()
    }

    fn handle_key(&mut self, event: winit::event::KeyEvent) {
        let text = match normalize_committed_text(WinitCommittedTextSnapshot::new(
            event.text.map(|text| text.to_string()),
        )) {
            Ok(text) => text,
            Err(error) => {
                self.status = Some(format!("文本输入不可用：{error}"));
                self.request_redraw();
                return;
            }
        };
        let key = normalize_key_event(WinitKeyEventSnapshot::new(
            event.physical_key,
            event.logical_key,
            self.modifiers,
            event.state,
            event.repeat,
        ));
        let model = match self.page_model() {
            Ok(model) => model,
            Err(error) => {
                self.status = Some(error);
                self.request_redraw();
                return;
            }
        };
        if matches!(model, PageModel::World(_)) {
            if key.phase == KeyPhase::Press && key.physical == Some(PhysicalKeyCode::F5) {
                let in_battle = self
                    .game
                    .snapshot()
                    .is_some_and(|snapshot| snapshot.scene() == GameScene::Battle);
                if in_battle {
                    self.status = Some("请在非战斗场景按 F5 切换调试队伍".to_owned());
                } else {
                    let next = (self.debug_preset_index + 1) % DEBUG_PRESETS.len();
                    if let Err(error) = self.restart_with_debug_preset(next) {
                        self.status = Some(error);
                    }
                }
                self.request_redraw();
                return;
            }
            let control = GameControl::from_key_event(&key);
            if key.phase == KeyPhase::Press && control == Some(GameControl::Start) {
                self.dispatch_page_intent(PageIntent::OpenPause);
                return;
            }
            let update = self
                .game
                .handle_key(&key, text.as_ref(), self.modifiers.shift_key());
            if update.ime_changed || update.redraw {
                self.request_redraw();
            }
            return;
        }
        let outcome = self.page_ui.handle_input(&key, text.as_ref(), &model);
        self.handle_page_outcome(outcome);
    }

    fn handle_page_outcome(&mut self, outcome: PageUiOutcome) {
        match outcome {
            PageUiOutcome::Intent(intent) => self.dispatch_page_intent(intent),
            PageUiOutcome::Updated => self.request_redraw(),
            PageUiOutcome::Ignored => {}
        }
    }

    fn dispatch_page_intent(&mut self, intent: PageIntent) {
        let model = match self.page_model() {
            Ok(model) => model,
            Err(error) => {
                self.status = Some(error);
                self.request_redraw();
                return;
            }
        };
        if let Some(outcome) = self.page_ui.handle_view_intent(&intent, &model) {
            self.handle_page_outcome(outcome);
            return;
        }
        let (state, effect) = match self.page_state.clone().transition(intent.clone()) {
            Ok(next) => next,
            Err(error) => {
                self.status = Some(format!("操作未执行：{error}"));
                self.request_redraw();
                return;
            }
        };
        self.page_state = state;
        self.status = match effect {
            Some(PageEffect::SubmitProduct(command)) => self.submit_product(command).err(),
            Some(PageEffect::RequestSave) => self.save_product().err(),
            None => None,
        };
        if let Ok(model) = self.page_model() {
            self.page_ui.focus_intent(&intent, &model);
        }
        self.request_redraw();
    }

    fn sync_keyboard_focus(&mut self, model: &PageModel, frame: &UiFrame<PageIntent>) {
        let Some(key) = self.page_ui.action_key(model) else {
            self.interaction.focus(None);
            return;
        };
        let id = frame
            .action_hits()
            .iter()
            .find(|hit| {
                hit.key
                    .as_ref()
                    .is_some_and(|hit_key| hit_key.as_str() == key)
            })
            .map(|hit| hit.id);
        self.interaction.focus(id);
    }

    fn sync_pointer(&mut self) {
        let Some(cursor) = self.cursor else {
            return;
        };
        let Some((x, y)) = cursor_position(cursor) else {
            return;
        };
        if self
            .interaction
            .pointer_move(&self.interaction_targets, x, y)
        {
            self.interaction.focus(self.interaction.snapshot().hovered);
            self.request_redraw();
        }
    }

    fn handle_pointer_press(&mut self) {
        let Some(cursor) = self.cursor else {
            return;
        };
        let Some((x, y)) = cursor_position(cursor) else {
            return;
        };
        self.interaction.press(&self.interaction_targets, x, y);
        self.request_redraw();
    }

    fn handle_pointer_release(&mut self) {
        let Some(cursor) = self.cursor else {
            self.interaction.pointer_leave();
            return;
        };
        let Some((x, y)) = cursor_position(cursor) else {
            return;
        };
        let Some(id) = self.interaction.release(&self.interaction_targets, x, y) else {
            self.request_redraw();
            return;
        };
        let Some(intent) = self
            .page_frame
            .as_ref()
            .and_then(|frame| frame.action_hit_by_id(id))
            .map(|hit| hit.action.clone())
        else {
            self.request_redraw();
            return;
        };
        self.dispatch_page_intent(intent);
    }

    fn advance_ui(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.last_ui_instant);
        self.last_ui_instant = now;
        if self.page_ui.advance(elapsed) || self.interaction.advance(elapsed) {
            self.request_redraw();
        }
    }

    fn advance_presentation(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.last_real_instant);
        self.last_real_instant = now;
        let update = self.game.advance(elapsed);
        if update.ime_changed || update.redraw {
            self.request_redraw();
        }
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl ApplicationHandler for CreatureGameApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none()
            && let Err(error) = self.initialize(event_loop)
        {
            eprintln!("game initialization failed: {error}");
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
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::Focused(false) => {
                self.interaction.clear_transient();
                let update = self.game.focus_lost();
                if update.redraw {
                    self.request_redraw();
                }
                self.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } => self.handle_key(event),
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = Some(position);
                self.sync_pointer();
            }
            WindowEvent::CursorLeft { .. } => {
                self.cursor = None;
                if self.interaction.pointer_leave() {
                    self.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => self.handle_pointer_press(),
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => self.handle_pointer_release(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        self.advance_ui(now);
        if matches!(self.page_state.route(), PlayerRoute::World) {
            self.advance_presentation(now);
            match self.game.next_delay() {
                Some(delay) => {
                    self.request_redraw();
                    event_loop
                        .set_control_flow(winit::event_loop::ControlFlow::WaitUntil(now + delay));
                }
                None if self.game.snapshot().is_none() => {
                    event_loop.exit();
                    return;
                }
                None => {
                    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
                }
            }
            return;
        }
        let motion_delay = self
            .page_ui
            .pokedex_motion_active()
            .then_some(Duration::from_millis(16));
        let next_delay = motion_delay
            .into_iter()
            .chain(self.page_ui.pokedex_filter_next_delay())
            .chain(self.interaction.next_delay())
            .min();
        if let Some(delay) = next_delay {
            self.request_redraw();
            event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(now + delay));
        } else {
            event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
        }
    }
}

fn cursor_position(position: PhysicalPosition<f64>) -> Option<(u32, u32)> {
    let x = coordinate(position.x)?;
    let y = coordinate(position.y)?;
    Some((x, y))
}

fn coordinate(value: f64) -> Option<u32> {
    if !value.is_finite() || value.is_sign_negative() || value > f64::from(u32::MAX) {
        return None;
    }
    Some(value as u32)
}

fn load_product_content_package() -> Result<ContentPackage, Box<dyn Error>> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../assets/content/starter-region/content.json");
    let json = std::fs::read_to_string(&path)?;
    let document = ContentPackageDocument::from_json(&json).map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("content package {}: {error:?}", path.display()),
        )
    })?;
    document.into_package().map_err(|error| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("content package {}: {error:?}", path.display()),
        )
        .into()
    })
}

fn write_foundation_save(bytes: &[u8]) -> Result<(), String> {
    let save_path = std::path::Path::new(FOUNDATION_SAVE_PATH);
    let parent = save_path
        .parent()
        .ok_or_else(|| String::from("save path has no parent directory"))?;
    std::fs::create_dir_all(parent).map_err(|error| format!("save directory: {error}"))?;
    let temporary = parent.join(format!("foundation-page.{}.tmp", std::process::id()));
    std::fs::write(&temporary, bytes).map_err(|error| format!("save write: {error}"))?;
    if let Err(error) = std::fs::rename(&temporary, save_path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(format!("save replace: {error}"));
    }
    Ok(())
}

fn pixel_size(size: PhysicalSize<u32>) -> PixelSize {
    PixelSize::new(size.width, size.height)
}

fn main() -> Result<(), Box<dyn Error>> {
    if let Some(exit) = thin_slice::run_from_arguments(std::env::args_os().skip(1))? {
        return exit;
    }
    let event_loop = EventLoop::new().map_err(|error| {
        std::io::Error::other(format!(
            "窗口系统初始化失败，可能是 WSLg 显示会话不可用（切换显示器/分辨率后常见）；请重启 WSL（wsl --shutdown）后重试：{error}"
        ))
    })?;
    let mut app = CreatureGameApp::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/game_host.rs"]
mod tests;
