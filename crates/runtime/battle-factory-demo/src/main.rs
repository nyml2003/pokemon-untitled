//! 对战工厂模式的独立原生 demo 壳。
//!
//! 该二进制打开窗口、加载战斗精灵资源，并把键盘输入转换为 `FactoryCommand`。
//! 对战阶段复用 `game-view::project_battle_ui` 与 `BattleUiState` 的输入语义；
//! 工厂菜单（开始、交换、结果）使用简单的像素 UI 屏幕。
//! 它不读写存档，也不把业务状态暴露给渲染层。

#![forbid(unsafe_code)]

use std::{
    error::Error,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use battle_factory::{FactoryCommand, FactoryEvent, FactoryPhase, FactorySession, FactorySnapshot};
use battle_session::BattleInteraction;
use game_asset_plan::{assemble_assets, battle_asset_requests};
use game_data::CurrentDataSet;
use game_fs_assets::{load_catalog, read_asset_requests};
use game_native_target::{
    FramePlan, NativeAssets, NativeTarget, PresentOutcome, TextScale, WinitCommittedTextSnapshot,
    instance_for_event_loop, normalize_committed_text, normalize_key_event,
};
use game_ui::{BattleUiOutcome, BattleUiState, GameControl};
use game_view::{BattleSpriteResources, project_battle_ui};
use punctum_gpu::{PixelSize, Rgba8};
use punctum_input::{KeyEvent, KeyPhase};
use punctum_ui::UiSize;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::ModifiersState,
    window::{Window, WindowId},
};

const CLEAR_COLOR: Rgba8 = Rgba8::new(14, 22, 32, 255);
const TEXT_SCALE: TextScale = TextScale::new(3, 5, 10, 28);
const BATTLE_PLAYBACK_INTERVAL: Duration = Duration::from_millis(600);
const BATTLE_FRAME_INTERVAL: Duration = Duration::from_millis(300);
const DEFAULT_TARGET_STREAK: u32 = 7;

/// 工厂菜单中用于交换选择的游标位置。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct SwapCursor {
    rental_slot: usize,
    opponent_slot: usize,
}

struct FactoryDemoApp {
    session: Option<FactorySession>,
    battle_ui: BattleUiState,
    assets: NativeAssets,
    status: Option<String>,
    modifiers: ModifiersState,
    window: Option<Arc<Window>>,
    runtime: Option<NativeTarget<'static>>,
    battle_sprite_remaining: Option<Duration>,
    battle_playback_remaining: Option<Duration>,
    sprite_frame: usize,
    last_ui_instant: Instant,
    swap_cursor: SwapCursor,
    loaded_manifest: Option<Vec<u32>>,
}

impl FactoryDemoApp {
    fn new(seed: u64, target_streak: u32) -> Result<Self, Box<dyn Error>> {
        let data = CurrentDataSet::embedded()?;
        let session = FactorySession::new(data);
        let (session, result) = session.transition(FactoryCommand::StartRun {
            seed,
            target_streak,
        });
        result.map_err(|error| std::io::Error::other(format!("factory start: {error:?}")))?;
        let assets = load_factory_assets(&session)?;
        Ok(Self {
            session: Some(session),
            battle_ui: BattleUiState::default(),
            assets,
            status: None,
            modifiers: ModifiersState::empty(),
            window: None,
            runtime: None,
            battle_sprite_remaining: None,
            battle_playback_remaining: None,
            sprite_frame: 0,
            last_ui_instant: Instant::now(),
            swap_cursor: SwapCursor::default(),
            loaded_manifest: None,
        })
    }

    fn initialize(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn Error>> {
        let window = Arc::new(
            event_loop.create_window(
                Window::default_attributes()
                    .with_title("对战工厂 Demo")
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

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        self.advance(Instant::now());
        let Some(surface_size) = self.runtime.as_ref().map(NativeTarget::surface_size) else {
            return;
        };
        let Some(snapshot) = self.session.as_ref().map(FactorySession::snapshot) else {
            event_loop.exit();
            return;
        };
        let tree = match snapshot.phase() {
            FactoryPhase::Battle => self.battle_tree(&snapshot),
            _ => factory_screen_tree(&snapshot, self.swap_cursor),
        };
        let tree = match tree {
            Ok(tree) => tree,
            Err(error) => {
                eprintln!("factory tree construction failed: {error}");
                event_loop.exit();
                return;
            }
        };
        let frame = match tree.resolve(UiSize::new(surface_size.width, surface_size.height)) {
            Ok(frame) => frame,
            Err(error) => {
                eprintln!("factory layout failed: {error}");
                event_loop.exit();
                return;
            }
        };
        let plan = match FramePlan::from_ui_frame(&frame, &self.assets, TEXT_SCALE) {
            Ok(plan) => plan,
            Err(error) => {
                eprintln!("factory GPU planning failed: {error}");
                event_loop.exit();
                return;
            }
        };
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
                eprintln!("factory presentation failed: {error}");
                event_loop.exit();
            }
        }
    }

    fn battle_tree(&self, snapshot: &FactorySnapshot) -> Result<punctum_ui::UiTree, String> {
        let battle = snapshot
            .battle()
            .ok_or_else(|| String::from("battle snapshot missing"))?;
        let sprites = BattleSpriteResources::for_slots(
            snapshot.own_sprite_slot(),
            snapshot.opponent_sprite_slot(),
        );
        project_battle_ui(battle, self.battle_ui, sprites, self.sprite_frame)
            .map_err(|error| format!("battle UI: {error}"))
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if let Some(runtime) = &mut self.runtime {
            runtime.resize(pixel_size(size));
        }
        self.request_redraw();
    }

    fn handle_key(&mut self, event: winit::event::KeyEvent) {
        let _ = normalize_committed_text(WinitCommittedTextSnapshot::new(
            event.text.map(|text| text.to_string()),
        ));
        let key = normalize_key_event(game_native_target::WinitKeyEventSnapshot::new(
            event.physical_key,
            event.logical_key,
            self.modifiers,
            event.state,
            event.repeat,
        ));
        if key.phase == KeyPhase::Release {
            return;
        }
        let Some(session) = self.session.as_ref() else {
            return;
        };
        let phase = session.snapshot().phase();
        match phase {
            FactoryPhase::Battle => self.handle_battle_key(&key),
            FactoryPhase::Ready => self.handle_ready_key(&key),
            FactoryPhase::SwapOffer => self.handle_swap_key(&key),
            FactoryPhase::Finished => self.handle_finished_key(&key),
        }
    }

    fn handle_battle_key(&mut self, key: &KeyEvent) {
        let Some(session) = self.session.as_ref() else {
            return;
        };
        let snapshot = session.snapshot();
        let Some(battle) = snapshot.battle() else {
            return;
        };
        if matches!(battle.interaction(), BattleInteraction::Finished(_))
            && GameControl::from_key_event(key) == Some(GameControl::A)
        {
            self.submit(FactoryCommand::LeaveFinishedBattle);
            return;
        }
        let (battle_ui, outcome) = self.battle_ui.handle_key(key, battle.interaction());
        self.battle_ui = battle_ui;
        match outcome {
            BattleUiOutcome::Updated => self.request_redraw(),
            BattleUiOutcome::Submit(action) => {
                self.submit(FactoryCommand::SubmitBattleAction(action))
            }
            BattleUiOutcome::Ignored => {}
        }
    }

    fn handle_ready_key(&mut self, key: &KeyEvent) {
        if GameControl::from_key_event(key) == Some(GameControl::A) {
            self.submit(FactoryCommand::StartNextBattle);
        }
    }

    fn handle_swap_key(&mut self, key: &KeyEvent) {
        let Some(session) = self.session.as_ref() else {
            return;
        };
        let snapshot = session.snapshot();
        let rental_count = snapshot.rental().len();
        let opponent_count = snapshot
            .opponent()
            .map_or(0, <[battle_factory::FactoryMember]>::len);
        if rental_count == 0 || opponent_count == 0 {
            return;
        }
        match GameControl::from_key_event(key) {
            Some(GameControl::Up) => {
                self.swap_cursor.rental_slot =
                    (self.swap_cursor.rental_slot + rental_count - 1) % rental_count;
                self.request_redraw();
            }
            Some(GameControl::Down) => {
                self.swap_cursor.rental_slot = (self.swap_cursor.rental_slot + 1) % rental_count;
                self.request_redraw();
            }
            Some(GameControl::Left) => {
                self.swap_cursor.opponent_slot =
                    (self.swap_cursor.opponent_slot + opponent_count - 1) % opponent_count;
                self.request_redraw();
            }
            Some(GameControl::Right) => {
                self.swap_cursor.opponent_slot =
                    (self.swap_cursor.opponent_slot + 1) % opponent_count;
                self.request_redraw();
            }
            Some(GameControl::A) => {
                self.submit(FactoryCommand::ConfirmSwap {
                    rental_slot: self.swap_cursor.rental_slot,
                    opponent_slot: self.swap_cursor.opponent_slot,
                });
            }
            Some(GameControl::B) => {
                self.submit(FactoryCommand::SkipSwap);
            }
            _ => {}
        }
    }

    fn handle_finished_key(&mut self, key: &KeyEvent) {
        if GameControl::from_key_event(key) == Some(GameControl::A) {
            self.submit(FactoryCommand::StartRun {
                seed: random_seed(),
                target_streak: DEFAULT_TARGET_STREAK,
            });
        }
    }

    fn submit(&mut self, command: FactoryCommand) {
        let Some(session) = self.session.take() else {
            return;
        };
        let (session, result) = session.transition(command);
        self.session = Some(session);
        match result {
            Ok(events) => {
                self.observe_events(&events);
                self.refresh_assets();
            }
            Err(error) => {
                self.status = Some(format!("命令被拒绝：{error:?}"));
                self.request_redraw();
            }
        }
    }

    fn observe_events(&mut self, events: &battle_factory::FactoryEvents) {
        for event in events.iter() {
            match event {
                FactoryEvent::BattleStarted => {
                    self.battle_sprite_remaining = Some(BATTLE_FRAME_INTERVAL);
                    self.sprite_frame = 0;
                    self.battle_ui = BattleUiState::default();
                }
                FactoryEvent::BattleActionSubmitted => {
                    self.battle_playback_remaining = Some(BATTLE_PLAYBACK_INTERVAL);
                    self.battle_sprite_remaining
                        .get_or_insert(BATTLE_FRAME_INTERVAL);
                }
                FactoryEvent::BattlePlaybackAdvanced { remains } => {
                    self.battle_playback_remaining = remains.then_some(BATTLE_PLAYBACK_INTERVAL);
                }
                FactoryEvent::RunStarted => {
                    self.battle_playback_remaining = None;
                    self.battle_sprite_remaining = None;
                    self.sprite_frame = 0;
                    self.battle_ui = BattleUiState::default();
                    self.swap_cursor = SwapCursor::default();
                    self.status = None;
                }
                FactoryEvent::BattleResolved { .. }
                | FactoryEvent::SwapApplied { .. }
                | FactoryEvent::RunEnded { .. } => {
                    self.battle_playback_remaining = None;
                    self.battle_sprite_remaining = None;
                    self.sprite_frame = 0;
                    self.battle_ui = BattleUiState::default();
                    self.swap_cursor = SwapCursor::default();
                }
            }
        }
        self.request_redraw();
    }

    fn refresh_assets(&mut self) {
        let Some(session) = self.session.as_ref() else {
            return;
        };
        let Ok(Some(manifest)) = session.sprite_manifest() else {
            return;
        };
        let signature = manifest
            .player()
            .iter()
            .chain(manifest.opponent())
            .map(|form| form.0)
            .collect::<Vec<_>>();
        if self.loaded_manifest.as_ref() == Some(&signature) {
            return;
        }
        let Ok(assets) = load_factory_assets(session) else {
            return;
        };
        self.assets = assets;
        self.loaded_manifest = Some(signature);
        if let Some(runtime) = &mut self.runtime
            && let Err(error) = runtime.update_assets(&self.assets)
        {
            self.status = Some(format!("精灵图集刷新失败：{error}"));
        }
        self.request_redraw();
    }

    fn advance(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.last_ui_instant);
        self.last_ui_instant = now;
        if take_elapsed(&mut self.battle_playback_remaining, elapsed) {
            self.submit(FactoryCommand::AdvanceBattlePlayback);
        }
        if advance_periodic(
            &mut self.battle_sprite_remaining,
            elapsed,
            BATTLE_FRAME_INTERVAL,
        ) {
            self.sprite_frame = self.sprite_frame.wrapping_add(1);
            self.request_redraw();
        }
    }

    fn next_delay(&self) -> Option<Duration> {
        self.battle_playback_remaining
            .into_iter()
            .chain(self.battle_sprite_remaining)
            .min()
    }
}

/// 用当前快照加载战斗精灵资源；无对手时只加载 UI 蒙版与属性图标。
fn load_factory_assets(session: &FactorySession) -> Result<NativeAssets, Box<dyn Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../assets");
    let catalog = load_catalog(&root)?;
    let requests = session
        .sprite_manifest()
        .map_err(|error| std::io::Error::other(format!("factory manifest: {error:?}")))?
        .map_or_else(Vec::new, |manifest| battle_asset_requests(&manifest));
    let sources = read_asset_requests(&root, &catalog, requests)?;
    Ok(assemble_assets(sources, Vec::new())?)
}

fn pixel_size(size: PhysicalSize<u32>) -> PixelSize {
    PixelSize::new(size.width, size.height)
}

fn random_seed() -> u64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    (now ^ u128::from(std::process::id())) as u64
}

fn take_elapsed(timer: &mut Option<Duration>, elapsed: Duration) -> bool {
    let Some(remaining) = *timer else {
        return false;
    };
    if elapsed >= remaining {
        *timer = None;
        true
    } else {
        *timer = Some(remaining - elapsed);
        false
    }
}

fn advance_periodic(timer: &mut Option<Duration>, elapsed: Duration, interval: Duration) -> bool {
    let Some(mut remaining) = *timer else {
        return false;
    };
    if elapsed < remaining {
        *timer = Some(remaining - elapsed);
        return false;
    }
    let mut excess = elapsed - remaining;
    while excess >= interval {
        excess -= interval;
    }
    remaining = interval - excess;
    *timer = Some(remaining);
    true
}

fn factory_screen_tree(
    snapshot: &FactorySnapshot,
    cursor: SwapCursor,
) -> Result<punctum_ui::UiTree, String> {
    let mut lines = Vec::new();
    lines.push(format!(
        "对战工厂 · 连胜 {}/{}",
        snapshot.streak(),
        snapshot.target_streak()
    ));
    lines.push("".to_owned());
    lines.push("租借队伍：".to_owned());
    for (index, member) in snapshot.rental().iter().enumerate() {
        lines.push(format!(
            "  {}  {:<8} Lv.{}  HP {}/{}",
            if index == cursor.rental_slot && snapshot.phase() == FactoryPhase::SwapOffer {
                "▶"
            } else {
                " "
            },
            member.name(),
            member.level(),
            member.current_hp(),
            member.max_hp(),
        ));
    }
    if let Some(opponent) = snapshot.opponent() {
        lines.push("".to_owned());
        lines.push("对手队伍：".to_owned());
        for (index, member) in opponent.iter().enumerate() {
            lines.push(format!(
                "  {}  {:<8} Lv.{}  HP {}/{}",
                if index == cursor.opponent_slot && snapshot.phase() == FactoryPhase::SwapOffer {
                    "▶"
                } else {
                    " "
                },
                member.name(),
                member.level(),
                member.current_hp(),
                member.max_hp(),
            ));
        }
    }
    lines.push("".to_owned());
    match snapshot.phase() {
        FactoryPhase::Ready => lines.push("按 A 开始下一场对战".to_owned()),
        FactoryPhase::SwapOffer => {
            lines.push("↑↓ 选择己方 · ←→ 选择对手 · A 交换 · B 跳过".to_owned())
        }
        FactoryPhase::Finished if snapshot.streak() >= snapshot.target_streak() => {
            lines.push(format!(
                "清关！连胜 {} 场。按 A 开始新一轮",
                snapshot.streak()
            ));
        }
        FactoryPhase::Finished => {
            lines.push(format!(
                "本轮结束，连胜 {} 场。按 A 开始新一轮",
                snapshot.streak()
            ));
        }
        FactoryPhase::Battle => {}
    }
    build_text_screen(&lines)
}

fn build_text_screen(lines: &[String]) -> Result<punctum_ui::UiTree, String> {
    let mut children = Vec::new();
    for line in lines {
        let color = if line.starts_with('▶') {
            punctum_ui::UiColor::new(73, 211, 168, 255)
        } else {
            punctum_ui::UiColor::new(244, 246, 239, 255)
        };
        children.push(
            punctum_ui::UiNode::auto()
                .with_style(punctum_ui::UiStyle {
                    width: punctum_ui::Dimension::Fill,
                    height: punctum_ui::Dimension::Px(30),
                    ..punctum_ui::UiStyle::default()
                })
                .with_content(punctum_ui::UiContent::Text {
                    content: line.clone(),
                    color,
                    font_size: 20,
                }),
        );
    }
    let root = punctum_ui::UiNode::auto()
        .with_style(punctum_ui::UiStyle {
            width: punctum_ui::Dimension::Fill,
            height: punctum_ui::Dimension::Fill,
            direction: punctum_ui::FlexDirection::Column,
            padding: punctum_ui::Insets::all(24),
            gap: 6,
            ..punctum_ui::UiStyle::default()
        })
        .with_content(punctum_ui::UiContent::Fill(CLEAR_COLOR.into_ui()))
        .with_children(children);
    punctum_ui::UiTree::new(root).map_err(|error| format!("text screen: {error}"))
}

impl ApplicationHandler for FactoryDemoApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none()
            && let Err(error) = self.initialize(event_loop)
        {
            eprintln!("factory demo initialization failed: {error}");
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
                self.battle_ui = BattleUiState::default();
                self.request_redraw();
            }
            WindowEvent::KeyboardInput { event, .. } => self.handle_key(event),
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                self.request_redraw();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        self.advance(now);
        if let Some(delay) = self.next_delay() {
            self.request_redraw();
            event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(now + delay));
        } else {
            event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
        }
    }
}

trait IntoUi {
    fn into_ui(self) -> punctum_ui::UiColor;
}

impl IntoUi for Rgba8 {
    fn into_ui(self) -> punctum_ui::UiColor {
        punctum_ui::UiColor::new(self.red, self.green, self.blue, self.alpha)
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new().map_err(|error| {
        std::io::Error::other(format!(
            "窗口系统初始化失败：{error}（WSLg 显示会话不可用时请重启 WSL）"
        ))
    })?;
    let mut app = FactoryDemoApp::new(random_seed(), DEFAULT_TARGET_STREAK)?;
    event_loop.run_app(&mut app)?;
    Ok(())
}
