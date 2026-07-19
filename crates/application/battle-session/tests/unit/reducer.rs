use super::*;

fn combatant(id: &str, hp: u32) -> CombatantScene {
    CombatantScene {
        id: PokemonId::new(id).unwrap(),
        name: id.into(),
        level: 50,
        primary_type: PokemonType::Normal,
        secondary_type: None,
        current_hp: hp,
        max_hp: 100,
        substitute_hp: None,
        major_status: None,
        stages: StatStages::neutral(),
        condition: condition(hp),
    }
}

fn scene() -> BattleScene {
    BattleScene {
        own: combatant("own", 100),
        opponent: combatant("opponent", 100),
        weather: None,
    }
}

#[test]
fn reducer_rejects_inactive_targets_fainting_with_hp_and_final_mismatch() {
    let mut reducer = BattleSceneReducer { scene: scene() };
    let inactive = reducer
        .apply(&BattleEvent::MoveUsed {
            participant: Participant::Own,
            pokemon: PokemonId::new("bench").unwrap(),
            used_move: UsedMove::Struggle,
        })
        .unwrap_err();
    assert!(matches!(
        inactive,
        ReplayError::EventTargetsInactivePokemon { .. }
    ));

    let fainted = reducer
        .apply(&BattleEvent::Fainted {
            participant: Participant::Opponent,
            pokemon: PokemonId::new("opponent").unwrap(),
        })
        .unwrap_err();
    assert!(matches!(fainted, ReplayError::FaintedWithHp { .. }));

    let critical = reducer
        .apply(&BattleEvent::Critical {
            participant: Participant::Opponent,
            target: Participant::Own,
            pokemon: PokemonId::new("own").unwrap(),
        })
        .unwrap()
        .unwrap();
    assert!(matches!(
        critical.cue(),
        BattleCue::Critical {
            participant: Participant::Own
        }
    ));

    let mut expected = scene();
    expected.own.current_hp = 99;
    let mismatch = reduce_events(scene(), &[], expected).unwrap_err();
    assert!(matches!(mismatch, ReplayError::FinalSceneMismatch { .. }));
    assert_eq!(reducer.scene.own.condition(), CombatantCondition::Able);
}
