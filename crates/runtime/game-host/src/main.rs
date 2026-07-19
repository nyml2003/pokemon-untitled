mod map;
mod narrative;
mod sprites;
mod thin_slice;

use std::{
    error::Error,
    mem,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use game_data::{CurrentDataSet, PokedexData};
use game_foundation::{GameState as FoundationState, ThinSliceContent};
use game_native_target::{
    FramePlan, NativeAssets, NativeTarget, PresentOutcome, TextScale, WinitCommittedTextSnapshot,
    WinitKeyEventSnapshot, normalize_committed_text, normalize_key_event,
};
use game_ramus_adapter::{GameRamusRouter, RoutedIntent};
use game_scene_view::{SceneFrame, SceneViewInput, game_viewport, project_scene};
use game_session::{GameCommand, GameError, GameEvents, GameScene, GameSession};
use game_ui::{
    GameConsole, PokedexAction, PresentationAction, PresentationState, PresentationUpdate,
};
use game_view::{FoundationPage, FoundationPageAction, project_foundation};
use map::load_map;
use map_project::MapProject;
use map_render::AtomicTileCatalog;
use narrative::load_narrative_scripts;
use punctum_gpu::{PixelSize, Rgba8};
use punctum_input::{KeyPhase, LogicalKey, NamedKey};
use punctum_ui::{UiFrame, UiSize};
use sprites::load_game_assets;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalPosition, PhysicalSize},
    event::{ElementState, Ime, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::ModifiersState,
    window::{Window, WindowId},
};

const CLEAR_COLOR: Rgba8 = Rgba8::new(14, 18, 24, 255);
const GAME_TEXT_SCALE: TextScale = TextScale::new(3, 5, 10, 28);
const WORLD_LOGIC_TICK: Duration = Duration::from_secs(1);
const FOUNDATION_SAVE_PATH: &str = "target/foundation-page.save.json";

struct CreatureGameApp {
    game: Option<GameSession>,
    foundation_content: ThinSliceContent,
    foundation_state: FoundationState,
    foundation_router: GameRamusRouter,
    foundation_page: Option<FoundationPage>,
    pokedex: PokedexData,
    presentation: PresentationState,
    map_project: MapProject,
    map_catalog: AtomicTileCatalog,
    console: GameConsole,
    assets: NativeAssets,
    modifiers: ModifiersState,
    cursor: Option<PhysicalPosition<f64>>,
    pokedex_frame: Option<UiFrame<PokedexAction>>,
    foundation_frame: Option<UiFrame<FoundationPageAction>>,
    last_real_instant: Instant,
    next_world_tick: Instant,
    next_wakeup: Option<Instant>,
    window: Option<Arc<Window>>,
    runtime: Option<NativeTarget<'static>>,
}

impl CreatureGameApp {
    fn new() -> Result<Self, Box<dyn Error>> {
        let foundation_content = ThinSliceContent::standard()
            .map_err(|error| std::io::Error::other(format!("foundation content: {error:?}")))?;
        let foundation_state = FoundationState::new(&foundation_content)
            .map_err(|error| std::io::Error::other(format!("foundation state: {error:?}")))?;
        let foundation_router = GameRamusRouter::new().map_err(|error| {
            std::io::Error::other(format!("foundation Ramus router: {error:?}"))
        })?;
        let loaded_map = load_map()?;
        let world = world_application::WorldApplication::from_map_project_with_scripts(
            &loaded_map.project,
            load_narrative_scripts()?,
        )
        .map_err(|error| std::io::Error::other(format!("map world: {error:?}")))?;
        let game = GameSession::new(CurrentDataSet::embedded()?, world, random_roster_seed())
            .map_err(|error| std::io::Error::other(format!("demo game: {error:?}")))?;
        let sprite_manifest = game
            .sprite_manifest()
            .map_err(|error| std::io::Error::other(format!("demo sprite manifest: {error:?}")))?;
        let pokedex = PokedexData::embedded_gen3()?;
        let snapshot = game.snapshot();
        let assets = load_game_assets(
            &sprite_manifest,
            &pokedex,
            snapshot.world(),
            loaded_map.images,
        )?;
        let now = Instant::now();
        Ok(Self {
            game: Some(game),
            foundation_content,
            foundation_state,
            foundation_router,
            foundation_page: None,
            pokedex,
            presentation: PresentationState::default(),
            map_project: loaded_map.project,
            map_catalog: loaded_map.catalog,
            console: GameConsole::default(),
            assets,
            modifiers: ModifiersState::empty(),
            cursor: None,
            pokedex_frame: None,
            foundation_frame: None,
            last_real_instant: now,
            next_world_tick: now + WORLD_LOGIC_TICK,
            next_wakeup: None,
            window: None,
            runtime: None,
        })
    }

    fn game(&self) -> Option<&GameSession> {
        self.game.as_ref()
    }

    fn submit_game(&mut self, command: GameCommand) -> Result<GameEvents, GameError> {
        let game = self.game.take().ok_or(GameError::BattleStateMissing)?;
        let (game, result) = game.transition(command);
        self.game = Some(game);
        result
    }

    fn initialize(&mut self, event_loop: &ActiveEventLoop) -> Result<(), Box<dyn Error>> {
        let window = Arc::new(
            event_loop.create_window(
                Window::default_attributes()
                    .with_title("宝可梦：还没想好名字")
                    .with_inner_size(LogicalSize::new(960.0, 720.0)),
            )?,
        );
        let size = pixel_size(window.inner_size());
        let runtime = NativeTarget::new(window.clone(), size, &self.assets, CLEAR_COLOR)?;
        window.set_ime_allowed(false);
        window.request_redraw();
        self.window = Some(window);
        self.runtime = Some(runtime);
        Ok(())
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        self.advance_presentation(Instant::now());
        let Some(surface_size) = self.runtime.as_ref().map(NativeTarget::surface_size) else {
            return;
        };
        let viewport = game_viewport(surface_size);
        if let Some(page) = self.foundation_page {
            let tree =
                match project_foundation(&self.foundation_content, &self.foundation_state, page) {
                    Ok(tree) => tree,
                    Err(error) => {
                        eprintln!("foundation page tree construction failed: {error}");
                        event_loop.exit();
                        return;
                    }
                };
            let frame = match tree.resolve(UiSize::new(
                viewport.target_size.width,
                viewport.target_size.height,
            )) {
                Ok(frame) => frame,
                Err(error) => {
                    eprintln!("foundation page layout failed: {error}");
                    event_loop.exit();
                    return;
                }
            };
            self.foundation_frame = Some(frame.clone());
            let plan = match FramePlan::from_ui_frame(
                &frame,
                &self.assets,
                TextScale::new(1, 1, 16, 28),
            ) {
                Ok(plan) => plan,
                Err(error) => {
                    eprintln!("foundation page GPU planning failed: {error}");
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
                Ok(
                    PresentOutcome::Presented
                    | PresentOutcome::PresentedAndReconfigured
                    | PresentOutcome::SkippedMinimized
                    | PresentOutcome::SkippedTimeout
                    | PresentOutcome::SkippedOccluded,
                ) => {}
                Err(error) => {
                    eprintln!("foundation page presentation failed: {error}");
                    event_loop.exit();
                }
            }
            return;
        }
        let Some(game) = self.game() else {
            event_loop.exit();
            return;
        };
        let game_snapshot = game.snapshot();
        let state = mem::take(&mut self.presentation);
        let (state, presentation) = state.snapshot(&game_snapshot, viewport.cell_size);
        self.presentation = state;
        let console = self
            .presentation
            .is_console_open()
            .then(|| self.presentation.console_view());
        let projected = match project_scene(SceneViewInput {
            game: &game_snapshot,
            presentation,
            console: console.as_ref(),
            pokedex: &self.pokedex,
            map_project: &self.map_project,
            map_catalog: &self.map_catalog,
            viewport,
        }) {
            Ok(projected) => projected,
            Err(error) => {
                eprintln!("game scene projection failed: {error}");
                event_loop.exit();
                return;
            }
        };
        let (Some(window), Some(runtime)) = (&self.window, &mut self.runtime) else {
            return;
        };
        self.pokedex_frame = match &projected.frame {
            SceneFrame::Pokedex(frame) => Some(frame.clone()),
            SceneFrame::PokedexWithUi { base, .. } => Some(base.clone()),
            _ => None,
        };
        self.foundation_frame = None;
        let plan_result = match projected.frame {
            SceneFrame::Grid(view) => {
                FramePlan::from_game_view(&view, &self.assets, viewport, GAME_TEXT_SCALE)
            }
            SceneFrame::Ui(frame) => {
                FramePlan::from_ui_frame(&frame, &self.assets, TextScale::new(1, 1, 16, 28))
            }
            SceneFrame::Pokedex(frame) => {
                FramePlan::from_ui_frame(&frame, &self.assets, TextScale::new(1, 1, 16, 28))
            }
            SceneFrame::GridWithUi { base, overlay } => FramePlan::from_game_view(
                &base,
                &self.assets,
                viewport,
                GAME_TEXT_SCALE,
            )
            .and_then(|base| {
                FramePlan::from_ui_frame(&overlay, &self.assets, TextScale::new(1, 1, 16, 28))
                    .map(|overlay| FramePlan::compose(base, overlay))
            }),
            SceneFrame::UiWithUi { base, overlay } => {
                FramePlan::from_ui_frame(&base, &self.assets, TextScale::new(1, 1, 16, 28))
                    .and_then(|base| {
                        FramePlan::from_ui_frame(
                            &overlay,
                            &self.assets,
                            TextScale::new(1, 1, 16, 28),
                        )
                        .map(|overlay| FramePlan::compose(base, overlay))
                    })
            }
            SceneFrame::PokedexWithUi { base, overlay } => {
                FramePlan::from_ui_frame(&base, &self.assets, TextScale::new(1, 1, 16, 28))
                    .and_then(|base| {
                        FramePlan::from_ui_frame(
                            &overlay,
                            &self.assets,
                            TextScale::new(1, 1, 16, 28),
                        )
                        .map(|overlay| FramePlan::compose(base, overlay))
                    })
            }
        };
        let plan = match plan_result {
            Ok(plan) => plan,
            Err(error) => {
                eprintln!("game GPU planning failed: {error}");
                event_loop.exit();
                return;
            }
        };
        let result = runtime.present(&plan);
        match result {
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
                eprintln!("game presentation failed: {error}");
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

    fn handle_key(&mut self, event: winit::event::KeyEvent) {
        self.advance_presentation(Instant::now());
        let text = match normalize_committed_text(WinitCommittedTextSnapshot::new(
            event.text.map(|text| text.to_string()),
        )) {
            Ok(text) => text,
            Err(error) => {
                eprintln!("ignored invalid committed text: {error}");
                None
            }
        };
        let key = normalize_key_event(WinitKeyEventSnapshot::new(
            event.physical_key,
            event.logical_key,
            self.modifiers,
            event.state,
            event.repeat,
        ));
        if key.phase == KeyPhase::Press
            && matches!(&key.logical, LogicalKey::Character(value) if value.eq_ignore_ascii_case("f"))
        {
            self.foundation_page = if self.foundation_page.is_some() {
                None
            } else {
                Some(FoundationPage::Journey)
            };
            self.request_redraw();
            return;
        }
        if self.foundation_page.is_some() {
            self.handle_foundation_key(&key);
            return;
        }
        let Some(game) = self.game() else {
            return;
        };
        let snapshot = game.snapshot();
        let entries = self.console.entries(&game.legal_player_actions());
        let presentation = mem::take(&mut self.presentation);
        let (presentation, update) = presentation.handle_key(
            &key,
            text.as_ref(),
            self.modifiers.shift_key(),
            &snapshot,
            entries,
        );
        self.presentation = presentation;
        self.apply_presentation_update(update);
    }

    fn handle_pokedex_click(&mut self) {
        let Some(cursor) = self.cursor else {
            return;
        };
        let Some(action) = self
            .pokedex_frame
            .as_ref()
            .and_then(|frame| frame.hit_action(cursor.x.max(0.0) as u32, cursor.y.max(0.0) as u32))
            .copied()
        else {
            return;
        };
        let presentation = mem::take(&mut self.presentation);
        let (presentation, update) = presentation.handle_pokedex_action(action);
        self.presentation = presentation;
        self.apply_presentation_update(update);
    }

    fn handle_foundation_click(&mut self) {
        let Some(cursor) = self.cursor else {
            return;
        };
        let Some(action) = self
            .foundation_frame
            .as_ref()
            .and_then(|frame| frame.hit_action(cursor.x.max(0.0) as u32, cursor.y.max(0.0) as u32))
            .copied()
        else {
            return;
        };
        self.dispatch_foundation_action(action);
    }

    fn handle_foundation_key(&mut self, key: &punctum_input::KeyEvent) {
        if key.phase != KeyPhase::Press {
            return;
        }
        let action = match &key.logical {
            LogicalKey::Named(NamedKey::Escape) => Some(FoundationPageAction::Close),
            LogicalKey::Named(NamedKey::ArrowUp) => {
                Some(FoundationPageAction::Move(game_foundation::Direction::Up))
            }
            LogicalKey::Named(NamedKey::ArrowDown) => {
                Some(FoundationPageAction::Move(game_foundation::Direction::Down))
            }
            LogicalKey::Named(NamedKey::ArrowLeft) => {
                Some(FoundationPageAction::Move(game_foundation::Direction::Left))
            }
            LogicalKey::Named(NamedKey::ArrowRight) => Some(FoundationPageAction::Move(
                game_foundation::Direction::Right,
            )),
            LogicalKey::Named(NamedKey::Enter) => Some(FoundationPageAction::Interact),
            LogicalKey::Character(value) if value.eq_ignore_ascii_case("e") => {
                Some(FoundationPageAction::Encounter)
            }
            LogicalKey::Character(value) if value.eq_ignore_ascii_case("r") => {
                Some(FoundationPageAction::ResolveBattle)
            }
            LogicalKey::Character(value) if value.eq_ignore_ascii_case("b") => {
                Some(FoundationPageAction::BuyPotion)
            }
            LogicalKey::Character(value) if value.eq_ignore_ascii_case("s") => {
                Some(FoundationPageAction::Save)
            }
            _ => None,
        };
        if let Some(action) = action {
            self.dispatch_foundation_action(action);
        }
    }

    fn dispatch_foundation_action(&mut self, action: FoundationPageAction) {
        match action {
            FoundationPageAction::SelectPage(page) => self.foundation_page = Some(page),
            FoundationPageAction::Close => self.foundation_page = None,
            FoundationPageAction::Move(direction) => {
                self.route_foundation_source(&format!(
                    "/game/world move direction={}",
                    foundation_direction(direction)
                ));
            }
            FoundationPageAction::Interact => {
                if let Some(npc) = self.foundation_npc_in_front() {
                    self.route_foundation_source(&format!("/game/npc interact npc={npc}"));
                } else {
                    eprintln!("foundation interaction rejected: no NPC in front of the player");
                }
            }
            FoundationPageAction::Encounter => {
                self.route_foundation_source("/game/world encounter roll=7");
            }
            FoundationPageAction::ResolveBattle => {
                let Some(creature) = self.foundation_state.party().first() else {
                    eprintln!("foundation battle resolution rejected: party is empty");
                    self.request_redraw();
                    return;
                };
                let hp = creature.hp().saturating_sub(1).max(1);
                let pp = creature.pp().saturating_sub(1);
                self.route_foundation_source(&format!(
                    "/game/battle resolve outcome=victory hp={hp} pp={pp}"
                ));
            }
            FoundationPageAction::BuyPotion => {
                if let Some(npc) = self.foundation_npc_in_front() {
                    self.route_foundation_source(&format!(
                        "/game/shop buy npc={npc} item=potion quantity=1"
                    ));
                } else {
                    eprintln!("foundation purchase rejected: no merchant in front of the player");
                }
            }
            FoundationPageAction::Save => self.route_foundation_source("/game/save save"),
        }
        self.request_redraw();
    }

    fn route_foundation_source(&mut self, source: &str) {
        let intents = match self.foundation_router.route(source) {
            Ok(intents) => intents,
            Err(error) => {
                eprintln!(
                    "foundation Ramus intent rejected: {}: {}",
                    error.code, error.message
                );
                return;
            }
        };
        for intent in intents {
            match intent {
                RoutedIntent::Command(command) => {
                    let (state, result) = self
                        .foundation_state
                        .clone()
                        .transition(&self.foundation_content, command);
                    match result {
                        Ok(_) => self.foundation_state = state,
                        Err(error) => {
                            eprintln!("foundation command rejected: {error:?}");
                            return;
                        }
                    }
                }
                RoutedIntent::Save => match thin_slice::save_and_reload(
                    &self.foundation_content,
                    self.foundation_state.clone(),
                    std::path::Path::new(FOUNDATION_SAVE_PATH),
                ) {
                    Ok(state) => self.foundation_state = state,
                    Err(error) => {
                        eprintln!("foundation save rejected: {error}");
                        return;
                    }
                },
            }
        }
    }

    fn foundation_npc_in_front(&self) -> Option<String> {
        let position = self.foundation_state.position();
        let facing = self.foundation_state.facing();
        self.foundation_content
            .npcs_on_map(self.foundation_state.map())
            .find(|npc| {
                let npc_position = npc.actor().position();
                match facing {
                    game_foundation::Direction::Up => {
                        npc_position.x() == position.x()
                            && npc_position.y().checked_add(1) == Some(position.y())
                    }
                    game_foundation::Direction::Down => {
                        npc_position.x() == position.x()
                            && position.y().checked_add(1) == Some(npc_position.y())
                    }
                    game_foundation::Direction::Left => {
                        npc_position.y() == position.y()
                            && npc_position.x().checked_add(1) == Some(position.x())
                    }
                    game_foundation::Direction::Right => {
                        npc_position.y() == position.y()
                            && position.x().checked_add(1) == Some(npc_position.x())
                    }
                }
            })
            .map(|npc| npc.actor().id().as_str().to_owned())
    }

    fn apply_presentation_update(&mut self, update: PresentationUpdate) {
        if let Some(action) = update.action {
            self.dispatch_presentation_action(action);
        }
        if update.ime_changed {
            self.sync_ime_allowed();
        }
        if update.redraw {
            self.request_redraw();
        }
    }

    fn dispatch_presentation_action(&mut self, action: PresentationAction) {
        match action {
            PresentationAction::Submit(command) => match self.submit_game(command) {
                Ok(events) => {
                    self.presentation =
                        mem::take(&mut self.presentation).observe_game_events(&events)
                }
                Err(error) => {
                    self.presentation = mem::take(&mut self.presentation).reject_action();
                    eprintln!("game command rejected: {error:?}");
                }
            },
            PresentationAction::ExecuteConsole(invocation) => {
                let result = self.console.execute(&invocation).and_then(|action| {
                    self.submit_game(GameCommand::SubmitBattleAction(action))
                        .map_err(|error| format!("战斗 action 被拒绝: {error:?}"))
                });
                match result {
                    Ok(events) => {
                        self.presentation =
                            mem::take(&mut self.presentation).observe_game_events(&events);
                        let presentation = mem::take(&mut self.presentation);
                        let (presentation, update) = presentation.console_execution_succeeded();
                        self.presentation = presentation;
                        if update.ime_changed {
                            self.sync_ime_allowed();
                        }
                    }
                    Err(error) => {
                        let presentation = mem::take(&mut self.presentation);
                        (self.presentation, _) = presentation.console_execution_failed(error);
                    }
                }
            }
        }
        self.request_redraw();
    }

    fn advance_presentation(&mut self, now: Instant) {
        let elapsed = now.saturating_duration_since(self.last_real_instant);
        self.last_real_instant = now;
        let Some(game) = self.game() else {
            return;
        };
        let snapshot = game.snapshot();
        let presentation = mem::take(&mut self.presentation);
        let (presentation, update) = presentation.advance(elapsed, &snapshot);
        self.presentation = presentation;
        self.apply_presentation_update(update);
    }

    fn world_clock_is_active(&self) -> bool {
        !self.presentation.is_console_open()
            && self
                .game()
                .is_some_and(|game| game.snapshot().scene() == GameScene::World)
    }

    fn advance_world_clock(&mut self, now: Instant) {
        if !self.world_clock_is_active() {
            self.next_world_tick = now + WORLD_LOGIC_TICK;
            return;
        }
        if now < self.next_world_tick {
            return;
        }

        let Some(game) = self.game.take() else {
            return;
        };
        let (game, result) = game.advance_world_tick();
        self.game = Some(game);
        match result {
            Ok(events) => {
                self.presentation = mem::take(&mut self.presentation).observe_game_events(&events);
                self.request_redraw();
            }
            Err(error) => eprintln!("world clock rejected: {error:?}"),
        }
        self.next_world_tick = now + WORLD_LOGIC_TICK;
    }

    fn handle_ime_event(&mut self, event: Ime) {
        self.advance_presentation(Instant::now());
        let presentation = mem::take(&mut self.presentation);
        let (presentation, update) = match event {
            Ime::Enabled => (presentation, PresentationUpdate::default()),
            Ime::Preedit(text, _) => presentation.handle_preedit(text),
            Ime::Commit(text) => presentation.handle_commit(text),
            Ime::Disabled => presentation.handle_ime_disabled(),
        };
        self.presentation = presentation;
        self.apply_presentation_update(update);
    }

    fn sync_ime_allowed(&self) {
        let allowed = self.presentation.is_console_open();
        if let Some(window) = &self.window {
            window.set_ime_allowed(allowed);
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
            WindowEvent::Resized(size) => self.resize(size),
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(window) = &self.window {
                    self.resize(window.inner_size());
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => self.modifiers = modifiers.state(),
            WindowEvent::Focused(false) => {
                self.advance_presentation(Instant::now());
                let presentation = mem::take(&mut self.presentation);
                let (presentation, update) = presentation.focus_lost();
                self.presentation = presentation;
                self.apply_presentation_update(update);
            }
            WindowEvent::KeyboardInput { event, .. } => self.handle_key(event),
            WindowEvent::CursorMoved { position, .. } => self.cursor = Some(position),
            WindowEvent::CursorLeft { .. } => self.cursor = None,
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if self.foundation_page.is_some() {
                    self.handle_foundation_click();
                } else {
                    self.handle_pokedex_click();
                }
            }
            WindowEvent::Ime(event) => self.handle_ime_event(event),
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        self.advance_presentation(now);
        self.advance_world_clock(now);
        let Some(game) = self.game() else {
            event_loop.exit();
            return;
        };
        let snapshot = game.snapshot();
        let presentation_wakeup = self
            .presentation
            .next_delay(&snapshot)
            .map(|delay| now + delay);
        let world_wakeup = self.world_clock_is_active().then_some(self.next_world_tick);
        self.next_wakeup = match (presentation_wakeup, world_wakeup) {
            (Some(presentation), Some(world)) => Some(presentation.min(world)),
            (Some(deadline), None) | (None, Some(deadline)) => Some(deadline),
            (None, None) => None,
        };
        if let Some(deadline) = self.next_wakeup {
            event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(deadline));
        } else {
            event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
        }
    }
}

fn random_roster_seed() -> u64 {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (elapsed.as_nanos() as u64) ^ u64::from(std::process::id()).rotate_left(17)
}

fn pixel_size(size: PhysicalSize<u32>) -> PixelSize {
    PixelSize::new(size.width, size.height)
}

const fn foundation_direction(direction: game_foundation::Direction) -> &'static str {
    match direction {
        game_foundation::Direction::Up => "up",
        game_foundation::Direction::Down => "down",
        game_foundation::Direction::Left => "left",
        game_foundation::Direction::Right => "right",
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    if let Some(exit) = thin_slice::run_from_arguments(std::env::args_os().skip(1))? {
        return exit;
    }
    let event_loop = EventLoop::new()?;
    let mut app = CreatureGameApp::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/game_host.rs"]
mod tests;
