use battle_application::Action;
use game_data::CurrentDataSet;
use game_foundation::{
    ContentPackage, Direction, GameIdError, GameState, ItemId, Money, NpcId, SaveEnvelope,
    ThinSliceContent, WarpId,
};

use super::super::{ProductCommand, ProductError, ProductSession};

fn id<T>(value: Result<T, GameIdError>) -> Result<T, String> {
    value.map_err(|error| format!("id: {error:?}"))
}

fn session() -> Result<ProductSession, String> {
    let data = CurrentDataSet::embedded().map_err(|error| format!("data: {error:?}"))?;
    let content = ThinSliceContent::standard().map_err(|error| format!("content: {error:?}"))?;
    ProductSession::new(data, content).map_err(|error| format!("session: {error:?}"))
}

fn transition(session: ProductSession, command: ProductCommand) -> Result<ProductSession, String> {
    let (session, result) = session.transition(command);
    result
        .map(|_| session)
        .map_err(|error| format!("command rejected: {error:?}"))
}

fn finish_battle(mut session: ProductSession) -> Result<ProductSession, String> {
    for _ in 0..1_000 {
        let snapshot = session.snapshot();
        let battle = snapshot
            .battle()
            .ok_or_else(|| String::from("battle missing before completion"))?;
        if battle.is_finished() {
            return transition(session, ProductCommand::LeaveFinishedBattle);
        }
        let actions = session.legal_player_actions();
        if actions.is_empty() {
            session = transition(session, ProductCommand::AdvanceBattlePlayback)?;
            continue;
        }
        let action = actions
            .iter()
            .copied()
            .find(|action| matches!(action, Action::UseMove(_)))
            .or_else(|| actions.first().copied())
            .ok_or_else(|| String::from("battle offered no action"))?;
        session = transition(session, ProductCommand::SubmitBattleAction(action))?;
    }
    Err(String::from(
        "battle did not complete within deterministic bound",
    ))
}

fn complete_m1_story() -> Result<ProductSession, String> {
    let game = session()?;
    let game = transition(game, ProductCommand::NewGame)?;
    let game = transition(game, ProductCommand::Move(Direction::Up))?;
    let game = transition(game, ProductCommand::InteractFront)?;
    let game = transition(
        game,
        ProductCommand::Warp(id(WarpId::new("town-to-route"))?),
    )?;
    let game = transition(game, ProductCommand::Move(Direction::Right))?;
    let game = transition(game, ProductCommand::BeginEncounter { roll: 7 })?;
    let game = finish_battle(game)?;
    let game = transition(game, ProductCommand::Move(Direction::Down))?;
    let game = transition(game, ProductCommand::Move(Direction::Right))?;
    let game = transition(game, ProductCommand::InteractFront)?;
    let game = finish_battle(game)?;
    let game = transition(
        game,
        ProductCommand::Warp(id(WarpId::new("route-to-town"))?),
    )?;
    let game = transition(game, ProductCommand::Move(Direction::Up))?;
    transition(
        game,
        ProductCommand::BuyFromFront {
            item: id(ItemId::new("potion"))?,
            quantity: 1,
        },
    )
}

#[test]
fn product_session_runs_real_wild_and_trainer_battles_then_commits_persistent_state()
-> Result<(), String> {
    let game = session()?;
    let game = transition(game, ProductCommand::NewGame)?;
    let game = transition(game, ProductCommand::Move(Direction::Up))?;
    let game = transition(game, ProductCommand::InteractFront)?;
    assert_eq!(game.snapshot().state().party().len(), 1);
    let game = transition(
        game,
        ProductCommand::Warp(id(WarpId::new("town-to-route"))?),
    )?;
    let game = transition(game, ProductCommand::Move(Direction::Right))?;
    let game = transition(game, ProductCommand::BeginEncounter { roll: 7 })?;

    let snapshot = game.snapshot();
    let state = snapshot.state();
    let battle = snapshot
        .battle()
        .ok_or_else(|| String::from("wild battle was not created"))?;
    let player = battle
        .observation()
        .own()
        .members()
        .first()
        .ok_or_else(|| String::from("missing product player"))?;
    let move_ = player
        .moves()
        .first()
        .ok_or_else(|| String::from("missing product move"))?;
    assert_eq!(player.current_hp(), u32::from(state.party()[0].hp()));
    assert_eq!(move_.current_pp(), state.party()[0].pp());
    assert!(state.active_battle().is_some());

    let game = finish_battle(game)?;
    let state = game.snapshot().state().clone();
    assert!(state.active_battle().is_none());
    assert_eq!(state.party()[0].experience(), 20);
    assert_eq!(state.money(), Money::new(200));
    assert!(state.party()[0].hp() <= 35);
    assert!(state.party()[0].pp() <= 35);

    let game = transition(game, ProductCommand::Move(Direction::Down))?;
    let game = transition(game, ProductCommand::Move(Direction::Right))?;
    let game = transition(game, ProductCommand::InteractFront)?;
    assert!(game.snapshot().battle().is_some());
    let game = finish_battle(game)?;
    let state = game.snapshot().state().clone();
    assert!(state.active_battle().is_none());
    assert!(
        state
            .defeated_trainers()
            .contains(&id(NpcId::new("route-trainer"))?)
    );
    assert_eq!(state.party()[0].experience(), 65);
    assert_eq!(state.money(), Money::new(320));
    Ok(())
}

#[test]
fn product_session_rejects_world_commands_during_a_real_battle_without_mutating_state()
-> Result<(), String> {
    let game = session()?;
    let game = transition(game, ProductCommand::Move(Direction::Up))?;
    let game = transition(game, ProductCommand::InteractFront)?;
    let game = transition(
        game,
        ProductCommand::Warp(id(WarpId::new("town-to-route"))?),
    )?;
    let game = transition(game, ProductCommand::Move(Direction::Right))?;
    let game = transition(game, ProductCommand::BeginEncounter { roll: 7 })?;
    let before = game.snapshot();
    let (game, result) = game.transition(ProductCommand::Move(Direction::Down));
    assert!(result.is_err());
    assert_eq!(game.snapshot(), before);
    Ok(())
}

#[test]
fn product_session_saves_only_at_a_world_safety_point_and_reloads_the_same_state()
-> Result<(), String> {
    let game = session()?;
    let game = transition(game, ProductCommand::Move(Direction::Up))?;
    let game = transition(game, ProductCommand::InteractFront)?;
    let game = transition(
        game,
        ProductCommand::Warp(id(WarpId::new("town-to-route"))?),
    )?;
    let game = transition(game, ProductCommand::Move(Direction::Right))?;
    let game = transition(game, ProductCommand::BeginEncounter { roll: 7 })?;
    assert!(game.save().is_err());
    let game = finish_battle(game)?;
    let before = game.snapshot().state().clone();
    let save = game.save().map_err(|error| format!("save: {error:?}"))?;
    assert_eq!(save.ruleset_reference(), Some("legacy-gen3@1"));
    assert_eq!(save.content_package_reference(), Some("starter-region@1"));
    let bytes = save
        .to_json()
        .map_err(|error| format!("encode: {error:?}"))?;
    let content = ThinSliceContent::standard().map_err(|error| format!("content: {error:?}"))?;
    let loaded =
        SaveEnvelope::from_json(&content, &bytes).map_err(|error| format!("load: {error:?}"))?;
    let data = CurrentDataSet::embedded().map_err(|error| format!("data: {error:?}"))?;
    let reloaded = ProductSession::from_save(data, content, loaded)
        .map_err(|error| format!("session reload: {error:?}"))?;
    assert_eq!(reloaded.snapshot().state(), &before);
    assert!(reloaded.snapshot().save_available());
    Ok(())
}

#[test]
fn product_session_completes_the_m1_story_then_reloads_the_same_persistent_state()
-> Result<(), String> {
    let game = complete_m1_story()?;

    let before = game.snapshot().state().clone();
    assert_eq!(before.money(), Money::new(290));
    assert_eq!(before.inventory().quantity(&id(ItemId::new("potion"))?), 2);
    assert!(
        before
            .defeated_trainers()
            .contains(&id(NpcId::new("route-trainer"))?)
    );

    let save = game.save().map_err(|error| format!("save: {error:?}"))?;
    assert_eq!(save.ruleset_reference(), Some("legacy-gen3@1"));
    assert_eq!(save.content_package_reference(), Some("starter-region@1"));
    let bytes = save
        .to_json()
        .map_err(|error| format!("encode: {error:?}"))?;
    let content = ThinSliceContent::standard().map_err(|error| format!("content: {error:?}"))?;
    let loaded =
        SaveEnvelope::from_json(&content, &bytes).map_err(|error| format!("load: {error:?}"))?;
    let data = CurrentDataSet::embedded().map_err(|error| format!("data: {error:?}"))?;
    let reloaded = ProductSession::from_save(data, content, loaded)
        .map_err(|error| format!("session reload: {error:?}"))?;
    assert_eq!(reloaded.snapshot().state(), &before);
    Ok(())
}

#[test]
fn product_session_repeats_the_complete_m1_story_deterministically() -> Result<(), String> {
    let first = complete_m1_story()?.snapshot();
    let second = complete_m1_story()?.snapshot();
    assert_eq!(first, second);
    Ok(())
}

#[test]
fn product_session_rejects_a_save_from_another_ruleset() -> Result<(), String> {
    let content = ThinSliceContent::standard().map_err(|error| format!("content: {error:?}"))?;
    let state =
        game_foundation::GameState::new(&content).map_err(|error| format!("state: {error:?}"))?;
    let save = SaveEnvelope::from_state_with_ruleset(&content, state, "another-ruleset@1")
        .map_err(|error| format!("save: {error:?}"))?;
    let data = CurrentDataSet::embedded().map_err(|error| format!("data: {error:?}"))?;
    assert!(matches!(
        ProductSession::from_save(data, content, save),
        Err(ProductError::RulesetReferenceMismatch { .. })
    ));
    Ok(())
}

#[test]
fn product_session_rejects_a_save_from_another_content_package() -> Result<(), String> {
    let content = ThinSliceContent::standard().map_err(|error| format!("content: {error:?}"))?;
    let state = GameState::new(&content).map_err(|error| format!("state: {error:?}"))?;
    let save = SaveEnvelope::from_state_with_references(
        &content,
        state,
        "legacy-gen3@1",
        "another-package@1",
    )
    .map_err(|error| format!("save: {error:?}"))?;
    let data = CurrentDataSet::embedded().map_err(|error| format!("data: {error:?}"))?;
    let package = ContentPackage::standard().map_err(|error| format!("package: {error:?}"))?;
    assert!(matches!(
        ProductSession::from_package_save(data, package, save),
        Err(ProductError::ContentPackageReferenceMismatch { .. })
    ));
    Ok(())
}

#[test]
fn product_front_intents_reject_missing_npcs_without_mutating_state() -> Result<(), String> {
    let game = session()?;
    let before = game.snapshot();
    let (game, result) = game.transition(ProductCommand::InteractFront);
    assert!(result.is_err());
    assert_eq!(game.snapshot(), before);

    let (game, result) = game.transition(ProductCommand::BuyFromFront {
        item: id(ItemId::new("potion"))?,
        quantity: 1,
    });
    assert!(result.is_err());
    assert_eq!(game.snapshot(), before);
    Ok(())
}
