mod map;
mod sprites;

use std::{
    error::Error,
    mem,
    sync::Arc,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use game_data::{CurrentDataSet, PokedexData};
use game_native_target::{
    FramePlan, NativeAssets, NativeTarget, PresentOutcome, TextScale, WinitCommittedTextSnapshot,
    WinitKeyEventSnapshot, normalize_committed_text, normalize_key_event,
};
use game_scene_view::{SceneViewInput, game_viewport, project_scene};
use game_session::{GameCommand, GameError, GameEvents, GameSession};
use game_ui::{GameConsole, PresentationAction, PresentationState, PresentationUpdate};
use map::load_map;
use map_project::MapProject;
use map_render::AtomicTileCatalog;
use punctum_gpu::{PixelSize, Rgba8};
use sprites::load_game_assets;
use winit::{
    application::ApplicationHandler,
    dpi::{LogicalSize, PhysicalSize},
    event::{Ime, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::ModifiersState,
    window::{Window, WindowId},
};

const CLEAR_COLOR: Rgba8 = Rgba8::new(14, 18, 24, 255);
const GAME_TEXT_SCALE: TextScale = TextScale::new(3, 5, 10, 28);

struct CreatureGameApp {
    game: Option<GameSession>,
    pokedex: PokedexData,
    presentation: PresentationState,
    map_project: MapProject,
    map_catalog: AtomicTileCatalog,
    console: GameConsole,
    assets: NativeAssets,
    modifiers: ModifiersState,
    last_real_instant: Instant,
    next_wakeup: Option<Instant>,
    window: Option<Arc<Window>>,
    runtime: Option<NativeTarget<'static>>,
}

impl CreatureGameApp {
    fn new() -> Result<Self, Box<dyn Error>> {
        let loaded_map = load_map()?;
        let world = world_application::WorldApplication::from_map_project(&loaded_map.project)
            .map_err(|error| std::io::Error::other(format!("map world: {error:?}")))?;
        let game = GameSession::new(CurrentDataSet::embedded()?, world, random_roster_seed())
            .map_err(|error| std::io::Error::other(format!("demo game: {error:?}")))?;
        let sprite_manifest = game
            .sprite_manifest()
            .map_err(|error| std::io::Error::other(format!("demo sprite manifest: {error:?}")))?;
        let pokedex = PokedexData::embedded_gen3()?;
        let assets = load_game_assets(&sprite_manifest, &pokedex, loaded_map.images)?;
        Ok(Self {
            game: Some(game),
            pokedex,
            presentation: PresentationState::default(),
            map_project: loaded_map.project,
            map_catalog: loaded_map.catalog,
            console: GameConsole::default(),
            assets,
            modifiers: ModifiersState::empty(),
            last_real_instant: Instant::now(),
            next_wakeup: None,
            window: None,
            runtime: None,
        })
    }

    fn game(&self) -> &GameSession {
        self.game.as_ref().expect("the host owns one game session")
    }

    fn submit_game(&mut self, command: GameCommand) -> Result<GameEvents, GameError> {
        let game = self.game.take().expect("the host owns one game session");
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
        let game_snapshot = self.game().snapshot();
        let state = mem::take(&mut self.presentation);
        let (state, presentation) = state.snapshot(&game_snapshot, viewport.cell_size);
        self.presentation = state;
        let console = self
            .presentation
            .is_console_open()
            .then(|| self.presentation.console_view());
        let view = match project_scene(SceneViewInput {
            game: &game_snapshot,
            presentation,
            console: console.as_ref(),
            pokedex: &self.pokedex,
            map_project: &self.map_project,
            map_catalog: &self.map_catalog,
            viewport,
        }) {
            Ok(projected) => projected.view,
            Err(error) => {
                eprintln!("game scene projection failed: {error}");
                event_loop.exit();
                return;
            }
        };
        let (Some(window), Some(runtime)) = (&self.window, &mut self.runtime) else {
            return;
        };
        let plan = match FramePlan::from_game_view(&view, &self.assets, viewport, GAME_TEXT_SCALE) {
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
        let snapshot = self.game().snapshot();
        let entries = self.console.entries(&self.game().legal_player_actions());
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
        let snapshot = self.game().snapshot();
        let presentation = mem::take(&mut self.presentation);
        let (presentation, update) = presentation.advance(elapsed, &snapshot);
        self.presentation = presentation;
        self.apply_presentation_update(update);
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
            WindowEvent::Ime(event) => self.handle_ime_event(event),
            WindowEvent::RedrawRequested => self.redraw(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        self.advance_presentation(now);
        let snapshot = self.game().snapshot();
        self.next_wakeup = self
            .presentation
            .next_delay(&snapshot)
            .map(|delay| now + delay);
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

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let mut app = CreatureGameApp::new()?;
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::CreatureGameApp;

    #[test]
    fn complete_game_atlas_fits_wgpu_texture_limits() {
        let app = CreatureGameApp::new().unwrap();
        let size = app.assets.atlas_size();
        assert!(size.width <= 8_192, "atlas width was {}", size.width);
        assert!(size.height <= 8_192, "atlas height was {}", size.height);
    }
}
