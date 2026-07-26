//! 不包含运行时地图渲染的玩家页面 demo 原生壳。
//!
//! 该二进制将键盘、鼠标点击和悬停转换为 `PageIntent`，并将纯页面 frame 提交给原生目标。
//! 产品命令和存档请求只显示反馈，不会在此处执行。

#![forbid(unsafe_code)]

use std::{
    error::Error,
    fmt, fs,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

mod metrics;

use game_assets::{AssetKey, DecodedImage, decode_png};
use game_native_target::{
    FramePlan, NativeAssets, NativeTarget, PresentOutcome, TextScale, WinitCommittedTextSnapshot,
    instance_for_event_loop, normalize_committed_text,
};
use game_page_model::{
    PageDemo, PageDemoContext, PageEffect, PageIntent, PageState, demo_named, page_demos,
    project_demo_page,
};
use game_ui::{PageUiOutcome, PageUiState};
use game_view::{
    page_party_pokemon_asset, page_pokedex_icon_asset, page_pokedex_pokemon_asset,
    page_world_player_asset, page_world_tile_asset, project_page_model_with_visual_state,
};
use punctum_gpu::{PixelSize, Rgba8};
use punctum_input::{KeyEvent, KeyPhase, LogicalKey, NamedKey};
use punctum_ui::{UiFrame, UiInteraction, UiInteractionTarget, UiSize};
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::ModifiersState,
    window::{Window, WindowId},
};

use metrics::{FrameMetrics, FrameSample, PerfReport};

const CLEAR_COLOR: Rgba8 = Rgba8::new(14, 22, 32, 255);
const TEXT_SCALE: TextScale = TextScale::new(1, 1, 14, 28);
const DEFAULT_DEMO: &str = "world-starting-town";

struct PageDemoApp {
    demo: PageDemo,
    context: PageDemoContext,
    state: PageState,
    page_ui: PageUiState,
    status: Option<String>,
    assets: NativeAssets,
    frame: Option<UiFrame<PageIntent>>,
    interaction: UiInteraction,
    interaction_targets: Vec<UiInteractionTarget>,
    cursor: Option<PhysicalPosition<f64>>,
    modifiers: ModifiersState,
    metrics: FrameMetrics,
    last_ui_instant: Instant,
    window: Option<Arc<Window>>,
    runtime: Option<NativeTarget<'static>>,
}

impl PageDemoApp {
    fn new(demo: PageDemo) -> Result<Self, Box<dyn Error>> {
        let now = Instant::now();
        let context = demo.context()?;
        let state = demo.initial_state()?;
        let assets = load_page_demo_assets()?;
        Ok(Self {
            demo,
            context,
            state,
            page_ui: PageUiState::default(),
            status: None,
            assets,
            frame: None,
            interaction: UiInteraction::default(),
            interaction_targets: Vec::new(),
            cursor: None,
            modifiers: ModifiersState::empty(),
            metrics: FrameMetrics::new(now),
            last_ui_instant: now,
            window: None,
            runtime: None,
        })
    }

    fn initialize(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn Error>> {
        let window = Arc::new(
            event_loop.create_window(
                Window::default_attributes()
                    .with_title(self.title())
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

    fn title(&self) -> String {
        match &self.status {
            Some(status) => format!("玩家页面 Demo · {} · {status}", self.demo.id().as_str()),
            None => format!("玩家页面 Demo · {}", self.demo.id().as_str()),
        }
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn update_title(&self) {
        if let Some(window) = &self.window {
            window.set_title(&self.title());
        }
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let frame_started = Instant::now();
        let advance_started = Instant::now();
        self.advance_ui(Instant::now());
        let advance = advance_started.elapsed();
        let Some(surface_size) = self.runtime.as_ref().map(NativeTarget::surface_size) else {
            return;
        };
        let model_started = Instant::now();
        let model = match project_demo_page(&self.context, self.state.route()) {
            Ok(model) => model,
            Err(error) => {
                eprintln!("page demo model construction failed: {error}");
                event_loop.exit();
                return;
            }
        };
        let model_time = model_started.elapsed();
        self.page_ui.sync(&model);
        let tree_started = Instant::now();
        let tree = match project_page_model_with_visual_state(
            &model,
            self.status.as_deref(),
            Some(self.page_ui.pokedex_visual_state()),
            UiSize::new(surface_size.width, surface_size.height),
        ) {
            Ok(tree) => tree,
            Err(error) => {
                eprintln!("page demo tree construction failed: {error}");
                event_loop.exit();
                return;
            }
        };
        let tree_time = tree_started.elapsed();
        let layout_started = Instant::now();
        let frame = match tree.resolve(UiSize::new(surface_size.width, surface_size.height)) {
            Ok(frame) => frame,
            Err(error) => {
                eprintln!("page demo layout failed: {error}");
                event_loop.exit();
                return;
            }
        };
        let layout_time = layout_started.elapsed();
        self.interaction_targets = frame.interaction_targets().to_vec();
        self.interaction.reconcile(&self.interaction_targets);
        self.sync_keyboard_focus(&model, &frame);
        self.sync_pointer();
        let frame = frame.with_interaction(self.interaction.snapshot());
        let commands = frame.commands().len();
        let action_hits = frame.action_hits().len();
        let interaction_targets = frame.interaction_targets().len();
        let plan_started = Instant::now();
        let plan = match FramePlan::from_ui_frame(&frame, &self.assets, TEXT_SCALE) {
            Ok(plan) => plan,
            Err(error) => {
                eprintln!("page demo GPU planning failed: {error}");
                event_loop.exit();
                return;
            }
        };
        let plan_time = plan_started.elapsed();
        let instances = plan
            .passes()
            .iter()
            .map(|pass| u64::from(pass.gpu().instance_count))
            .fold(0_u64, u64::saturating_add);
        self.frame = Some(frame);
        let (Some(window), Some(runtime)) = (&self.window, &mut self.runtime) else {
            return;
        };
        let present_started = Instant::now();
        let present_result = runtime.present(&plan);
        let present_time = present_started.elapsed();
        let outcome = match present_result {
            Ok(outcome @ (PresentOutcome::Reconfigured | PresentOutcome::SurfaceLost)) => {
                runtime.resize(runtime.surface_size());
                window.request_redraw();
                Some(outcome)
            }
            Ok(outcome) => Some(outcome),
            Err(error) => {
                eprintln!("page demo presentation failed: {error}");
                event_loop.exit();
                None
            }
        };
        if let Some(outcome) = outcome {
            let report = self.metrics.record(
                Instant::now(),
                FrameSample {
                    total: frame_started.elapsed(),
                    advance,
                    model: model_time,
                    tree: tree_time,
                    layout: layout_time,
                    plan: plan_time,
                    present: present_time,
                    commands,
                    action_hits,
                    interaction_targets,
                    instances,
                    outcome,
                },
            );
            if let Some(report) = report {
                eprintln!("[page-demo perf] {report}");
            }
        }
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if let Some(runtime) = &mut self.runtime {
            runtime.resize(pixel_size(size));
        }
        self.request_redraw();
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

    fn sync_keyboard_focus(
        &mut self,
        model: &game_page_model::PageModel,
        frame: &UiFrame<PageIntent>,
    ) {
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

    fn press(&mut self) {
        let Some(cursor) = self.cursor else {
            return;
        };
        let Some((x, y)) = cursor_position(cursor) else {
            return;
        };
        self.interaction.press(&self.interaction_targets, x, y);
        self.request_redraw();
    }

    fn release(&mut self) {
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
            .frame
            .as_ref()
            .and_then(|frame| frame.action_hit_by_id(id))
            .map(|hit| hit.action.clone())
        else {
            self.request_redraw();
            return;
        };
        self.dispatch_intent(intent);
    }

    fn handle_key(&mut self, event: winit::event::KeyEvent) {
        let text = match normalize_committed_text(WinitCommittedTextSnapshot::new(
            event.text.map(|text| text.to_string()),
        )) {
            Ok(text) => text,
            Err(error) => {
                self.status = Some(format!("文本输入不可用：{error}"));
                self.update_title();
                self.request_redraw();
                return;
            }
        };
        let key = game_native_target::normalize_key_event(
            game_native_target::WinitKeyEventSnapshot::new(
                event.physical_key,
                event.logical_key,
                self.modifiers,
                event.state,
                event.repeat,
            ),
        );
        if let Some(step) = demo_switch_step(&key) {
            self.switch_demo(step);
            return;
        }
        let model = match project_demo_page(&self.context, self.state.route()) {
            Ok(model) => model,
            Err(error) => {
                self.status = Some(format!("页面状态不可用：{error}"));
                self.request_redraw();
                return;
            }
        };
        match self.page_ui.handle_input(&key, text.as_ref(), &model) {
            PageUiOutcome::Intent(intent) => self.dispatch_intent(intent),
            PageUiOutcome::Updated => self.request_redraw(),
            PageUiOutcome::Ignored => {}
        }
    }

    fn switch_demo(&mut self, step: isize) {
        let demos = page_demos();
        let current = demos
            .iter()
            .position(|demo| demo.id() == self.demo.id())
            .unwrap_or(0);
        let next = if step < 0 {
            current.checked_sub(1).unwrap_or(demos.len() - 1)
        } else {
            (current + 1) % demos.len()
        };
        let demo = demos[next];
        let context = match demo.context() {
            Ok(context) => context,
            Err(error) => {
                self.status = Some(format!("页面素材不可用：{error}"));
                self.update_title();
                self.request_redraw();
                return;
            }
        };
        let state = match demo.initial_state() {
            Ok(state) => state,
            Err(error) => {
                self.status = Some(format!("页面状态不可用：{error}"));
                self.update_title();
                self.request_redraw();
                return;
            }
        };
        self.demo = demo;
        self.context = context;
        self.state = state;
        self.page_ui = PageUiState::default();
        self.status = None;
        self.update_title();
        self.request_redraw();
    }

    fn dispatch_intent(&mut self, intent: PageIntent) {
        if let Ok(model) = project_demo_page(&self.context, self.state.route())
            && let Some(outcome) = self.page_ui.handle_view_intent(&intent, &model)
        {
            match outcome {
                PageUiOutcome::Intent(intent) => self.dispatch_intent(intent),
                PageUiOutcome::Updated | PageUiOutcome::Ignored => self.request_redraw(),
            }
            return;
        }
        match self.state.clone().transition(intent.clone()) {
            Ok((state, effect)) => {
                self.state = state;
                if let Ok(model) = project_demo_page(&self.context, self.state.route()) {
                    self.page_ui.focus_intent(&intent, &model);
                }
                self.status = effect.map(effect_status);
            }
            Err(error) => self.status = Some(format!("操作未执行：{error}")),
        }
        self.update_title();
        self.request_redraw();
    }

    fn advance_ui(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.last_ui_instant);
        self.last_ui_instant = now;
        if self.page_ui.advance(elapsed) || self.interaction.advance(elapsed) {
            self.request_redraw();
        }
    }
}

fn load_page_demo_assets() -> Result<NativeAssets, Box<dyn Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../assets");
    let treecko_party = page_party_pokemon_asset("Treecko").ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Treecko page asset slot is not registered",
        )
    })?;
    let mut sources = vec![
        (
            page_world_player_asset(),
            String::from("source/character/red/down/stand/00.png"),
        ),
        (
            treecko_party,
            String::from("source/pokemon/0263/form/00/normal/front/00.png"),
        ),
    ];
    for tile in [8_u16, 9, 10, 11, 12, 13] {
        sources.push((
            page_world_tile_asset(tile),
            format!("source/map/tile/{tile:04}.png"),
        ));
    }
    for number in 1_u16..=386 {
        let Some(key) = page_pokedex_pokemon_asset(number) else {
            continue;
        };
        sources.push((
            key,
            format!("source/pokemon/{number:04}/form/00/normal/front/00.png"),
        ));
        let Some(key) = page_pokedex_icon_asset(number) else {
            continue;
        };
        sources.push((
            key,
            format!("source/pokemon/{number:04}/form/00/icon/00.png"),
        ));
    }
    for type_name in [
        "bug", "dark", "dragon", "electric", "fighting", "fire", "flying", "ghost", "grass",
        "ground", "ice", "normal", "poison", "psychic", "rock", "steel", "water",
    ] {
        let key = AssetKey::from_resource_template(format!("ui/battle/type/{type_name}"));
        sources.push((key, format!("source/ui/battle/type/{type_name}.png")));
    }
    for category in ["physical", "special", "status"] {
        let key = AssetKey::from_resource_template(format!("ui/battle/move-category/{category}"));
        sources.push((
            key,
            format!("source/ui/battle/move-category/{category}.png"),
        ));
    }
    let mut images = vec![(
        AssetKey::from_resource_template("solid/white".into()),
        DecodedImage::solid(Rgba8::new(255, 255, 255, 255)),
    )];
    for (key, relative_path) in sources {
        let path = root.join(relative_path);
        let bytes = fs::read(&path).map_err(|error| {
            std::io::Error::new(error.kind(), format!("{}: {error}", path.display()))
        })?;
        let image = decode_png(&bytes).map_err(|error| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("{}: {error}", path.display()),
            )
        })?;
        images.push((key, image));
    }
    Ok(NativeAssets::new(images)?)
}

impl ApplicationHandler for PageDemoApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none()
            && let Err(error) = self.initialize(event_loop)
        {
            eprintln!("page demo initialization failed: {error}");
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
            } => self.press(),
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => self.release(),
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::KeyboardInput { event, .. } => self.handle_key(event),
            WindowEvent::Focused(false) => {
                self.interaction.clear_transient();
                self.request_redraw();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        self.advance_ui(now);
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

fn effect_status(effect: PageEffect) -> String {
    match effect {
        PageEffect::SubmitProduct(_) => String::from("产品命令已发出；demo 不会修改产品状态"),
        PageEffect::RequestSave => String::from("存档请求已发出；demo 不会写入文件"),
    }
}

fn demo_switch_step(key: &KeyEvent) -> Option<isize> {
    if key.phase != KeyPhase::Press {
        return None;
    }
    match key.logical {
        LogicalKey::Named(NamedKey::PageUp) => Some(-1),
        LogicalKey::Named(NamedKey::PageDown) => Some(1),
        _ => None,
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

fn pixel_size(size: PhysicalSize<u32>) -> PixelSize {
    PixelSize::new(size.width, size.height)
}

#[derive(Debug)]
struct DemoSelectionError(String);

impl fmt::Display for DemoSelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for DemoSelectionError {}

fn selected_demo(
    arguments: impl IntoIterator<Item = String>,
) -> Result<PageDemo, DemoSelectionError> {
    let mut arguments = arguments.into_iter();
    let Some(first) = arguments.next() else {
        return demo_named(DEFAULT_DEMO).ok_or_else(|| {
            DemoSelectionError(String::from("default page demo is not registered"))
        });
    };
    if first != "--page-demo" {
        return Err(DemoSelectionError(format!(
            "unknown argument {first}; use --page-demo <PageDemoId>"
        )));
    }
    let Some(name) = arguments.next() else {
        return Err(DemoSelectionError(String::from(
            "--page-demo requires a registered PageDemoId",
        )));
    };
    if arguments.next().is_some() {
        return Err(DemoSelectionError(String::from(
            "only one --page-demo argument is supported",
        )));
    }
    demo_named(&name).ok_or_else(|| DemoSelectionError(format!("unknown page demo {name}")))
}

fn main() -> Result<(), Box<dyn Error>> {
    let demo = selected_demo(std::env::args().skip(1))?;
    let event_loop = EventLoop::new()?;
    let mut app = PageDemoApp::new(demo)?;
    let result = event_loop.run_app(&mut app);
    if let Some(report) = app.metrics.finish(Instant::now()) {
        print_final_report(report);
    }
    result?;
    Ok(())
}

fn print_final_report(report: PerfReport) {
    let (dominant_stage, dominant_ms) = report.dominant_stage();
    eprintln!("[page-demo perf final]");
    eprintln!(
        "  runtime: {:.2}s, frames: {}, fps: {:.1}",
        report.elapsed_s, report.frames, report.fps
    );
    eprintln!(
        "  frame: average {:.2}ms, max {:.2}ms",
        report.frame_ms, report.max_frame_ms
    );
    eprintln!(
        "  stages: advance {:.2}ms, model {:.2}ms, tree {:.2}ms, layout {:.2}ms, plan {:.2}ms, present {:.2}ms",
        report.advance_ms,
        report.model_ms,
        report.tree_ms,
        report.layout_ms,
        report.plan_ms,
        report.present_ms,
    );
    eprintln!(
        "  dominant stage: {dominant_stage} ({dominant_ms:.2}ms average); last frame: commands={}, action_hits={}, targets={}, instances={}, outcome={:?}",
        report.commands,
        report.action_hits,
        report.interaction_targets,
        report.instances,
        report.outcome,
    );
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_DEMO, selected_demo};

    #[test]
    fn selects_default_and_registered_demos() -> Result<(), Box<dyn std::error::Error>> {
        let default = selected_demo(Vec::new())?;
        assert_eq!(default.id().as_str(), DEFAULT_DEMO);
        let selected = selected_demo(vec![
            String::from("--page-demo"),
            String::from("shop-potion-preview"),
        ])?;
        assert_eq!(selected.id().as_str(), "shop-potion-preview");
        let world_down = selected_demo(vec![
            String::from("--page-demo"),
            String::from("world-starting-down"),
        ])?;
        assert_eq!(world_down.id().as_str(), "world-starting-down");
        Ok(())
    }

    #[test]
    fn rejects_unknown_page_demo() {
        assert!(
            selected_demo(vec![String::from("--page-demo"), String::from("unknown"),]).is_err()
        );
    }
}
