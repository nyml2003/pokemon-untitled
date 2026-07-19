use battle_application::Action;
use game_data::CurrentDataSet;
use game_session::{GameCommand, GameScene, GameSession};
use world_application::{Direction, Position};

fn submit(game: GameSession, command: GameCommand) -> GameSession {
    let (game, result) = game.transition(command);
    result.unwrap();
    game
}

fn enter_battle(mut game: GameSession) -> GameSession {
    for _ in 0..4 {
        game = submit(game, GameCommand::StepWorld(Direction::Right));
    }
    assert_eq!(game.snapshot().scene(), GameScene::Battle);
    game
}

#[test]
fn command_battle_slice_can_reach_a_deterministic_finish() {
    let mut game = GameSession::new_demo(CurrentDataSet::embedded().unwrap(), 0xA2B3_C4D5).unwrap();
    game = enter_battle(game);
    let opening_turn = game.snapshot().battle().unwrap().observation().turn();
    let mut submitted_actions = 0_usize;

    while !game.snapshot().battle().unwrap().is_finished() {
        if game.has_pending_playback() {
            game = submit(game, GameCommand::AdvanceBattlePlayback);
            continue;
        }
        let actions = game.legal_player_actions();
        let action = actions
            .iter()
            .copied()
            .find(|action| matches!(action, Action::UseMove(_)))
            .or_else(|| actions.first().copied())
            .expect("an unfinished battle always offers a legal player action");
        game = submit(game, GameCommand::SubmitBattleAction(action));
        submitted_actions += 1;
        assert!(submitted_actions < 500, "the demo battle must converge");
    }

    let finished_turn = game.snapshot().battle().unwrap().observation().turn();
    assert!(finished_turn > opening_turn);
    assert!(submitted_actions > 1);
}

#[test]
fn world_slice_enters_battle_and_returns_to_the_same_map_position() {
    let mut game = GameSession::new_demo(CurrentDataSet::embedded().unwrap(), 0xA2B3_C4D5).unwrap();

    game = submit(game, GameCommand::StepWorld(Direction::Right));
    assert_eq!(game.snapshot().world().player(), Position::new(3, 6));
    assert_eq!(game.snapshot().world().facing(), Direction::Right);

    for _ in 0..3 {
        game = submit(game, GameCommand::StepWorld(Direction::Right));
    }
    assert_eq!(game.snapshot().scene(), GameScene::Battle);
    assert_eq!(game.snapshot().world().player(), Position::new(6, 6));

    game = submit(game, GameCommand::SubmitBattleAction(Action::Run));
    while game.has_pending_playback() {
        game = submit(game, GameCommand::AdvanceBattlePlayback);
    }
    game = submit(game, GameCommand::LeaveFinishedBattle);

    assert_eq!(game.snapshot().scene(), GameScene::World);
    assert_eq!(game.snapshot().world().player(), Position::new(6, 6));
}
