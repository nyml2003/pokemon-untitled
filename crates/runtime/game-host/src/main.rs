mod map;
mod narrative;
mod sprites;
mod thin_slice;
mod trainer_content;

use std::{
    error::Error,
    mem,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use battle_application::Action;
use game_data::{CurrentDataSet, PokedexData};
use game_foundation::{
    ContentPackage, ContentPackageDocument, GameCommand as FoundationCommand, ItemId, SaveEnvelope,
    ThinSliceContent,
};
use game_native_target::{
    FramePlan, NativeAssets, NativeTarget, PresentOutcome, TextScale, WinitCommittedTextSnapshot,
    WinitKeyEventSnapshot, instance_for_event_loop, normalize_committed_text, normalize_key_event,
};
use game_ramus_adapter::{GameRamusRouter, RoutedIntent};
use game_scene_view::{SceneFrame, SceneViewInput, game_viewport, project_scene};
use game_session::{
    GameCommand, GameError, GameEvents, GameScene, GameSession, ProductCommand, ProductSession,
};
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
    product: Option<ProductSession>,
    foundation_content: ThinSliceContent,
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
        let package = load_product_content_package()?;
        let foundation_content = package.content().clone();
        let product = ProductSession::from_package(CurrentDataSet::embedded()?, package)
            .map_err(|error| std::io::Error::other(format!("product session: {error:?}")))?;
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
            // 旧演示会话只用于启动期资源清单，不能与产品会话并存为运行中业务状态。
            game: None,
            product: Some(product),
            foundation_content,
            foundation_router,
            foundation_page: Some(FoundationPage::Journey),
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

    fn product(&self) -> Option<&ProductSession> {
        self.product.as_ref()
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

    fn advance_product_battle(&mut self) {
        let command = {
            let Some(product) = self.product() else {
                eprintln!("product battle rejected: product session is unavailable");
                return;
            };
            let snapshot = product.snapshot();
            let Some(battle) = snapshot.battle() else {
                eprintln!("product battle rejected: no active battle");
                return;
            };
            if battle.is_finished() {
                ProductCommand::LeaveFinishedBattle
            } else {
                let actions = product.legal_player_actions();
                if actions.is_empty() {
                    ProductCommand::AdvanceBattlePlayback
                } else {
                    let Some(action) = actions
                        .iter()
                        .copied()
                        .find(|action| matches!(action, Action::UseMove(_)))
                        .or_else(|| actions.first().copied())
                    else {
                        eprintln!("product battle rejected: no legal action");
                        return;
                    };
                    ProductCommand::SubmitBattleAction(action)
                }
            }
        };
        if let Err(error) = self.submit_product(command) {
            eprintln!("{error}");
        }
    }

    fn save_product(&mut self) -> Result<(), String> {
        let product = self
            .product()
            .ok_or_else(|| String::from("product session is unavailable"))?;
        let save = product
            .save()
            .map_err(|error| format!("product save: {error:?}"))?;
        let bytes = save
            .to_json()
            .map_err(|error| format!("save encode: {error:?}"))?;
        let loaded = SaveEnvelope::from_json(&self.foundation_content, &bytes)
            .map_err(|error| format!("save reload: {error:?}"))?;
        let data = CurrentDataSet::embedded().map_err(|error| format!("data reload: {error:?}"))?;
        let product = ProductSession::from_save(data, self.foundation_content.clone(), loaded)
            .map_err(|error| format!("product reload: {error:?}"))?;
        write_foundation_save(&bytes)?;
        self.product = Some(product);
        Ok(())
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
        let instance = instance_for_event_loop(event_loop);
        let runtime =
            NativeTarget::new(&instance, window.clone(), size, &self.assets, CLEAR_COLOR)?;
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
            let Some(product) = self.product() else {
                event_loop.exit();
                return;
            };
            let snapshot = product.snapshot();
            let tree = match project_foundation(&self.foundation_content, snapshot.state(), page) {
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
            self.foundation_page = Some(FoundationPage::Journey);
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
            FoundationPageAction::Close => self.foundation_page = Some(FoundationPage::Journey),
            FoundationPageAction::Move(direction) => {
                self.route_foundation_source(&format!(
                    "/game/world move direction={}",
                    foundation_direction(direction)
                ));
            }
            FoundationPageAction::Interact => {
                if let Err(error) = self.submit_product(ProductCommand::InteractFront) {
                    eprintln!("{error}");
                }
            }
            FoundationPageAction::Encounter => {
                self.route_foundation_source("/game/world encounter roll=7");
            }
            FoundationPageAction::ResolveBattle => self.advance_product_battle(),
            FoundationPageAction::BuyPotion => {
                let item = match ItemId::new("potion") {
                    Ok(item) => item,
                    Err(error) => {
                        eprintln!("foundation purchase rejected: {error:?}");
                        self.request_redraw();
                        return;
                    }
                };
                if let Err(error) =
                    self.submit_product(ProductCommand::BuyFromFront { item, quantity: 1 })
                {
                    eprintln!("{error}");
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
                RoutedIntent::Command(command) => match product_command(command) {
                    Ok(command) => {
                        if let Err(error) = self.submit_product(command) {
                            eprintln!("{error}");
                            return;
                        }
                    }
                    Err(error) => {
                        eprintln!("foundation command rejected: {error}");
                        return;
                    }
                },
                RoutedIntent::Save => {
                    if let Err(error) = self.save_product() {
                        eprintln!("foundation save rejected: {error}");
                        return;
                    }
                }
            }
        }
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
        self.foundation_page.is_none()
            && !self.presentation.is_console_open()
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
        if self.foundation_page.is_some() {
            event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
            return;
        }
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

fn product_command(command: FoundationCommand) -> Result<ProductCommand, &'static str> {
    match command {
        FoundationCommand::NewGame => Ok(ProductCommand::NewGame),
        FoundationCommand::Interact { npc } => Ok(ProductCommand::Interact(npc)),
        FoundationCommand::Move { direction } => Ok(ProductCommand::Move(direction)),
        FoundationCommand::Warp { warp } => Ok(ProductCommand::Warp(warp)),
        FoundationCommand::Encounter { roll } => Ok(ProductCommand::BeginEncounter { roll }),
        FoundationCommand::Buy {
            npc,
            item,
            quantity,
        } => Ok(ProductCommand::Buy {
            npc,
            item,
            quantity,
        }),
        FoundationCommand::ResolveBattle { .. } => {
            Err("summary battle resolution is unavailable in the product session")
        }
    }
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
