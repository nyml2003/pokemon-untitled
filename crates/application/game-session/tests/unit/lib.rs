use battle_application::Action;
use game_data::CurrentDataSet;
use world_application::{Direction, Position};

use super::{GameCommand, GameError, GameScene, GameSession};

fn submit(game: GameSession, command: GameCommand) -> GameSession {
    let (game, result) = game.transition(command);
    result.unwrap();
    game
}

#[test]
fn equal_seed_and_commands_produce_equal_snapshots() {
    let mut first = GameSession::new_demo(CurrentDataSet::embedded().unwrap(), 17).unwrap();
    let mut second = GameSession::new_demo(CurrentDataSet::embedded().unwrap(), 17).unwrap();
    let commands = [
        GameCommand::StepWorld(Direction::Right),
        GameCommand::StepWorld(Direction::Right),
        GameCommand::StepWorld(Direction::Right),
        GameCommand::StepWorld(Direction::Right),
    ];

    for command in commands {
        let (next_first, first_result) = first.transition(command);
        let (next_second, second_result) = second.transition(command);
        first = next_first;
        second = next_second;
        assert_eq!(first_result, second_result);
        assert_eq!(first.snapshot(), second.snapshot());
    }
    assert_eq!(first.snapshot().scene(), GameScene::Battle);
    assert_eq!(first.snapshot().world().player(), Position::new(6, 6));
}

#[test]
fn battle_lifecycle_is_owned_by_the_game_session() {
    let mut game = GameSession::new_demo(CurrentDataSet::embedded().unwrap(), 23).unwrap();
    for _ in 0..4 {
        game = submit(game, GameCommand::StepWorld(Direction::Right));
    }

    while game.snapshot().scene() == GameScene::Battle {
        let battle = game.snapshot().battle().unwrap().session().clone();
        if matches!(
            battle.interaction(),
            battle_session::BattleInteraction::Finished(_)
        ) {
            game = submit(game, GameCommand::LeaveFinishedBattle);
            break;
        }
        if game.legal_player_actions().is_empty() {
            game = submit(game, GameCommand::AdvanceBattlePlayback);
        } else {
            let action = game
                .legal_player_actions()
                .into_iter()
                .find(|action| matches!(action, Action::UseMove(_)))
                .or_else(|| game.legal_player_actions().into_iter().next())
                .unwrap();
            game = submit(game, GameCommand::SubmitBattleAction(action));
        }
    }

    assert_eq!(game.snapshot().scene(), GameScene::World);
    assert_eq!(game.snapshot().world().player(), Position::new(6, 6));
}

#[test]
fn wrong_scene_rejects_commands_without_mutating_the_snapshot() {
    let mut game = GameSession::new_demo(CurrentDataSet::embedded().unwrap(), 31).unwrap();
    let before = game.snapshot();
    let (next, result) = game.transition(GameCommand::SubmitBattleAction(Action::Run));
    game = next;

    assert!(matches!(
        result,
        Err(GameError::WrongScene {
            expected: GameScene::Battle,
            actual: GameScene::World,
        })
    ));
    assert_eq!(game.snapshot(), before);
}

#[test]
fn commands_events_and_battle_guards_cover_the_public_boundary() {
    let game = GameSession::new_demo(CurrentDataSet::embedded().unwrap(), 41).unwrap();
    let (game, faced) = game.transition(GameCommand::FaceWorld(Direction::Up));
    let faced = faced.unwrap();
    assert!(faced.world_event().is_some());
    assert_eq!(faced.iter().count(), 1);

    let (mut game, moved) = game.transition(GameCommand::MoveWorld(Direction::Down));
    assert!(moved.unwrap().world_event().is_some());
    for _ in 0..4 {
        game = submit(game, GameCommand::StepWorld(Direction::Right));
    }
    let battle = game.snapshot();
    let battle = battle.battle().unwrap();
    assert_eq!(battle.observation().viewer(), battle_application::Side::One);
    assert!(!battle.is_finished());
    assert!(battle.own_sprite_slot() < battle_application::TEAM_SIZE);
    assert!(battle.opponent_sprite_slot() < battle_application::TEAM_SIZE);
    let manifest = game.sprite_manifest().unwrap();
    assert_eq!(manifest.player().len(), battle_application::TEAM_SIZE);
    assert_eq!(manifest.opponent().len(), battle_application::TEAM_SIZE);
    assert!(!game.has_pending_playback());

    let (game, unavailable) = game.transition(GameCommand::AdvanceBattlePlayback);
    assert_eq!(unavailable, Err(GameError::PlaybackUnavailable));
    let (game, unfinished) = game.transition(GameCommand::LeaveFinishedBattle);
    assert_eq!(unfinished, Err(GameError::BattleNotFinished));

    let action = game.legal_player_actions()[0];
    let (game, submitted) = game.transition(GameCommand::SubmitBattleAction(action));
    let submitted = submitted.unwrap();
    assert_eq!(submitted.world_event(), None);
    assert!(matches!(
        submitted.iter().next(),
        Some(super::GameEvent::BattleActionSubmitted)
    ));
    assert!(game.has_pending_playback());
    let (_game, locked) = game.transition(GameCommand::SubmitBattleAction(action));
    assert_eq!(locked, Err(GameError::PlayerActionUnavailable));
}

#[test]
fn error_conversions_keep_their_owner_layer() {
    let world = world_application::WorldError::PlayerOutOfBounds(Position::new(99, 99));
    assert!(matches!(GameError::from(world), GameError::World(_)));
    assert!(matches!(
        GameError::from(battle_session::SessionError::InputLocked),
        GameError::Battle(_)
    ));
    let roster = super::RosterError::MissingPokemon(game_data::PokemonFormId(999));
    let setup = super::GameSetupError::from(roster);
    assert!(matches!(GameError::from(setup), GameError::Setup(_)));
    let battle = battle_application::BattleError::BattleAlreadyFinished {
        outcome: battle_application::BattleOutcome::Draw,
    };
    assert!(matches!(
        super::GameSetupError::from(battle),
        super::GameSetupError::Battle(_)
    ));
}

#[test]
fn autonomous_world_ticks_are_explicit_non_player_transitions() {
    let game = GameSession::new_demo(CurrentDataSet::embedded().unwrap(), 47).unwrap();
    let before = game.snapshot();
    let (game, events) = game.advance_world_tick();

    assert_eq!(events.unwrap().iter().count(), 0);
    assert_eq!(game.snapshot(), before);
}
