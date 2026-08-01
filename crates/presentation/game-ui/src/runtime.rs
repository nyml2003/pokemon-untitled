use std::time::Duration;

use game_data::CurrentDataSet;
use game_session::{DebugTeamPreset, GameCommand, GameError, GameSession, GameSnapshot};
use punctum_gpu::PixelSize;
use punctum_input::{KeyEvent, TextEvent};
use world_application::WorldApplication;

use crate::{
    GameConsole, PresentationAction, PresentationSnapshot, PresentationState, PresentationUpdate,
};

/// Owns the game session and the presentation state that interprets player input.
pub struct GameRuntime {
    game: Option<GameSession>,
    presentation: PresentationState,
    console: GameConsole,
}

/// Describes host-level work requested by the game runtime.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GameRuntimeUpdate {
    /// Requests a new native frame after input or animation changed the visible scene.
    pub redraw: bool,
    /// Reports a command-console IME state change to the native host.
    pub ime_changed: bool,
}

enum Submission {
    Accepted(GameRuntimeUpdate),
    Rejected,
}

impl GameRuntime {
    /// Creates a world runtime with its input and animation state colocated.
    pub fn new(
        data: CurrentDataSet,
        world: WorldApplication,
        roster_seed: u64,
    ) -> Result<Self, GameError> {
        Ok(Self {
            game: Some(GameSession::new(data, world, roster_seed)?),
            presentation: PresentationState::default(),
            console: GameConsole::default(),
        })
    }

    /// 使用调试预置队伍创建运行时。
    pub fn new_with_debug_preset(
        data: CurrentDataSet,
        world: WorldApplication,
        preset: &'static DebugTeamPreset,
    ) -> Result<Self, GameError> {
        Ok(Self {
            game: Some(GameSession::new(data, world, 0)?.with_debug_preset(preset)),
            presentation: PresentationState::default(),
            console: GameConsole::default(),
        })
    }

    /// Returns the current game snapshot when the runtime owns a session.
    pub fn snapshot(&self) -> Option<GameSnapshot> {
        self.game.as_ref().map(GameSession::snapshot)
    }

    /// Returns the sprite requirements for the current session when it is available.
    pub fn sprite_manifest(&self) -> Option<Result<game_session::DemoSpriteManifest, GameError>> {
        self.game.as_ref().map(GameSession::sprite_manifest)
    }

    /// Synchronizes and returns the presentation snapshot for the supplied map-cell size.
    pub fn presentation_snapshot(&mut self, cell_size: PixelSize) -> Option<PresentationSnapshot> {
        let snapshot = self.snapshot()?;
        let presentation = std::mem::take(&mut self.presentation);
        let (presentation, snapshot) = presentation.snapshot(&snapshot, cell_size);
        self.presentation = presentation;
        Some(snapshot)
    }

    /// Returns the next animation deadline, if the current presentation needs one.
    pub fn next_delay(&self) -> Option<Duration> {
        self.snapshot()
            .and_then(|snapshot| self.presentation.next_delay(&snapshot))
    }

    /// Interprets one normalized input event and submits any resulting game action.
    pub fn handle_key(
        &mut self,
        key: &KeyEvent,
        text: Option<&TextEvent>,
        running: bool,
    ) -> GameRuntimeUpdate {
        let Some(snapshot) = self.snapshot() else {
            return GameRuntimeUpdate::default();
        };
        let Some(game) = self.game.as_ref() else {
            return GameRuntimeUpdate::default();
        };
        let entries = self.console.entries(&game.legal_player_actions());
        let presentation = std::mem::take(&mut self.presentation);
        let (presentation, update) =
            presentation.handle_key(key, text, running, &snapshot, entries);
        self.presentation = presentation;
        self.apply_presentation_update(update)
    }

    /// Advances presentation timing and submits any action released by a completed animation.
    pub fn advance(&mut self, elapsed: Duration) -> GameRuntimeUpdate {
        let Some(snapshot) = self.snapshot() else {
            return GameRuntimeUpdate::default();
        };
        let presentation = std::mem::take(&mut self.presentation);
        let (presentation, update) = presentation.advance(elapsed, &snapshot);
        self.presentation = presentation;
        self.apply_presentation_update(update)
    }

    /// Clears held controls when the native window loses keyboard focus.
    pub fn focus_lost(&mut self) -> GameRuntimeUpdate {
        let presentation = std::mem::take(&mut self.presentation);
        let (presentation, update) = presentation.focus_lost();
        self.presentation = presentation;
        self.apply_presentation_update(update)
    }

    fn apply_presentation_update(&mut self, update: PresentationUpdate) -> GameRuntimeUpdate {
        let mut result = GameRuntimeUpdate {
            redraw: update.redraw,
            ime_changed: update.ime_changed,
        };
        if let Some(action) = update.action {
            let action_update = self.dispatch_presentation_action(action);
            result.redraw |= action_update.redraw;
            result.ime_changed |= action_update.ime_changed;
        }
        result
    }

    fn dispatch_presentation_action(&mut self, action: PresentationAction) -> GameRuntimeUpdate {
        match action {
            PresentationAction::Submit(command) => match self.submit(command) {
                Submission::Accepted(update) => update,
                Submission::Rejected => GameRuntimeUpdate::default(),
            },
            PresentationAction::ExecuteConsole(invocation) => {
                match self.console.execute(&invocation) {
                    Ok(action) => {
                        let Submission::Accepted(mut result) =
                            self.submit(GameCommand::SubmitBattleAction(action))
                        else {
                            return GameRuntimeUpdate::default();
                        };
                        let presentation = std::mem::take(&mut self.presentation);
                        let (presentation, update) = presentation.console_execution_succeeded();
                        self.presentation = presentation;
                        result.redraw |= update.redraw;
                        result.ime_changed |= update.ime_changed;
                        result
                    }
                    Err(error) => {
                        let presentation = std::mem::take(&mut self.presentation);
                        let (presentation, _) = presentation.console_execution_failed(error);
                        self.presentation = presentation;
                        GameRuntimeUpdate::default()
                    }
                }
            }
        }
    }

    fn submit(&mut self, command: GameCommand) -> Submission {
        let Some(game) = self.game.take() else {
            return Submission::Rejected;
        };
        let (game, result) = game.transition(command);
        self.game = Some(game);
        match result {
            Ok(events) => {
                self.presentation =
                    std::mem::take(&mut self.presentation).observe_game_events(&events);
                Submission::Accepted(GameRuntimeUpdate {
                    redraw: true,
                    ime_changed: false,
                })
            }
            Err(_) => {
                self.presentation = std::mem::take(&mut self.presentation).reject_action();
                Submission::Rejected
            }
        }
    }
}
