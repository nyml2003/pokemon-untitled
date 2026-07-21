use game_foundation::{
    BattleId, BattleOutcome, CreatureId, Direction, GameCommand, GameState, MapId, Money, NpcId,
    Position, ThinSliceContent, TrainerId,
};

use super::super::{
    ActiveBattleContext, BattleContinuation, BattleContractError, BattleResultPatch, BattleSource,
    BattleStartRequest,
};

fn content() -> Result<ThinSliceContent, String> {
    ThinSliceContent::standard().map_err(|error| format!("content: {error:?}"))
}

fn id<T>(value: Result<T, game_foundation::GameIdError>) -> Result<T, String> {
    value.map_err(|error| format!("id: {error:?}"))
}

fn starter_party(
    content: &ThinSliceContent,
) -> Result<Vec<game_foundation::CreatureState>, String> {
    let state = GameState::new(content).map_err(|error| format!("state: {error:?}"))?;
    let (state, result) = state.transition(content, GameCommand::NewGame);
    result.map_err(|error| format!("new game: {error:?}"))?;
    let (state, result) = state.transition(
        content,
        GameCommand::Move {
            direction: Direction::Up,
        },
    );
    result.map_err(|error| format!("move: {error:?}"))?;
    let professor = id(NpcId::new("professor"))?;
    let (state, result) = state.transition(content, GameCommand::Interact { npc: professor });
    result.map_err(|error| format!("gift: {error:?}"))?;
    Ok(state.party().to_vec())
}

fn continuation() -> Result<BattleContinuation, String> {
    Ok(BattleContinuation::new(
        id(MapId::new("verdant-route"))?,
        Position::new(2, 1),
    ))
}

#[test]
fn wild_request_and_result_patch_preserve_explicit_fixture_facts() -> Result<(), String> {
    let content = content()?;
    let party = starter_party(&content)?;
    let participant = party
        .first()
        .map(|creature| creature.id().clone())
        .ok_or_else(|| String::from("missing starter"))?;
    let request = BattleStartRequest::new(
        &content,
        id(BattleId::new("route-wild"))?,
        BattleSource::Wild { encounter_roll: 7 },
        party,
        participant.clone(),
        0xC0DE_0001,
        continuation()?,
    )
    .map_err(|error| format!("request: {error:?}"))?;

    assert_eq!(request.content_version(), "thin-slice-v3");
    assert_eq!(request.ruleset().storage_key(), "legacy-gen3@1");
    assert_eq!(request.participant(), &participant);
    assert_eq!(request.random_seed(), 0xC0DE_0001);
    let context =
        ActiveBattleContext::begin(None, request).map_err(|error| format!("context: {error:?}"))?;
    let patch = BattleResultPatch::from_context(&content, &context, BattleOutcome::Victory, 28, 24)
        .map_err(|error| format!("patch: {error:?}"))?;

    assert_eq!(patch.battle().as_str(), "route-wild");
    assert_eq!(patch.ruleset(), context.request().ruleset());
    assert_eq!(patch.participant().creature(), &participant);
    assert_eq!(patch.participant().hp(), 28);
    assert_eq!(patch.participant().pp(), 24);
    assert_eq!(patch.experience_reward(), 20);
    assert_eq!(patch.money_reward(), Money::new(0));
    assert_eq!(patch.defeated_trainer(), None);
    Ok(())
}

#[test]
fn trainer_result_patch_carries_the_content_defined_completion_reward() -> Result<(), String> {
    let content = content()?;
    let party = starter_party(&content)?;
    let participant = party
        .first()
        .map(|creature| creature.id().clone())
        .ok_or_else(|| String::from("missing starter"))?;
    let request = BattleStartRequest::new(
        &content,
        id(BattleId::new("route-trainer-battle"))?,
        BattleSource::Trainer {
            npc: id(NpcId::new("route-trainer"))?,
            trainer: id(TrainerId::new("route-rival"))?,
        },
        party,
        participant,
        0xC0DE_0002,
        continuation()?,
    )
    .map_err(|error| format!("request: {error:?}"))?;
    let context =
        ActiveBattleContext::begin(None, request).map_err(|error| format!("context: {error:?}"))?;
    let patch = BattleResultPatch::from_context(&content, &context, BattleOutcome::Victory, 19, 16)
        .map_err(|error| format!("patch: {error:?}"))?;

    assert_eq!(patch.experience_reward(), 45);
    assert_eq!(patch.money_reward(), Money::new(120));
    assert_eq!(
        patch.defeated_trainer().map(NpcId::as_str),
        Some("route-trainer")
    );
    Ok(())
}

#[test]
fn rejected_requests_and_patches_leave_the_existing_context_unchanged() -> Result<(), String> {
    let content = content()?;
    let party = starter_party(&content)?;
    let participant = party
        .first()
        .map(|creature| creature.id().clone())
        .ok_or_else(|| String::from("missing starter"))?;
    let valid = BattleStartRequest::new(
        &content,
        id(BattleId::new("route-wild"))?,
        BattleSource::Wild { encounter_roll: 7 },
        party.clone(),
        participant.clone(),
        0xC0DE_0003,
        continuation()?,
    )
    .map_err(|error| format!("request: {error:?}"))?;
    let context = ActiveBattleContext::begin(None, valid.clone())
        .map_err(|error| format!("context: {error:?}"))?;
    let before = context.clone();

    assert_eq!(
        ActiveBattleContext::begin(Some(&context), valid),
        Err(BattleContractError::BattleAlreadyActive)
    );
    assert_eq!(context, before);
    assert_eq!(
        BattleResultPatch::from_context(&content, &context, BattleOutcome::Victory, 36, 24),
        Err(BattleContractError::InvalidParticipantState {
            creature: participant.clone(),
            hp: 36,
            pp: 24,
        })
    );
    assert_eq!(context, before);

    let invalid = BattleStartRequest::new(
        &content,
        id(BattleId::new("route-wild"))?,
        BattleSource::Wild {
            encounter_roll: 100,
        },
        party,
        participant,
        0xC0DE_0004,
        continuation()?,
    );
    assert_eq!(invalid, Err(BattleContractError::InvalidEncounterRoll(100)));
    Ok(())
}

#[test]
fn request_rejects_unknown_content_references() -> Result<(), String> {
    let content = content()?;
    let party = starter_party(&content)?;
    let participant = party
        .first()
        .map(|creature| creature.id().clone())
        .ok_or_else(|| String::from("missing starter"))?;
    let missing = id(BattleId::new("missing-battle"))?;
    assert_eq!(
        BattleStartRequest::new(
            &content,
            missing.clone(),
            BattleSource::Wild { encounter_roll: 7 },
            party,
            participant,
            0xC0DE_0005,
            continuation()?,
        ),
        Err(BattleContractError::UnknownBattle(missing))
    );

    let unknown_npc = id(NpcId::new("missing-npc"))?;
    assert_eq!(
        BattleStartRequest::new(
            &content,
            id(BattleId::new("route-trainer-battle"))?,
            BattleSource::Trainer {
                npc: unknown_npc.clone(),
                trainer: id(TrainerId::new("route-rival"))?,
            },
            starter_party(&content)?,
            id(CreatureId::new("starter-treecko-1"))?,
            0xC0DE_0006,
            continuation()?,
        ),
        Err(BattleContractError::UnknownNpc(unknown_npc))
    );
    Ok(())
}
