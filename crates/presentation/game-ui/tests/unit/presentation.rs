use std::time::Duration;

use battle_session::Action;
use game_data::CurrentDataSet;
use game_session::{GameCommand, GameSession};
use punctum_gpu::{PixelOffset, PixelSize};
use punctum_input::{KeyEvent, KeyPhase, LogicalKey, Modifiers, NamedKey, PhysicalKeyCode};
use world_application::{Direction, Position, WorldEvent};

use super::*;

fn key(name: NamedKey) -> KeyEvent {
    let physical = match name {
        NamedKey::ArrowUp => PhysicalKeyCode::KeyW,
        NamedKey::ArrowDown => PhysicalKeyCode::KeyS,
        NamedKey::ArrowLeft => PhysicalKeyCode::KeyA,
        NamedKey::ArrowRight => PhysicalKeyCode::KeyD,
        NamedKey::Enter => PhysicalKeyCode::KeyJ,
        NamedKey::Escape => PhysicalKeyCode::KeyK,
        _ => PhysicalKeyCode::Unidentified,
    };
    KeyEvent {
        physical: Some(physical),
        logical: LogicalKey::Named(name),
        modifiers: Modifiers::default(),
        phase: KeyPhase::Press,
    }
}

#[test]
fn pokedex_browses_a_bounded_catalog_with_gba_controls() {
    let game = GameSession::new_demo(CurrentDataSet::embedded().unwrap(), 13).unwrap();
    let snapshot = game.snapshot();
    let mut state = PresentationState {
        pokedex: Some(PokedexUiSnapshot { selected_index: 0 }),
        ..PresentationState::default()
    };
    let (next, view) = state.snapshot(&snapshot, PixelSize::new(30, 30));
    state = next;
    assert_eq!(view.pokedex.unwrap().selected_index, 0);

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
    assert_eq!(view.pokedex.unwrap().selected_index, 1);

    let (next, _) = state.handle_key(&key(NamedKey::Escape), None, false, &snapshot, Vec::new());
    let (_, view) = next.snapshot(&snapshot, PixelSize::new(30, 30));
    assert!(view.pokedex.is_none());
}

#[test]
fn pokedex_action_selects_an_entry_without_exposing_mutable_ui_state() {
    let game = GameSession::new_demo(CurrentDataSet::embedded().unwrap(), 13).unwrap();
    let snapshot = game.snapshot();
    let state = PresentationState {
        pokedex: Some(PokedexUiSnapshot { selected_index: 0 }),
        ..PresentationState::default()
    };
    let (state, update) = state.handle_pokedex_action(PokedexAction::SelectEntry { index: 42 });
    assert!(update.redraw);
    let (_, view) = state.snapshot(&snapshot, PixelSize::new(30, 30));
    assert_eq!(view.pokedex.unwrap().selected_index, 42);

    let (state, update) = PresentationState::default()
        .handle_pokedex_action(PokedexAction::SelectEntry { index: 42 });
    assert!(!update.redraw);
    assert!(state.pokedex.is_none());
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
        assert_eq!(
            direction_for_control(GameControl::from_key_event(&key(name))),
            Some(direction)
        );
        assert_eq!(direction_from_index(direction_index(direction)), direction);
    }
    assert_eq!(
        direction_for_control(GameControl::from_key_event(&key(NamedKey::Enter))),
        None
    );
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

    let (next, update) = state.handle_key(&key(NamedKey::Tab), None, false, &snapshot, Vec::new());
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
