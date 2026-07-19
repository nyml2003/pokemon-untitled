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
            let Some(pokedex) = self.pokedex.as_ref() else {
                return PresentationUpdate::default();
            };
            let selected_index = pokedex.selected_index;
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
                let Some((direction, gait)) = pending else {
                    return;
                };
                self.turn_hold_remaining = None;
                self.world_motion = Some(WorldMotion::new(direction, gait));
                self.run_stop_remaining = None;
            }
            WorldEvent::Blocked { .. } | WorldEvent::BlockedByActor { .. } => {}
            WorldEvent::EncounterTriggered { .. } | WorldEvent::TransitionRejected { .. } => {
                self.clear_world()
            }
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
#[path = "../tests/unit/presentation.rs"]
mod tests;
