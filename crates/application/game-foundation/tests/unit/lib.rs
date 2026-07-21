use crate::{
    BattleId, BattleOutcome, BattleResolution, ContentPackage, ContentPackageDocument,
    ContentPackageError, ContentPackageId, ContentPackageManifest, EMBEDDED_GEN3_DATA_REFERENCE,
    GameCommand, GameError, GameState, ItemId, LEGACY_GEN3_RULESET_REFERENCE, Money, NpcId,
    SaveEnvelope, ThinSliceContent, TrainerCatalog, TrainerEditCommand, TrainerId, TrainerPokemon,
    WarpId,
};

fn apply(
    state: GameState,
    content: &ThinSliceContent,
    command: GameCommand,
) -> Result<GameState, String> {
    let (state, result) = state.transition(content, command);
    result
        .map(|_| state)
        .map_err(|error| format!("command rejected: {error:?}"))
}

fn npc(value: &str) -> Result<NpcId, String> {
    NpcId::new(value).map_err(|error| format!("npc id: {error:?}"))
}

fn item(value: &str) -> Result<ItemId, String> {
    ItemId::new(value).map_err(|error| format!("item id: {error:?}"))
}

fn warp(value: &str) -> Result<WarpId, String> {
    WarpId::new(value).map_err(|error| format!("warp id: {error:?}"))
}

#[test]
fn thin_slice_round_trip_preserves_the_complete_player_state() -> Result<(), String> {
    let content = ThinSliceContent::standard().map_err(|error| format!("content: {error:?}"))?;
    let potion = item("potion")?;
    let trainer = npc("route-trainer")?;
    let state = GameState::new(&content).map_err(|error| format!("state: {error:?}"))?;
    let state = apply(state, &content, GameCommand::NewGame)?;
    let state = apply(
        state,
        &content,
        GameCommand::Move {
            direction: crate::Direction::Up,
        },
    )?;
    let state = apply(
        state,
        &content,
        GameCommand::Interact {
            npc: npc("professor")?,
        },
    )?;
    assert_eq!(state.party().len(), 1);
    assert_eq!(state.inventory().quantity(&potion), 1);

    let state = apply(
        state,
        &content,
        GameCommand::Warp {
            warp: warp("town-to-route")?,
        },
    )?;
    let state = apply(
        state,
        &content,
        GameCommand::Move {
            direction: crate::Direction::Right,
        },
    )?;
    let state = apply(state, &content, GameCommand::Encounter { roll: 7 })?;
    let state = apply(
        state,
        &content,
        GameCommand::ResolveBattle {
            outcome: BattleOutcome::Victory,
            hp: 28,
            pp: 24,
        },
    )?;
    assert_eq!(state.party()[0].hp(), 28);
    assert_eq!(state.party()[0].pp(), 24);
    assert_eq!(state.party()[0].experience(), 20);
    assert_eq!(state.money(), Money::new(200));

    let state = apply(
        state,
        &content,
        GameCommand::Move {
            direction: crate::Direction::Down,
        },
    )?;
    let state = apply(
        state,
        &content,
        GameCommand::Move {
            direction: crate::Direction::Right,
        },
    )?;
    let state = apply(
        state,
        &content,
        GameCommand::Interact {
            npc: trainer.clone(),
        },
    )?;
    assert_eq!(
        state.last_message(),
        Some("前方是训练家的道路。准备好就来对战吧。")
    );
    let state = apply(
        state,
        &content,
        GameCommand::ResolveBattle {
            outcome: BattleOutcome::Victory,
            hp: 19,
            pp: 16,
        },
    )?;
    assert!(state.defeated_trainers().contains(&trainer));
    assert_eq!(state.party()[0].experience(), 65);
    assert_eq!(state.money(), Money::new(320));

    let state = apply(
        state,
        &content,
        GameCommand::Warp {
            warp: warp("route-to-town")?,
        },
    )?;
    let state = apply(
        state,
        &content,
        GameCommand::Move {
            direction: crate::Direction::Up,
        },
    )?;
    let state = apply(
        state,
        &content,
        GameCommand::Buy {
            npc: npc("merchant")?,
            item: potion.clone(),
            quantity: 1,
        },
    )?;
    assert_eq!(state.inventory().quantity(&potion), 2);
    assert_eq!(state.money(), Money::new(290));

    let envelope = SaveEnvelope::from_state(&content, state.clone())
        .map_err(|error| format!("save: {error:?}"))?;
    let bytes = envelope
        .to_json()
        .map_err(|error| format!("encode: {error:?}"))?;
    let loaded =
        SaveEnvelope::from_json(&content, &bytes).map_err(|error| format!("load: {error:?}"))?;
    assert_eq!(loaded.state(), &state);
    Ok(())
}

#[test]
fn rejected_command_does_not_mutate_state() -> Result<(), String> {
    let content = ThinSliceContent::standard().map_err(|error| format!("content: {error:?}"))?;
    let state = GameState::new(&content).map_err(|error| format!("state: {error:?}"))?;
    let before = state.clone();
    let (after, result) = state.transition(
        &content,
        GameCommand::Warp {
            warp: warp("town-to-route")?,
        },
    );
    assert!(matches!(result, Err(GameError::PartyRequired)));
    assert_eq!(after, before);
    Ok(())
}

#[test]
fn rejected_battle_resolution_preserves_the_active_battle_and_player_state() -> Result<(), String> {
    let content = ThinSliceContent::standard().map_err(|error| format!("content: {error:?}"))?;
    let state = GameState::new(&content).map_err(|error| format!("state: {error:?}"))?;
    let state = apply(
        state,
        &content,
        GameCommand::Move {
            direction: crate::Direction::Up,
        },
    )?;
    let state = apply(
        state,
        &content,
        GameCommand::Interact {
            npc: npc("professor")?,
        },
    )?;
    let state = apply(
        state,
        &content,
        GameCommand::Warp {
            warp: warp("town-to-route")?,
        },
    )?;
    let state = apply(
        state,
        &content,
        GameCommand::Move {
            direction: crate::Direction::Right,
        },
    )?;
    let state = apply(state, &content, GameCommand::Encounter { roll: 7 })?;
    let participant = state
        .active_battle()
        .map(|active| active.participant().clone())
        .ok_or_else(|| String::from("missing active battle"))?;
    let before = state.clone();
    let (after, result) = state.apply_battle_resolution(
        &content,
        BattleResolution::new(
            BattleId::new("route-trainer-battle")
                .map_err(|error| format!("battle id: {error:?}"))?,
            participant,
            BattleOutcome::Victory,
            28,
            24,
        ),
    );
    assert!(matches!(result, Err(GameError::BattleMismatch { .. })));
    assert_eq!(after, before);
    Ok(())
}

#[test]
fn trainer_cannot_be_completed_twice() -> Result<(), String> {
    let content = ThinSliceContent::standard().map_err(|error| format!("content: {error:?}"))?;
    let trainer = npc("route-trainer")?;
    let state = GameState::new(&content).map_err(|error| format!("state: {error:?}"))?;
    let state = apply(state, &content, GameCommand::NewGame)?;
    let state = apply(
        state,
        &content,
        GameCommand::Move {
            direction: crate::Direction::Up,
        },
    )?;
    let state = apply(
        state,
        &content,
        GameCommand::Interact {
            npc: npc("professor")?,
        },
    )?;
    let state = apply(
        state,
        &content,
        GameCommand::Warp {
            warp: warp("town-to-route")?,
        },
    )?;
    let state = apply(
        state,
        &content,
        GameCommand::Move {
            direction: crate::Direction::Right,
        },
    )?;
    let state = apply(state, &content, GameCommand::Encounter { roll: 7 })?;
    let state = apply(
        state,
        &content,
        GameCommand::ResolveBattle {
            outcome: BattleOutcome::Victory,
            hp: 20,
            pp: 20,
        },
    )?;
    let state = apply(
        state,
        &content,
        GameCommand::Move {
            direction: crate::Direction::Down,
        },
    )?;
    let state = apply(
        state,
        &content,
        GameCommand::Move {
            direction: crate::Direction::Right,
        },
    )?;
    let state = apply(
        state,
        &content,
        GameCommand::Interact {
            npc: trainer.clone(),
        },
    )?;
    let state = apply(
        state,
        &content,
        GameCommand::ResolveBattle {
            outcome: BattleOutcome::Victory,
            hp: 20,
            pp: 20,
        },
    )?;
    let before = state.clone();
    let (after, result) = state.transition(&content, GameCommand::Interact { npc: trainer });
    assert!(matches!(result, Err(GameError::TrainerAlreadyDefeated(_))));
    assert_eq!(after, before);
    Ok(())
}

#[test]
fn trainer_catalog_edits_name_pokemon_and_script_transactionally() -> Result<(), String> {
    let catalog = TrainerCatalog::standard().map_err(|error| error.to_string())?;
    let trainer =
        TrainerId::new("route-rival").map_err(|error| format!("trainer id: {error:?}"))?;
    let catalog = catalog
        .transition(TrainerEditCommand::SetName {
            trainer: trainer.clone(),
            name: String::from("短裤小子 阿健"),
        })
        .map_err(|error| error.to_string())?;
    let catalog = catalog
        .transition(TrainerEditCommand::AddPokemon {
            trainer: trainer.clone(),
            pokemon: TrainerPokemon::new("Poochyena", 6).map_err(|error| error.to_string())?,
        })
        .map_err(|error| error.to_string())?;
    let catalog = catalog
        .transition(TrainerEditCommand::SetScript {
            trainer: trainer.clone(),
            script: String::from("我的宝可梦绝不会输。"),
        })
        .map_err(|error| error.to_string())?;
    let definition = catalog
        .trainer(&trainer)
        .ok_or_else(|| String::from("missing edited trainer"))?;
    assert_eq!(definition.name(), "短裤小子 阿健");
    assert_eq!(definition.pokemon().len(), 2);
    assert_eq!(
        definition.pokemon().get(1).map(TrainerPokemon::species),
        Some("Poochyena")
    );
    assert_eq!(definition.script(), "我的宝可梦绝不会输。");

    let json = catalog
        .to_json_pretty()
        .map_err(|error| error.to_string())?;
    let decoded = TrainerCatalog::from_json(&json).map_err(|error| error.to_string())?;
    assert_eq!(decoded, catalog);

    let result = catalog.transition(TrainerEditCommand::RemovePokemon { trainer, slot: 8 });
    assert!(result.is_err());
    Ok(())
}

#[test]
fn content_package_binds_static_content_to_a_versioned_manifest() -> Result<(), String> {
    let package = ContentPackage::standard().map_err(|error| format!("package: {error:?}"))?;
    assert_eq!(package.manifest().storage_key(), "starter-region@1");
    assert_eq!(
        package.manifest().content_version(),
        package.content().content_version()
    );

    let invalid_revision = ContentPackageManifest::new(
        ContentPackageId::new("alternate").map_err(|error| format!("id: {error:?}"))?,
        0,
        package.content().content_version(),
        EMBEDDED_GEN3_DATA_REFERENCE,
        LEGACY_GEN3_RULESET_REFERENCE,
    );
    assert!(matches!(
        invalid_revision,
        Err(ContentPackageError::InvalidRevision(0))
    ));

    let mismatched_manifest = ContentPackageManifest::new(
        ContentPackageId::new("alternate").map_err(|error| format!("id: {error:?}"))?,
        1,
        "another-content-version",
        EMBEDDED_GEN3_DATA_REFERENCE,
        LEGACY_GEN3_RULESET_REFERENCE,
    )
    .map_err(|error| format!("manifest: {error:?}"))?;
    let mismatched_content =
        ThinSliceContent::standard().map_err(|error| format!("content: {error:?}"))?;
    assert!(matches!(
        ContentPackage::new(mismatched_manifest, mismatched_content),
        Err(ContentPackageError::ContentVersionMismatch { .. })
    ));
    Ok(())
}

#[test]
fn starter_region_json_document_loads_the_same_static_content() -> Result<(), String> {
    let document = ContentPackageDocument::from_json(include_str!(
        "../../../../../assets/content/starter-region/content.json"
    ))
    .map_err(|error| format!("document: {error:?}"))?;
    let package = document
        .into_package()
        .map_err(|error| format!("package: {error:?}"))?;
    let standard = ThinSliceContent::standard().map_err(|error| format!("standard: {error:?}"))?;
    assert_eq!(package.manifest().storage_key(), "starter-region@1");
    assert_eq!(package.content(), &standard);
    Ok(())
}
