mod assets;

use std::{collections::BTreeSet, error::Error, fs, mem, path::PathBuf, sync::Arc};

use assets::{default_project_path, load_assets, load_project};
use game_native_target::{
    FramePlan, NativeAssets, NativeTarget, PresentOutcome, TextScale, WinitKeyEventSnapshot,
    normalize_key_event,
};
use map_editor_core::{
    EditorController, EditorEffect, EditorIntent, EditorModel, PointerButton, key_intent,
    wheel_intent,
};
use map_editor_view::{centered_map_viewport, editor_viewport, intent_for_ui_hit, project};
use map_render::AtomicTileCatalog;
use map_tile_semantics::TileSemanticsCatalog;
use punctum_gpu::{PixelSize, Rgba8, Viewport};
use punctum_ui::UiFrame;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::ModifiersState,
    window::{Window, WindowId},
};

const CLEAR_COLOR: Rgba8 = Rgba8::new(17, 19, 22, 255);
const EDITOR_TEXT_SCALE: TextScale = TextScale::new(11, 20, 11, 22);
const MIN_MAP_TILE_SPAN: u32 = 1;
const MAX_MAP_TILE_SPAN: u32 = 4;

struct MapEditorApp {
    project_path: PathBuf,
    model: EditorModel,
    controller: EditorController,
    assets: NativeAssets,
    catalog: AtomicTileCatalog,
    semantics: TileSemanticsCatalog,
    modifiers: ModifiersState,
    map_tile_span: u32,
    viewport: Viewport,
    chrome: Option<UiFrame>,
    cursor: Option<winit::dpi::PhysicalPosition<f64>>,
    window: Option<Arc<Window>>,
    runtime: Option<NativeTarget<'static>>,
}

impl MapEditorApp {
    fn new() -> Result<Self, Box<dyn Error>> {
        let assets = load_assets()?;
        let project_path = std::env::args_os()
            .nth(1)
            .map(PathBuf::from)
            .unwrap_or_else(default_project_path);
        let project = load_project(&project_path, &assets.project_ids)?;
        let model = EditorModel::with_semantics(project, assets.ids, assets.semantics.clone());
        Ok(Self {
            project_path,
            model,
            controller: EditorController::default(),
            assets: assets.native,
            catalog: assets.catalog,
            semantics: assets.semantics,
            modifiers: ModifiersState::empty(),
            map_tile_span: map_editor_core::layout::MAP_TILE_SPAN,
            viewport: editor_viewport(PixelSize::new(1600, 950)),
            chrome: None,
            cursor: None,
            window: None,
            runtime: None,
        })
    }

    fn initialize(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn Error>> {
        let window = Arc::new(
            event_loop.create_window(
                Window::default_attributes()
                    .with_title("Gen3 地图编辑器")
                    .with_inner_size(LogicalSize::new(1600.0, 950.0)),
            )?,
        );
        let size = pixel_size(window.inner_size());
        let runtime = NativeTarget::new(window.clone(), size, &self.assets, CLEAR_COLOR)?;
        self.viewport = editor_viewport(size);
        self.window = Some(window);
        self.runtime = Some(runtime);
        self.update_title();
        self.request_redraw();
        Ok(())
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let Some(target_size) = self.runtime.as_ref().map(NativeTarget::surface_size) else {
            return;
        };
        let map_viewport = self.map_viewport();
        let frame = match project(
            &self.model,
            &self.catalog,
            self.controller.hover,
            target_size,
            map_viewport,
        ) {
            Ok(frame) => frame,
            Err(error) => {
                self.model = self.model.with_error(error);
                self.update_title();
                return;
            }
        };
        self.viewport = frame.viewport;
        let chrome = match frame.chrome.resolve(punctum_ui::UiSize::new(
            target_size.width,
            target_size.height,
        )) {
            Ok(chrome) => chrome,
            Err(error) => {
                self.model = self.model.with_error(error);
                self.update_title();
                return;
            }
        };
        let map_plan = match FramePlan::from_game_view(
            &frame.map,
            &self.assets,
            frame.viewport,
            EDITOR_TEXT_SCALE,
        ) {
            Ok(plan) => plan,
            Err(error) => {
                self.model = self.model.with_error(error);
                self.update_title();
                return;
            }
        };
        let chrome_plan = match FramePlan::from_ui_frame(&chrome, &self.assets, EDITOR_TEXT_SCALE) {
            Ok(plan) => plan,
            Err(error) => {
                self.model = self.model.with_error(error);
                self.update_title();
                return;
            }
        };
        let plan = FramePlan::compose(map_plan, chrome_plan);
        self.chrome = Some(chrome);
        let (Some(window), Some(runtime)) = (&self.window, &mut self.runtime) else {
            return;
        };
        let result = runtime.present(&plan);
        match result {
            Ok(PresentOutcome::Reconfigured | PresentOutcome::SurfaceLost) => {
                runtime.resize(runtime.surface_size());
                window.request_redraw();
            }
            Ok(_) => {}
            Err(error) => {
                eprintln!("map editor presentation failed: {error}");
                event_loop.exit();
            }
        }
    }

    fn dispatch(&mut self, intent: EditorIntent) {
        match self.model.reduce(intent) {
            Ok((model, EditorEffect::None)) => self.model = model,
            Ok((model, EditorEffect::SaveRequested)) => {
                self.model = model;
                self.save();
            }
            Err(error) => self.model = self.model.with_error(error),
        }
        self.update_title();
        self.request_redraw();
    }

    fn save(&mut self) {
        let diagnostics = self.semantics.lint(&self.model.project);
        if let Some(diagnostic) = diagnostics.first() {
            self.model = self.model.with_error(format!(
                "地图语义校验失败（共 {} 项）：{diagnostic:?}",
                diagnostics.len()
            ));
            return;
        }
        let known = self
            .model
            .atomic_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let result = self
            .model
            .project
            .to_json_pretty(&known)
            .map_err(|error| Box::new(error) as Box<dyn Error>)
            .and_then(|json| {
                if let Some(parent) = self.project_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&self.project_path, json)?;
                Ok(())
            });
        match result {
            Ok(()) => self.model = self.model.saved(),
            Err(error) => self.model = self.model.with_error(error),
        }
    }

    fn handle_key(&mut self, event: winit::event::KeyEvent) {
        let key = normalize_key_event(WinitKeyEventSnapshot::new(
            event.physical_key,
            event.logical_key,
            self.modifiers,
            event.state,
            event.repeat,
        ));
        if let Some(intent) = key_intent(
            &key,
            self.model.selected_atomic,
            self.model.atomic_ids.len(),
        ) {
            self.dispatch(intent);
        }
    }

    fn handle_cursor(&mut self, position: winit::dpi::PhysicalPosition<f64>) {
        self.cursor = Some(position);
        let controller = mem::take(&mut self.controller);
        let (controller, intent) = controller.move_cursor(
            position.x,
            position.y,
            self.viewport,
            self.map_viewport(),
            &self.model,
        );
        self.controller = controller;
        if let Some(intent) = intent {
            self.dispatch(intent);
        } else {
            self.request_redraw();
        }
    }

    fn handle_mouse(&mut self, state: ElementState, button: MouseButton) {
        let button = match button {
            MouseButton::Left => PointerButton::Primary,
            MouseButton::Right => PointerButton::Secondary,
            _ => return,
        };
        match state {
            ElementState::Pressed => {
                if button == PointerButton::Primary {
                    if let (Some(position), Some(chrome)) = (self.cursor, &self.chrome) {
                        if position.x >= 0.0 && position.y >= 0.0 {
                            if let Some(id) = chrome.hit_test(position.x as u32, position.y as u32)
                            {
                                self.controller = mem::take(&mut self.controller).release(button);
                                if let Some(intent) = intent_for_ui_hit(&self.model, id) {
                                    self.dispatch(intent);
                                }
                                return;
                            }
                        }
                    }
                }
                let controller = mem::take(&mut self.controller);
                let (controller, intent) =
                    controller.press(button, self.map_viewport(), &self.model);
                self.controller = controller;
                if let Some(intent) = intent {
                    self.dispatch(intent);
                }
            }
            ElementState::Released => {
                self.controller = mem::take(&mut self.controller).release(button)
            }
        }
    }

    fn handle_wheel(&mut self, delta: MouseScrollDelta) {
        let direction = match delta {
            MouseScrollDelta::LineDelta(_, y) => y.signum(),
            MouseScrollDelta::PixelDelta(position) => position.y.signum() as f32,
        };
        if self.modifiers.control_key() {
            self.adjust_map_zoom(direction);
            return;
        }
        if let Some(intent) = wheel_intent(
            direction,
            self.model.selected_atomic,
            self.model.atomic_ids.len(),
        ) {
            self.dispatch(intent);
        }
    }

    fn map_viewport(&self) -> map_editor_core::EditorMapViewport {
        centered_map_viewport(&self.model, self.map_tile_span)
    }

    fn adjust_map_zoom(&mut self, direction: f32) {
        let next = if direction > 0.0 {
            (self.map_tile_span + 1).min(MAX_MAP_TILE_SPAN)
        } else if direction < 0.0 {
            self.map_tile_span.saturating_sub(1).max(MIN_MAP_TILE_SPAN)
        } else {
            self.map_tile_span
        };
        if next == self.map_tile_span {
            return;
        }
        self.map_tile_span = next;
        self.update_title();
        if let Some(cursor) = self.cursor {
            self.handle_cursor(cursor);
        } else {
            self.request_redraw();
        }
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if let Some(runtime) = &mut self.runtime {
            runtime.resize(pixel_size(size));
        }
        self.viewport = editor_viewport(pixel_size(size));
        self.request_redraw();
    }

    fn update_title(&self) {
        if let Some(window) = &self.window {
            let dirty = if self.model.dirty { " *" } else { "" };
            window.set_title(&format!(
                "Gen3 地图编辑器 - {} - {}x{}",
                self.model.project.id, self.map_tile_span, dirty
            ));
        }
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl ApplicationHandler for MapEditorApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            if let Err(error) = self.initialize(event_loop) {
                eprintln!("map editor initialization failed: {error}");
                event_loop.exit();
            }
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
            WindowEvent::KeyboardInput { event, .. } => self.handle_key(event),
            WindowEvent::CursorMoved { position, .. } => self.handle_cursor(position),
            WindowEvent::CursorLeft { .. } => {
                self.controller = mem::take(&mut self.controller).leave();
                self.cursor = None;
                self.request_redraw();
            }
            WindowEvent::MouseInput { state, button, .. } => self.handle_mouse(state, button),
            WindowEvent::MouseWheel { delta, .. } => self.handle_wheel(delta),
            _ => {}
        }
    }
}

fn pixel_size(size: PhysicalSize<u32>) -> PixelSize {
    PixelSize::new(size.width, size.height)
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = MapEditorApp::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}
