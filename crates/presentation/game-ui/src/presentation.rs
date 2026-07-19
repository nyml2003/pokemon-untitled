use std::time::Duration;

use game_session::{GameCommand, GameEvent, GameEvents, GameScene, GameSnapshot};
use punctum_gpu::{PixelOffset, PixelSize};
use punctum_input::{KeyEvent, KeyPhase, LogicalKey, NamedKey, PhysicalKeyCode, TextEvent};
use world_application::{Direction, WorldEvent};

use crate::{
    BattleUiOutcome, BattleUiState, CommandConsoleView, ConsoleEntry, ConsoleIntent,
    ConsoleOutcome, ConsoleState, WorldAnimation,
};

const BATTLE_PLAYBACK_INTERVAL: Duration = Duration::from_millis(600);
const BATTLE_FRAME_INTERVAL: Duration = Duration::from_millis(300);
const WORLD_TICK_INTERVAL: Duration = Duration::from_millis(16);
const TURN_HOLD_DURATION: Duration = Duration::from_millis(90);
const RUN_STOP_DURATION: Duration = Duration::from_millis(90);
const SETTLE_DURATION: Duration = Duration::from_millis(50);
const POKEDEX_ENTRY_COUNT: usize = 386;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PresentationAction {
    Submit(GameCommand),
    ExecuteConsole(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PresentationUpdate {
    pub action: Option<PresentationAction>,
    pub redraw: bool,
    pub ime_changed: bool,
}

impl PresentationUpdate {
    fn redraw() -> Self {
        Self {
            redraw: true,
            ..Self::default()
        }
    }

    fn action(action: PresentationAction) -> Self {
        Self {
            action: Some(action),
            redraw: true,
            ..Self::default()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentationSnapshot {
    pub battle_ui: BattleUiState,
    pub pokedex: Option<PokedexUiSnapshot>,
    pub world_animation: WorldAnimation,
    pub sprite_frame: usize,
    pub world_pixel_offset: PixelOffset,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PokedexUiSnapshot {
    pub selected_index: usize,
}

/// A page-level intent emitted by the Pokedex UI tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PokedexAction {
    SelectEntry { index: usize },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PresentationState {
    battle_ui: BattleUiState,
    pokedex: Option<PokedexUiSnapshot>,
    console: ConsoleState,
    battle_playback_remaining: Option<Duration>,
    battle_sprite_remaining: Option<Duration>,
    sprite_frame: usize,
    pressed_directions: PressedDirections,
    world_motion: Option<WorldMotion>,
    pending_world_step: Option<(Direction, Gait)>,
    turn_hold_remaining: Option<Duration>,
    run_stop_remaining: Option<Duration>,
    running: bool,
}

impl PresentationState {
    pub const fn is_console_open(&self) -> bool {
        self.console.is_open()
    }

    pub fn console_view(&self) -> CommandConsoleView {
        CommandConsoleView {
            query: self.console.query.clone(),
            preedit: self.console.preedit.clone(),
            items: self
                .console
                .items
                .iter()
                .map(|item| item.invocation.clone())
                .collect(),
            selected_index: self.console.selected_index,
            diagnostic: self.console.diagnostic.clone(),
        }
    }

    /// Applies a Pokedex intent without giving the UI access to mutable state.
    pub fn handle_pokedex_action(mut self, action: PokedexAction) -> (Self, PresentationUpdate) {
        let update = self.handle_pokedex_action_mut(action);
        (self, update)
    }

    fn handle_pokedex_action_mut(&mut self, action: PokedexAction) -> PresentationUpdate {
        let Some(pokedex) = &mut self.pokedex else {
            return PresentationUpdate::default();
        };
        match action {
            PokedexAction::SelectEntry { index } => {
                let selected_index = index.min(POKEDEX_ENTRY_COUNT.saturating_sub(1));
                if pokedex.selected_index == selected_index {
                    return PresentationUpdate::default();
                }
                pokedex.selected_index = selected_index;
                PresentationUpdate::redraw()
            }
        }
    }

    pub fn snapshot(
        mut self,
        game: &GameSnapshot,
        cell_size: PixelSize,
    ) -> (Self, PresentationSnapshot) {
        let snapshot = self.snapshot_mut(game, cell_size);
        (self, snapshot)
    }

    fn snapshot_mut(&mut self, game: &GameSnapshot, cell_size: PixelSize) -> PresentationSnapshot {
        self.sync_scene(game);
        if let Some(battle) = game.battle() {
            self.battle_ui = self.battle_ui.synced(battle.session().interaction());
        }
        let (world_animation, sprite_frame, world_pixel_offset) =
            if game.scene() == GameScene::Battle {
                (
                    WorldAnimation::Stand,
                    self.sprite_frame,
                    PixelOffset::new(0, 0),
                )
            } else if let Some(motion) = self.world_motion {
                (
                    motion.gait().animation(),
                    motion.sprite_frame(),
                    motion.pixel_offset(cell_size),
                )
            } else {
                (
                    if self.run_stop_remaining.is_some() {
                        WorldAnimation::RunStopping
                    } else {
                        WorldAnimation::Stand
                    },
                    0,
                    PixelOffset::new(0, 0),
                )
            };
        PresentationSnapshot {
            battle_ui: self.battle_ui,
            pokedex: self.pokedex,
            world_animation,
            sprite_frame,
            world_pixel_offset,
        }
    }

    pub fn handle_key(
        mut self,
        key: &KeyEvent,
        text: Option<&TextEvent>,
        running: bool,
        game: &GameSnapshot,
        console_entries: Vec<ConsoleEntry>,
    ) -> (Self, PresentationUpdate) {
        let update = self.handle_key_mut(key, text, running, game, console_entries);
        (self, update)
    }

    fn handle_key_mut(
        &mut self,
        key: &KeyEvent,
        text: Option<&TextEvent>,
        running: bool,
        game: &GameSnapshot,
        console_entries: Vec<ConsoleEntry>,
    ) -> PresentationUpdate {
        self.running = running;
        self.sync_scene(game);
        if !self.console.preedit.is_empty() && !is_console_toggle(key) {
            return PresentationUpdate::default();
        }
        if is_console_toggle(key) {
            if key.phase != KeyPhase::Press {
                return PresentationUpdate::default();
            }
            let was_open = self.console.is_open();
            let outcome = if was_open {
                self.console.handle(ConsoleIntent::Close)
            } else {
                self.console.handle(ConsoleIntent::Open(console_entries))
            };
            return PresentationUpdate {
                redraw: outcome != ConsoleOutcome::Ignored,
                ime_changed: was_open != self.console.is_open(),
                ..PresentationUpdate::default()
            };
        }
        if self.console.is_open() {
            return self.handle_console_key(key, text);
        }
        if self.pokedex.is_some() {
            if key.phase == KeyPhase::Release {
                return PresentationUpdate::default();
            }
            let selected_index = self
                .pokedex
                .as_ref()
                .expect("the Pokedex remains open while handling its key")
                .selected_index;
            let action = match key.logical {
                LogicalKey::Named(NamedKey::Escape) => {
                    self.pokedex = None;
                    return PresentationUpdate::redraw();
                }
                LogicalKey::Character(ref value) if value.eq_ignore_ascii_case("p") => {
                    self.pokedex = None;
                    return PresentationUpdate::redraw();
                }
                LogicalKey::Named(NamedKey::ArrowUp) | LogicalKey::Named(NamedKey::ArrowLeft) => {
                    PokedexAction::SelectEntry {
                        index: (selected_index + POKEDEX_ENTRY_COUNT - 1) % POKEDEX_ENTRY_COUNT,
                    }
                }
                LogicalKey::Named(NamedKey::ArrowDown)
                | LogicalKey::Named(NamedKey::ArrowRight) => PokedexAction::SelectEntry {
                    index: (selected_index + 1) % POKEDEX_ENTRY_COUNT,
                },
                LogicalKey::Named(NamedKey::PageUp) => PokedexAction::SelectEntry {
                    index: (selected_index + POKEDEX_ENTRY_COUNT - 10) % POKEDEX_ENTRY_COUNT,
                },
                LogicalKey::Named(NamedKey::PageDown) => PokedexAction::SelectEntry {
                    index: (selected_index + 10) % POKEDEX_ENTRY_COUNT,
                },
                LogicalKey::Named(NamedKey::Home) => PokedexAction::SelectEntry { index: 0 },
                LogicalKey::Named(NamedKey::End) => PokedexAction::SelectEntry {
                    index: POKEDEX_ENTRY_COUNT - 1,
                },
                _ => return PresentationUpdate::default(),
            };
            return self.handle_pokedex_action_mut(action);
        }
        if game.scene() == GameScene::World
            && key.phase == KeyPhase::Press
            && matches!(&key.logical, LogicalKey::Character(value) if value.eq_ignore_ascii_case("p"))
        {
            self.pokedex = Some(PokedexUiSnapshot { selected_index: 0 });
            return PresentationUpdate::redraw();
        }
        if game.scene() == GameScene::World
            && let Some(direction) = direction_for_key(key)
        {
            return self.handle_world_direction(direction, key.phase);
        }
        let Some(battle) = game.battle() else {
            return PresentationUpdate::default();
        };
        if matches!(
            battle.session().interaction(),
            battle_session::BattleInteraction::Finished(_)
        ) && is_enter_press(key)
        {
            return PresentationUpdate::action(PresentationAction::Submit(
                GameCommand::LeaveFinishedBattle,
            ));
        }
        let (battle_ui, outcome) = self
            .battle_ui
            .handle_key(key, battle.session().interaction());
        self.battle_ui = battle_ui;
        match outcome {
            BattleUiOutcome::Updated => PresentationUpdate::redraw(),
            BattleUiOutcome::Submit(action) => PresentationUpdate::action(
                PresentationAction::Submit(GameCommand::SubmitBattleAction(action)),
            ),
            BattleUiOutcome::Ignored => PresentationUpdate::default(),
        }
    }

    pub fn handle_preedit(mut self, text: String) -> (Self, PresentationUpdate) {
        let update = self.handle_preedit_mut(text);
        (self, update)
    }

    fn handle_preedit_mut(&mut self, text: String) -> PresentationUpdate {
        if !self.console.is_open() {
            return PresentationUpdate::default();
        }
        self.console.set_preedit(text);
        PresentationUpdate::redraw()
    }

    pub fn handle_commit(mut self, text: String) -> (Self, PresentationUpdate) {
        let update = self.handle_commit_mut(text);
        (self, update)
    }

    fn handle_commit_mut(&mut self, text: String) -> PresentationUpdate {
        if text.is_empty() || !self.console.is_open() {
            return PresentationUpdate::default();
        }
        self.console.set_preedit(String::new());
        let outcome = self.console.handle(ConsoleIntent::InsertText(text));
        PresentationUpdate {
            redraw: outcome == ConsoleOutcome::Updated,
            ..PresentationUpdate::default()
        }
    }

    pub fn handle_ime_disabled(mut self) -> (Self, PresentationUpdate) {
        let update = self.handle_ime_disabled_mut();
        (self, update)
    }

    fn handle_ime_disabled_mut(&mut self) -> PresentationUpdate {
        self.console.set_preedit(String::new());
        PresentationUpdate::redraw()
    }

    pub fn focus_lost(mut self) -> (Self, PresentationUpdate) {
        let update = self.focus_lost_mut();
        (self, update)
    }

    fn focus_lost_mut(&mut self) -> PresentationUpdate {
        self.pressed_directions.clear();
        self.turn_hold_remaining = None;
        if let Some(motion) = &mut self.world_motion {
            motion.settle();
        } else {
            self.run_stop_remaining = None;
        }
        PresentationUpdate::redraw()
    }

    pub fn advance(mut self, elapsed: Duration, game: &GameSnapshot) -> (Self, PresentationUpdate) {
        let update = self.advance_mut(elapsed, game);
        (self, update)
    }

    fn advance_mut(&mut self, elapsed: Duration, game: &GameSnapshot) -> PresentationUpdate {
        self.sync_scene(game);
        if self.console.is_open() {
            return PresentationUpdate::default();
        }
        let mut redraw = false;
        if advance_periodic(
            &mut self.battle_sprite_remaining,
            elapsed,
            BATTLE_FRAME_INTERVAL,
        ) {
            self.sprite_frame = self.sprite_frame.wrapping_add(1);
            redraw = true;
        }
        if take_elapsed(&mut self.battle_playback_remaining, elapsed) {
            return PresentationUpdate {
                action: Some(PresentationAction::Submit(
                    GameCommand::AdvanceBattlePlayback,
                )),
                redraw: true,
                ime_changed: false,
            };
        }
        if take_elapsed(&mut self.turn_hold_remaining, elapsed)
            && let Some(action) = self.start_world_step()
        {
            return PresentationUpdate::action(action);
        }
        let mut started_run_stop = false;
        if let Some(motion) = &mut self.world_motion {
            motion.advance(elapsed);
            redraw = true;
            if motion.is_complete() {
                let gait = motion.gait();
                self.world_motion = None;
                if let Some(action) = self.start_world_step() {
                    return PresentationUpdate::action(action);
                }
                self.run_stop_remaining = (gait == Gait::Run).then_some(RUN_STOP_DURATION);
                started_run_stop = self.run_stop_remaining.is_some();
            }
        }
        if !started_run_stop && take_elapsed(&mut self.run_stop_remaining, elapsed) {
            redraw = true;
        }
        PresentationUpdate {
            redraw,
            ..PresentationUpdate::default()
        }
    }

    pub fn next_delay(&self, game: &GameSnapshot) -> Option<Duration> {
        if self.console.is_open() {
            return None;
        }
        let mut delays = Vec::with_capacity(5);
        if game.scene() == GameScene::Battle {
            delays.extend(self.battle_sprite_remaining);
            delays.extend(self.battle_playback_remaining);
        } else {
            delays.extend(self.turn_hold_remaining);
            delays.extend(self.run_stop_remaining);
            if let Some(motion) = self.world_motion {
                delays.push(WORLD_TICK_INTERVAL.min(motion.remaining()));
            }
        }
        delays.into_iter().min()
    }

    pub fn observe_game_events(mut self, events: &GameEvents) -> Self {
        for event in events.iter() {
            match event {
                GameEvent::World(world_event) => self.observe_world_event(world_event.clone()),
                GameEvent::BattleStarted => {
                    self.clear_world();
                    self.battle_sprite_remaining = Some(BATTLE_FRAME_INTERVAL);
                    self.sprite_frame = 0;
                }
                GameEvent::BattleActionSubmitted => {
                    self.battle_playback_remaining = Some(BATTLE_PLAYBACK_INTERVAL);
                    self.battle_sprite_remaining
                        .get_or_insert(BATTLE_FRAME_INTERVAL);
                }
                GameEvent::BattlePlaybackAdvanced { remains } => {
                    self.battle_playback_remaining = remains.then_some(BATTLE_PLAYBACK_INTERVAL);
                }
                GameEvent::ReturnedToWorld => {
                    self.battle_playback_remaining = None;
                    self.battle_sprite_remaining = None;
                    self.sprite_frame = 0;
                    self.battle_ui = BattleUiState::default();
                }
            }
        }
        self
    }

    pub fn reject_action(mut self) -> Self {
        self.pending_world_step = None;
        self
    }

    pub fn console_execution_succeeded(mut self) -> (Self, PresentationUpdate) {
        self.console.execution_succeeded();
        self.battle_playback_remaining = Some(BATTLE_PLAYBACK_INTERVAL);
        let update = PresentationUpdate {
            redraw: true,
            ime_changed: true,
            ..PresentationUpdate::default()
        };
        (self, update)
    }

    pub fn console_execution_failed(
        mut self,
        message: impl Into<String>,
    ) -> (Self, PresentationUpdate) {
        self.console.execution_failed(message);
        (self, PresentationUpdate::redraw())
    }

    fn handle_console_key(
        &mut self,
        key: &KeyEvent,
        text: Option<&TextEvent>,
    ) -> PresentationUpdate {
        let intent = console_intent_for_key(key).or_else(|| {
            (key.phase != KeyPhase::Release)
                .then(|| text.map(|text| ConsoleIntent::InsertText(text.text().to_owned())))
                .flatten()
        });
        let Some(intent) = intent else {
            return PresentationUpdate::default();
        };
        let was_open = self.console.is_open();
        match self.console.handle(intent) {
            ConsoleOutcome::Execute(invocation) => {
                PresentationUpdate::action(PresentationAction::ExecuteConsole(invocation))
            }
            ConsoleOutcome::Closed => PresentationUpdate {
                redraw: true,
                ime_changed: was_open,
                ..PresentationUpdate::default()
            },
            ConsoleOutcome::Updated | ConsoleOutcome::NoSelection | ConsoleOutcome::Ignored => {
                PresentationUpdate::redraw()
            }
        }
    }

    fn handle_world_direction(
        &mut self,
        direction: Direction,
        phase: KeyPhase,
    ) -> PresentationUpdate {
        match phase {
            KeyPhase::Press => {
                self.pressed_directions.press(direction);
                self.run_stop_remaining = None;
                self.settle_if_direction_changed();
                self.start_world_step()
                    .map_or_else(PresentationUpdate::redraw, PresentationUpdate::action)
            }
            KeyPhase::Repeat => PresentationUpdate::default(),
            KeyPhase::Release => {
                self.pressed_directions.release(direction);
                if self.pressed_directions.active().is_none() {
                    self.turn_hold_remaining = None;
                }
                self.settle_if_direction_changed();
                self.start_world_step()
                    .map_or_else(PresentationUpdate::redraw, PresentationUpdate::action)
            }
        }
    }

    fn start_world_step(&mut self) -> Option<PresentationAction> {
        if self.world_motion.is_some() || self.pending_world_step.is_some() {
            return None;
        }
        let direction = self.pressed_directions.active()?;
        let gait = if self.running { Gait::Run } else { Gait::Walk };
        self.pending_world_step = Some((direction, gait));
        Some(PresentationAction::Submit(GameCommand::StepWorld(
            direction,
        )))
    }

    fn settle_if_direction_changed(&mut self) {
        if let Some(motion) = &mut self.world_motion
            && self.pressed_directions.active() != Some(motion.direction())
        {
            motion.settle();
        }
    }

    fn observe_world_event(&mut self, event: WorldEvent) {
        let pending = self.pending_world_step.take();
        match event {
            WorldEvent::Turned { .. } => {
                self.turn_hold_remaining = Some(TURN_HOLD_DURATION);
            }
            WorldEvent::Moved { .. } => {
                let (direction, gait) = pending.expect("a moved event follows a UI world step");
                self.turn_hold_remaining = None;
                self.world_motion = Some(WorldMotion::new(direction, gait));
                self.run_stop_remaining = None;
            }
            WorldEvent::Blocked { .. } | WorldEvent::BlockedByActor { .. } => {}
            WorldEvent::EncounterTriggered { .. } => self.clear_world(),
        }
    }

    fn clear_world(&mut self) {
        self.pressed_directions.clear();
        self.world_motion = None;
        self.pending_world_step = None;
        self.turn_hold_remaining = None;
        self.run_stop_remaining = None;
    }

    fn sync_scene(&mut self, game: &GameSnapshot) {
        match game.scene() {
            GameScene::Battle => {
                self.battle_sprite_remaining
                    .get_or_insert(BATTLE_FRAME_INTERVAL);
            }
            GameScene::World => {
                self.battle_sprite_remaining = None;
                self.battle_playback_remaining = None;
                self.sprite_frame = 0;
            }
        }
    }
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

fn direction_for_key(key: &KeyEvent) -> Option<Direction> {
    match key.logical {
        LogicalKey::Named(NamedKey::ArrowUp) => Some(Direction::Up),
        LogicalKey::Named(NamedKey::ArrowDown) => Some(Direction::Down),
        LogicalKey::Named(NamedKey::ArrowLeft) => Some(Direction::Left),
        LogicalKey::Named(NamedKey::ArrowRight) => Some(Direction::Right),
        _ => None,
    }
}

fn is_console_toggle(key: &KeyEvent) -> bool {
    key.modifiers.control
        && (key.physical == Some(PhysicalKeyCode::KeyP)
            || matches!(&key.logical, LogicalKey::Character(character) if character.eq_ignore_ascii_case("p")))
}

fn console_intent_for_key(key: &KeyEvent) -> Option<ConsoleIntent> {
    if key.phase == KeyPhase::Release {
        return None;
    }
    match key.logical {
        LogicalKey::Named(NamedKey::ArrowUp) => Some(ConsoleIntent::Previous),
        LogicalKey::Named(NamedKey::ArrowDown) => Some(ConsoleIntent::Next),
        LogicalKey::Named(NamedKey::Backspace) => Some(ConsoleIntent::Backspace),
        LogicalKey::Named(NamedKey::Enter) if key.phase == KeyPhase::Press => {
            Some(ConsoleIntent::Execute)
        }
        LogicalKey::Named(NamedKey::Escape) if key.phase == KeyPhase::Press => {
            Some(ConsoleIntent::Close)
        }
        _ => None,
    }
}

fn is_enter_press(key: &KeyEvent) -> bool {
    key.phase == KeyPhase::Press && key.logical == LogicalKey::Named(NamedKey::Enter)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Gait {
    Walk,
    Run,
}

impl Gait {
    const fn duration(self) -> Duration {
        match self {
            Self::Walk => Duration::from_millis(240),
            Self::Run => Duration::from_millis(150),
        }
    }

    const fn frame_interval(self) -> Duration {
        match self {
            Self::Walk => Duration::from_millis(60),
            Self::Run => Duration::from_millis(40),
        }
    }

    const fn animation(self) -> WorldAnimation {
        match self {
            Self::Walk => WorldAnimation::Walk,
            Self::Run => WorldAnimation::Run,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct WorldMotion {
    direction: Direction,
    gait: Gait,
    elapsed: Duration,
    settling: Option<Settling>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Settling {
    elapsed: Duration,
    remaining_at_start: Duration,
}

impl WorldMotion {
    fn new(direction: Direction, gait: Gait) -> Self {
        Self {
            direction,
            gait,
            elapsed: Duration::ZERO,
            settling: None,
        }
    }

    const fn direction(self) -> Direction {
        self.direction
    }

    const fn gait(self) -> Gait {
        self.gait
    }

    fn advance(&mut self, elapsed: Duration) {
        self.elapsed = self.elapsed.saturating_add(elapsed);
        if let Some(settling) = &mut self.settling {
            settling.elapsed = settling.elapsed.saturating_add(elapsed);
        }
    }

    fn settle(&mut self) {
        if self.settling.is_none() {
            self.settling = Some(Settling {
                elapsed: Duration::ZERO,
                remaining_at_start: self.remaining(),
            });
        }
    }

    fn is_complete(self) -> bool {
        self.remaining().is_zero()
    }

    fn sprite_frame(self) -> usize {
        (self.elapsed.as_millis() / self.gait.frame_interval().as_millis()) as usize
    }

    fn pixel_offset(self, cell_size: PixelSize) -> PixelOffset {
        let duration = self.gait.duration();
        let remaining = self.remaining();
        let horizontal = remaining_pixels(cell_size.width.saturating_mul(2), remaining, duration);
        let vertical = remaining_pixels(cell_size.height.saturating_mul(2), remaining, duration);
        match self.direction {
            Direction::Up => PixelOffset::new(0, vertical),
            Direction::Down => PixelOffset::new(0, -vertical),
            Direction::Left => PixelOffset::new(horizontal, 0),
            Direction::Right => PixelOffset::new(-horizontal, 0),
        }
    }

    fn remaining(self) -> Duration {
        if let Some(settling) = self.settling {
            let elapsed = settling.elapsed.min(SETTLE_DURATION);
            let settle_remaining = SETTLE_DURATION - elapsed;
            let nanos = settling.remaining_at_start.as_nanos() * settle_remaining.as_nanos()
                / SETTLE_DURATION.as_nanos();
            return Duration::from_nanos(nanos as u64);
        }
        let duration = self.gait.duration();
        duration - self.elapsed.min(duration)
    }
}

fn remaining_pixels(extent: u32, remaining: Duration, duration: Duration) -> i32 {
    let pixels = u128::from(extent) * remaining.as_nanos() / duration.as_nanos();
    pixels.min(i32::MAX as u128) as i32
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct PressedDirections {
    pressed_at: [Option<u64>; 4],
    sequence: u64,
}

impl PressedDirections {
    fn press(&mut self, direction: Direction) {
        let index = direction_index(direction);
        if self.pressed_at[index].is_none() {
            self.sequence = self.sequence.wrapping_add(1);
            self.pressed_at[index] = Some(self.sequence);
        }
    }

    fn release(&mut self, direction: Direction) {
        self.pressed_at[direction_index(direction)] = None;
    }

    fn clear(&mut self) {
        self.pressed_at = [None; 4];
    }

    fn active(&self) -> Option<Direction> {
        let mut latest = None;
        for (index, sequence) in self.pressed_at.iter().enumerate() {
            let Some(sequence) = sequence else {
                continue;
            };
            if latest.is_none_or(|(_, latest_sequence)| sequence > latest_sequence) {
                latest = Some((index, sequence));
            }
        }
        latest.map(|(index, _)| direction_from_index(index))
    }
}

const fn direction_index(direction: Direction) -> usize {
    match direction {
        Direction::Up => 0,
        Direction::Down => 1,
        Direction::Left => 2,
        Direction::Right => 3,
    }
}

const fn direction_from_index(index: usize) -> Direction {
    match index {
        0 => Direction::Up,
        1 => Direction::Down,
        2 => Direction::Left,
        _ => Direction::Right,
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use battle_session::Action;
    use game_data::CurrentDataSet;
    use game_session::{GameCommand, GameSession};
    use punctum_gpu::{PixelOffset, PixelSize};
    use punctum_input::{
        KeyEvent, KeyPhase, LogicalKey, Modifiers, NamedKey, PhysicalKeyCode, TextEvent,
    };
    use world_application::{Direction, Position, WorldEvent};

    use super::*;

    fn key(name: NamedKey) -> KeyEvent {
        KeyEvent {
            physical: Some(PhysicalKeyCode::Unidentified),
            logical: LogicalKey::Named(name),
            modifiers: Modifiers::default(),
            phase: KeyPhase::Press,
        }
    }

    fn character(value: &str) -> KeyEvent {
        KeyEvent {
            physical: Some(PhysicalKeyCode::KeyP),
            logical: LogicalKey::Character(value.into()),
            modifiers: Modifiers::default(),
            phase: KeyPhase::Press,
        }
    }

    #[test]
    fn pokedex_opens_in_world_and_browses_a_bounded_catalog() {
        let game = GameSession::new_demo(CurrentDataSet::embedded().unwrap(), 13).unwrap();
        let snapshot = game.snapshot();
        let (mut state, update) = PresentationState::default().handle_key(
            &character("p"),
            None,
            false,
            &snapshot,
            Vec::new(),
        );
        assert!(update.redraw);
        let (next, view) = state.snapshot(&snapshot, PixelSize::new(30, 30));
        state = next;
        assert_eq!(view.pokedex.unwrap().selected_index, 0);

        let (next, update) =
            state.handle_key(&key(NamedKey::End), None, false, &snapshot, Vec::new());
        state = next;
        assert!(update.redraw);
        let (next, view) = state.snapshot(&snapshot, PixelSize::new(30, 30));
        state = next;
        assert_eq!(
            view.pokedex.unwrap().selected_index,
            POKEDEX_ENTRY_COUNT - 1
        );

        let (next, _) = state.handle_key(
            &key(NamedKey::ArrowRight),
            None,
            false,
            &snapshot,
            Vec::new(),
        );
        state = next;
        let (next, view) = state.snapshot(&snapshot, PixelSize::new(30, 30));
        state = next;
        assert_eq!(view.pokedex.unwrap().selected_index, 0);

        let (next, _) =
            state.handle_key(&key(NamedKey::Escape), None, false, &snapshot, Vec::new());
        let (_, view) = next.snapshot(&snapshot, PixelSize::new(30, 30));
        assert!(view.pokedex.is_none());
    }

    #[test]
    fn pokedex_action_selects_an_entry_without_exposing_mutable_ui_state() {
        let game = GameSession::new_demo(CurrentDataSet::embedded().unwrap(), 13).unwrap();
        let snapshot = game.snapshot();
        let (state, _) = PresentationState::default().handle_key(
            &character("p"),
            None,
            false,
            &snapshot,
            Vec::new(),
        );
        let (state, update) = state.handle_pokedex_action(PokedexAction::SelectEntry { index: 42 });
        assert!(update.redraw);
        let (_, view) = state.snapshot(&snapshot, PixelSize::new(30, 30));
        assert_eq!(view.pokedex.unwrap().selected_index, 42);

        let (state, update) = PresentationState::default()
            .handle_pokedex_action(PokedexAction::SelectEntry { index: 42 });
        assert!(!update.redraw);
        assert!(state.pokedex.is_none());
    }

    fn toggle(phase: KeyPhase, physical: bool) -> KeyEvent {
        KeyEvent {
            physical: physical.then_some(PhysicalKeyCode::KeyP),
            logical: if physical {
                LogicalKey::Unidentified
            } else {
                LogicalKey::Character("P".into())
            },
            modifiers: Modifiers {
                control: true,
                ..Modifiers::default()
            },
            phase,
        }
    }

    fn entries() -> Vec<ConsoleEntry> {
        ["/battle/move/one use", "/battle/team/two switch"]
            .into_iter()
            .map(|invocation| ConsoleEntry {
                invocation: invocation.into(),
            })
            .collect()
    }

    #[test]
    fn world_motion_uses_manual_time_and_emits_one_command_path() {
        let mut game = GameSession::new_demo(CurrentDataSet::embedded().unwrap(), 7).unwrap();
        let mut presentation = PresentationState::default();
        let (next, update) = presentation.handle_key(
            &key(NamedKey::ArrowRight),
            None,
            false,
            &game.snapshot(),
            Vec::new(),
        );
        presentation = next;
        assert_eq!(
            update.action,
            Some(PresentationAction::Submit(GameCommand::StepWorld(
                world_application::Direction::Right
            )))
        );
        let (next, events) =
            game.transition(GameCommand::StepWorld(world_application::Direction::Right));
        game = next;
        let events = events.unwrap();
        presentation = presentation.observe_game_events(&events);
        let (next, update) = presentation.advance(Duration::from_millis(90), &game.snapshot());
        presentation = next;
        assert_eq!(
            update.action,
            Some(PresentationAction::Submit(GameCommand::StepWorld(
                world_application::Direction::Right
            )))
        );
        let (next, events) =
            game.transition(GameCommand::StepWorld(world_application::Direction::Right));
        game = next;
        let events = events.unwrap();
        presentation = presentation.observe_game_events(&events);

        (presentation, _) = presentation.advance(Duration::from_millis(120), &game.snapshot());
        let (_, snapshot) = presentation.snapshot(&game.snapshot(), PixelSize::new(30, 30));
        assert_eq!(snapshot.world_pixel_offset, PixelOffset::new(-30, 0));
    }

    #[test]
    fn console_pauses_logical_time_without_a_resume_jump() {
        let game = GameSession::new_demo(CurrentDataSet::embedded().unwrap(), 9).unwrap();
        let mut presentation = PresentationState::default();
        let mut toggle = key(NamedKey::Enter);
        toggle.logical = LogicalKey::Character("p".into());
        toggle.physical = Some(PhysicalKeyCode::KeyP);
        toggle.modifiers.control = true;
        (presentation, _) =
            presentation.handle_key(&toggle, None, false, &game.snapshot(), Vec::new());
        assert!(presentation.is_console_open());

        (presentation, _) = presentation.advance(Duration::from_secs(30), &game.snapshot());
        assert_eq!(presentation.next_delay(&game.snapshot()), None);
        (presentation, _) =
            presentation.handle_key(&toggle, None, false, &game.snapshot(), Vec::new());
        assert!(!presentation.is_console_open());
        assert_eq!(
            presentation
                .snapshot(&game.snapshot(), PixelSize::new(30, 30))
                .1
                .world_pixel_offset,
            PixelOffset::new(0, 0)
        );
    }

    #[test]
    fn console_ime_and_keyboard_paths_are_explicit_updates() {
        let game = GameSession::new_demo(CurrentDataSet::embedded().unwrap(), 11).unwrap();
        let snapshot = game.snapshot();
        let state = PresentationState::default();
        let (state, update) = state.handle_preedit("closed".into());
        assert_eq!(update, PresentationUpdate::default());
        let (state, update) = state.handle_commit("closed".into());
        assert_eq!(update, PresentationUpdate::default());
        let (state, update) = state.handle_key(
            &toggle(KeyPhase::Release, false),
            None,
            false,
            &snapshot,
            entries(),
        );
        assert_eq!(update, PresentationUpdate::default());

        let (mut state, update) = state.handle_key(
            &toggle(KeyPhase::Press, false),
            None,
            false,
            &snapshot,
            entries(),
        );
        assert!(state.is_console_open());
        assert!(update.redraw && update.ime_changed);
        assert_eq!(state.console_view().items.len(), 2);
        let (next, update) =
            state.handle_key(&key(NamedKey::Tab), None, false, &snapshot, Vec::new());
        state = next;
        assert_eq!(update, PresentationUpdate::default());

        (state, _) = state.handle_preedit("拼音".into());
        assert_eq!(state.console_view().preedit, "拼音");
        let (next, update) = state.handle_key(
            &key(NamedKey::ArrowDown),
            None,
            false,
            &snapshot,
            Vec::new(),
        );
        state = next;
        assert_eq!(update, PresentationUpdate::default());
        (state, _) = state.handle_commit(String::new());
        (state, _) = state.handle_commit("battle".into());
        assert_eq!(state.console_view().query, "battle");
        (state, _) = state.handle_ime_disabled();
        assert!(state.console_view().preedit.is_empty());

        let text = TextEvent::new("x").unwrap();
        let other = KeyEvent {
            physical: None,
            logical: LogicalKey::Unidentified,
            modifiers: Modifiers::default(),
            phase: KeyPhase::Repeat,
        };
        (state, _) = state.handle_key(&other, Some(&text), false, &snapshot, Vec::new());
        assert!(state.console_view().query.ends_with('x'));
        (state, _) = state.handle_key(
            &key(NamedKey::Backspace),
            None,
            false,
            &snapshot,
            Vec::new(),
        );
        (state, _) = state.handle_key(&key(NamedKey::ArrowUp), None, false, &snapshot, Vec::new());
        (state, _) = state.handle_key(
            &key(NamedKey::ArrowDown),
            None,
            false,
            &snapshot,
            Vec::new(),
        );

        let (state, update) = state.console_execution_failed("runtime failed");
        assert!(update.redraw);
        assert_eq!(
            state.console_view().diagnostic.as_deref(),
            Some("runtime failed")
        );
        let (state, update) = state.console_execution_succeeded();
        assert!(!state.is_console_open());
        assert!(update.redraw && update.ime_changed);

        let (mut state, _) = state.handle_key(
            &toggle(KeyPhase::Press, true),
            None,
            false,
            &snapshot,
            entries(),
        );
        let (next, update) =
            state.handle_key(&key(NamedKey::Enter), None, false, &snapshot, Vec::new());
        state = next;
        assert!(matches!(
            update.action,
            Some(PresentationAction::ExecuteConsole(_))
        ));
        let (state, update) =
            state.handle_key(&key(NamedKey::Escape), None, false, &snapshot, Vec::new());
        assert!(!state.is_console_open());
        assert!(update.ime_changed);
    }

    #[test]
    fn timers_motion_and_direction_helpers_cover_all_boundaries() {
        use std::hint::black_box;

        let mut timer = None;
        assert!(!take_elapsed(&mut timer, Duration::from_secs(1)));
        timer = Some(black_box(Duration::from_millis(10)));
        assert!(!take_elapsed(
            &mut timer,
            black_box(Duration::from_millis(4))
        ));
        assert_eq!(timer, Some(Duration::from_millis(6)));
        assert!(take_elapsed(
            &mut timer,
            black_box(Duration::from_millis(6))
        ));

        assert!(!advance_periodic(
            &mut None,
            Duration::from_millis(1),
            Duration::from_millis(10)
        ));
        let mut periodic = Some(black_box(Duration::from_millis(10)));
        assert!(!advance_periodic(
            &mut periodic,
            black_box(Duration::from_millis(4)),
            black_box(Duration::from_millis(10))
        ));
        assert!(advance_periodic(
            &mut periodic,
            black_box(Duration::from_millis(27)),
            black_box(Duration::from_millis(10))
        ));
        assert_eq!(periodic, Some(Duration::from_millis(9)));

        for (name, direction) in [
            (NamedKey::ArrowUp, Direction::Up),
            (NamedKey::ArrowDown, Direction::Down),
            (NamedKey::ArrowLeft, Direction::Left),
            (NamedKey::ArrowRight, Direction::Right),
        ] {
            assert_eq!(direction_for_key(&key(name)), Some(direction));
            assert_eq!(direction_from_index(direction_index(direction)), direction);
        }
        assert_eq!(direction_for_key(&key(NamedKey::Enter)), None);
        assert!(is_console_toggle(&toggle(KeyPhase::Press, true)));
        assert!(is_console_toggle(&toggle(KeyPhase::Press, false)));
        assert!(is_enter_press(&key(NamedKey::Enter)));
        assert!(!is_enter_press(&key(NamedKey::Escape)));
        let mut release = key(NamedKey::ArrowUp);
        release.phase = KeyPhase::Release;
        assert_eq!(console_intent_for_key(&release), None);
        assert_eq!(
            console_intent_for_key(&key(NamedKey::ArrowUp)),
            Some(ConsoleIntent::Previous)
        );
        assert_eq!(
            console_intent_for_key(&key(NamedKey::ArrowDown)),
            Some(ConsoleIntent::Next)
        );
        assert_eq!(
            console_intent_for_key(&key(NamedKey::Backspace)),
            Some(ConsoleIntent::Backspace)
        );
        assert_eq!(
            console_intent_for_key(&key(NamedKey::Enter)),
            Some(ConsoleIntent::Execute)
        );
        assert_eq!(
            console_intent_for_key(&key(NamedKey::Escape)),
            Some(ConsoleIntent::Close)
        );
        assert_eq!(console_intent_for_key(&key(NamedKey::Tab)), None);

        assert_eq!(Gait::Walk.duration(), Duration::from_millis(240));
        assert_eq!(Gait::Run.duration(), Duration::from_millis(150));
        assert_eq!(Gait::Walk.frame_interval(), Duration::from_millis(60));
        assert_eq!(Gait::Run.frame_interval(), Duration::from_millis(40));
        assert_eq!(Gait::Walk.animation(), WorldAnimation::Walk);
        assert_eq!(Gait::Run.animation(), WorldAnimation::Run);

        let cell = PixelSize::new(10, 20);
        for (direction, expected) in [
            (Direction::Up, PixelOffset::new(0, 40)),
            (Direction::Down, PixelOffset::new(0, -40)),
            (Direction::Left, PixelOffset::new(20, 0)),
            (Direction::Right, PixelOffset::new(-20, 0)),
        ] {
            let mut motion = WorldMotion::new(direction, Gait::Walk);
            assert_eq!(motion.direction(), direction);
            assert_eq!(motion.gait(), Gait::Walk);
            assert_eq!(motion.pixel_offset(cell), expected);
            assert_eq!(motion.sprite_frame(), 0);
            motion.advance(Duration::from_millis(60));
            assert_eq!(motion.sprite_frame(), 1);
            motion.settle();
            motion.settle();
            motion.advance(SETTLE_DURATION);
            assert!(motion.is_complete());
        }
        assert_eq!(
            remaining_pixels(u32::MAX, Duration::from_secs(1), Duration::from_secs(1)),
            i32::MAX
        );

        let mut directions = PressedDirections::default();
        for direction in [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ] {
            directions.press(direction);
            directions.press(direction);
            assert_eq!(directions.active(), Some(direction));
        }
        directions.release(Direction::Right);
        assert_eq!(directions.active(), Some(Direction::Left));
        directions.clear();
        assert_eq!(directions.active(), None);
    }

    #[test]
    fn world_focus_motion_and_deadlines_are_reduced_without_system_time() {
        let game = GameSession::new_demo(CurrentDataSet::embedded().unwrap(), 13).unwrap();
        let snapshot = game.snapshot();
        let mut state = PresentationState {
            battle_sprite_remaining: Some(Duration::from_secs(1)),
            battle_playback_remaining: Some(Duration::from_secs(1)),
            sprite_frame: 9,
            ..PresentationState::default()
        };

        let (next, update) =
            state.handle_key(&key(NamedKey::Tab), None, false, &snapshot, Vec::new());
        state = next;
        assert_eq!(update, PresentationUpdate::default());
        assert!(state.battle_sprite_remaining.is_none());
        assert!(state.battle_playback_remaining.is_none());
        assert_eq!(state.sprite_frame, 0);

        let mut repeat = key(NamedKey::ArrowRight);
        repeat.phase = KeyPhase::Repeat;
        let (next, update) = state.handle_key(&repeat, None, false, &snapshot, Vec::new());
        state = next;
        assert_eq!(update, PresentationUpdate::default());
        let mut release = key(NamedKey::ArrowRight);
        release.phase = KeyPhase::Release;
        let (next, update) = state.handle_key(&release, None, false, &snapshot, Vec::new());
        state = next;
        assert!(update.redraw);
        state.run_stop_remaining = Some(Duration::from_secs(1));

        let (next, update) = state.handle_key(
            &key(NamedKey::ArrowRight),
            None,
            true,
            &snapshot,
            Vec::new(),
        );
        state = next;
        assert_eq!(
            update.action,
            Some(PresentationAction::Submit(GameCommand::StepWorld(
                Direction::Right
            )))
        );
        assert!(state.start_world_step().is_none());
        state = state.reject_action();
        assert!(state.pending_world_step.is_none());

        state.pressed_directions.press(Direction::Right);
        state.pending_world_step = Some((Direction::Right, Gait::Run));
        state.run_stop_remaining = Some(Duration::from_secs(1));
        state.observe_world_event(WorldEvent::Moved {
            from: Position::new(3, 6),
            to: Position::new(4, 6),
        });
        assert_eq!(state.next_delay(&snapshot), Some(WORLD_TICK_INTERVAL));
        let (_, moving) = state.clone().snapshot(&snapshot, PixelSize::new(30, 30));
        assert_eq!(moving.world_animation, WorldAnimation::Run);
        assert_eq!(moving.world_pixel_offset, PixelOffset::new(-60, 0));

        state.pressed_directions.press(Direction::Left);
        state.settle_if_direction_changed();
        assert!(state.world_motion.unwrap().settling.is_some());
        let (next, update) = state.focus_lost();
        state = next;
        assert!(update.redraw);
        let (next, update) = state.advance(SETTLE_DURATION, &snapshot);
        state = next;
        assert!(update.redraw);
        assert!(state.world_motion.is_none());
        assert_eq!(state.run_stop_remaining, Some(RUN_STOP_DURATION));
        let (_, stopping) = state.clone().snapshot(&snapshot, PixelSize::new(30, 30));
        assert_eq!(stopping.world_animation, WorldAnimation::RunStopping);
        let (next, update) = state.advance(RUN_STOP_DURATION, &snapshot);
        state = next;
        assert!(update.redraw);
        assert!(state.run_stop_remaining.is_none());
        (state, _) = state.focus_lost();

        state.pressed_directions.press(Direction::Up);
        state.pending_world_step = Some((Direction::Up, Gait::Walk));
        state.observe_world_event(WorldEvent::Turned {
            from: Direction::Down,
            to: Direction::Up,
        });
        assert_eq!(state.next_delay(&snapshot), Some(TURN_HOLD_DURATION));
        let (next, update) = state.advance(TURN_HOLD_DURATION, &snapshot);
        state = next;
        assert_eq!(
            update.action,
            Some(PresentationAction::Submit(GameCommand::StepWorld(
                Direction::Up
            )))
        );

        state.observe_world_event(WorldEvent::Blocked {
            at: Position::new(3, 5),
        });
        assert!(state.pending_world_step.is_none());

        let mut chained = PresentationState::default();
        chained.pressed_directions.press(Direction::Right);
        let mut complete = WorldMotion::new(Direction::Right, Gait::Walk);
        complete.advance(Gait::Walk.duration());
        chained.world_motion = Some(complete);
        let (_chained, update) = chained.advance(Duration::ZERO, &snapshot);
        assert_eq!(
            update.action,
            Some(PresentationAction::Submit(GameCommand::StepWorld(
                Direction::Right
            )))
        );

        state.pending_world_step = Some((Direction::Down, Gait::Walk));
        state.world_motion = Some(WorldMotion::new(Direction::Down, Gait::Walk));
        state.turn_hold_remaining = Some(Duration::from_secs(1));
        state.run_stop_remaining = Some(Duration::from_secs(1));
        state.observe_world_event(WorldEvent::EncounterTriggered {
            at: Position::new(3, 7),
        });
        assert!(state.pressed_directions.active().is_none());
        assert!(state.world_motion.is_none());
        assert_eq!(state.next_delay(&snapshot), None);
    }

    #[test]
    fn real_game_events_drive_battle_playback_and_return_to_world() {
        let mut game = GameSession::new_demo(CurrentDataSet::embedded().unwrap(), 19).unwrap();
        let mut state = PresentationState::default();
        for _ in 0..4 {
            state.pending_world_step = Some((Direction::Right, Gait::Walk));
            let (next, events) = game.transition(GameCommand::StepWorld(Direction::Right));
            game = next;
            state = state.observe_game_events(&events.unwrap());
        }
        assert_eq!(game.snapshot().scene(), GameScene::Battle);
        let (next, frame) = state.snapshot(&game.snapshot(), PixelSize::new(30, 30));
        state = next;
        assert_eq!(frame.world_animation, WorldAnimation::Stand);
        assert_eq!(frame.world_pixel_offset, PixelOffset::new(0, 0));
        assert_eq!(
            state.next_delay(&game.snapshot()),
            Some(BATTLE_FRAME_INTERVAL)
        );

        let (next, update) = state.handle_key(
            &key(NamedKey::Tab),
            None,
            false,
            &game.snapshot(),
            Vec::new(),
        );
        state = next;
        assert_eq!(update, PresentationUpdate::default());
        let (next, update) = state.handle_key(
            &key(NamedKey::Enter),
            None,
            false,
            &game.snapshot(),
            Vec::new(),
        );
        state = next;
        assert!(update.redraw && update.action.is_none());
        let (next, update) = state.handle_key(
            &key(NamedKey::Enter),
            None,
            false,
            &game.snapshot(),
            Vec::new(),
        );
        state = next;
        assert!(matches!(
            update.action,
            Some(PresentationAction::Submit(GameCommand::SubmitBattleAction(
                _
            )))
        ));

        let (next, events) = game.transition(GameCommand::SubmitBattleAction(Action::Run));
        game = next;
        state = state.observe_game_events(&events.unwrap());
        assert!(game.has_pending_playback());
        let (next, update) = state.advance(BATTLE_PLAYBACK_INTERVAL, &game.snapshot());
        state = next;
        assert_eq!(
            update.action,
            Some(PresentationAction::Submit(
                GameCommand::AdvanceBattlePlayback
            ))
        );
        assert!(update.redraw);

        while game.has_pending_playback() {
            let (next, events) = game.transition(GameCommand::AdvanceBattlePlayback);
            game = next;
            state = state.observe_game_events(&events.unwrap());
        }
        assert!(game.snapshot().battle().unwrap().is_finished());
        let (next, update) = state.handle_key(
            &key(NamedKey::Enter),
            None,
            false,
            &game.snapshot(),
            Vec::new(),
        );
        state = next;
        assert_eq!(
            update.action,
            Some(PresentationAction::Submit(GameCommand::LeaveFinishedBattle))
        );

        let (next, events) = game.transition(GameCommand::LeaveFinishedBattle);
        game = next;
        state = state.observe_game_events(&events.unwrap());
        assert_eq!(game.snapshot().scene(), GameScene::World);
        assert_eq!(state.next_delay(&game.snapshot()), None);
        assert_eq!(state.sprite_frame, 0);
    }
}
