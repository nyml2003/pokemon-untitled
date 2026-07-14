use super::*;

fn team_slot(index: usize) -> TeamSlot {
    TeamSlot::new(index).unwrap()
}

fn move_slot(index: usize) -> MoveSlot {
    MoveSlot::new(index).unwrap()
}

fn battle_move(id: &str, move_type: PokemonType, power: u16, pp: u8, priority: i8) -> Move {
    battle_move_with_accuracy(id, move_type, power, Accuracy::AlwaysHit, pp, priority)
}

fn battle_move_with_accuracy(
    id: &str,
    move_type: PokemonType,
    power: u16,
    accuracy: Accuracy,
    pp: u8,
    priority: i8,
) -> Move {
    Move::new(
        MoveId::new(id).unwrap(),
        id,
        move_type,
        power,
        accuracy,
        pp.max(1),
        pp,
        priority,
    )
    .unwrap()
}

#[allow(clippy::too_many_arguments)]
fn pokemon_with_hp(
    id: &str,
    primary: PokemonType,
    secondary: Option<PokemonType>,
    max_hp: u32,
    current_hp: u32,
    attack: u16,
    defense: u16,
    special_attack: u16,
    special_defense: u16,
    speed: u16,
    moves: Vec<Move>,
) -> Pokemon {
    Pokemon::new(
        PokemonId::new(id).unwrap(),
        id,
        50,
        primary,
        secondary,
        max_hp,
        current_hp,
        BattleStats::new(attack, defense, special_attack, special_defense, speed).unwrap(),
        moves,
    )
    .unwrap()
}

#[allow(clippy::too_many_arguments)]
fn pokemon(
    id: &str,
    primary: PokemonType,
    secondary: Option<PokemonType>,
    hp: u32,
    attack: u16,
    defense: u16,
    special_attack: u16,
    special_defense: u16,
    speed: u16,
    moves: Vec<Move>,
) -> Pokemon {
    pokemon_with_hp(
        id,
        primary,
        secondary,
        hp,
        hp,
        attack,
        defense,
        special_attack,
        special_defense,
        speed,
        moves,
    )
}

fn team(prefix: &str, lead: Pokemon) -> Team {
    let mut members = vec![lead];
    for index in 1..TEAM_SIZE {
        members.push(pokemon(
            &format!("{prefix}-{index}"),
            PokemonType::Normal,
            None,
            100,
            50,
            50,
            50,
            50,
            10,
            vec![battle_move(
                &format!("{prefix}-move-{index}"),
                PokemonType::Normal,
                40,
                10,
                0,
            )],
        ));
    }
    Team::new(members).unwrap()
}

fn team_with_only_lead_alive(prefix: &str, lead: Pokemon) -> Team {
    let mut members = vec![lead];
    for index in 1..TEAM_SIZE {
        members.push(pokemon_with_hp(
            &format!("{prefix}-{index}"),
            PokemonType::Normal,
            None,
            1,
            0,
            1,
            1,
            1,
            1,
            1,
            vec![battle_move(
                &format!("{prefix}-move-{index}"),
                PokemonType::Normal,
                1,
                1,
                0,
            )],
        ));
    }
    Team::new(members).unwrap()
}

fn fainted_team(prefix: &str) -> Team {
    let members = (0..TEAM_SIZE)
        .map(|index| {
            pokemon_with_hp(
                &format!("{prefix}-{index}"),
                PokemonType::Normal,
                None,
                1,
                0,
                1,
                1,
                1,
                1,
                1,
                vec![battle_move(
                    &format!("{prefix}-move-{index}"),
                    PokemonType::Normal,
                    1,
                    1,
                    0,
                )],
            )
        })
        .collect();
    Team::new(members).unwrap()
}

fn basic_battle(seed: u64, one_speed: u16, two_speed: u16) -> Battle {
    let one = pokemon(
        "one-lead",
        PokemonType::Normal,
        None,
        200,
        100,
        100,
        100,
        100,
        one_speed,
        vec![battle_move("tackle-one", PokemonType::Normal, 40, 10, 0)],
    );
    let two = pokemon(
        "two-lead",
        PokemonType::Normal,
        None,
        200,
        100,
        100,
        100,
        100,
        two_speed,
        vec![battle_move("tackle-two", PokemonType::Normal, 40, 10, 0)],
    );
    Battle::new(team("one", one), team("two", two), seed).unwrap()
}

fn submit_turn(battle: &mut Battle, one: Action, two: Action) -> Vec<BattleEvent> {
    battle.submit(BattleCommand::new(Side::One, one)).unwrap();
    battle
        .submit(BattleCommand::new(Side::Two, two))
        .unwrap()
        .events()
        .to_vec()
}

#[test]
fn constructors_enforce_exact_team_and_input_invariants() {
    assert_eq!(
        Team::new(Vec::new()),
        Err(ValidationError::InvalidTeamSize { count: 0 })
    );
    assert_eq!(
        TeamSlot::new(TEAM_SIZE),
        Err(ValidationError::InvalidTeamSlot { index: TEAM_SIZE })
    );
    assert_eq!(
        MoveSlot::new(MAX_MOVES),
        Err(ValidationError::InvalidMoveSlot { index: MAX_MOVES })
    );
    assert_eq!(
        BattleStats::new(0, 1, 1, 1, 1),
        Err(ValidationError::ZeroStat { stat: "attack" })
    );
    let invalid = Pokemon::new(
        PokemonId::new("invalid").unwrap(),
        "invalid",
        0,
        PokemonType::Fire,
        None,
        1,
        1,
        BattleStats::new(1, 1, 1, 1, 1).unwrap(),
        vec![battle_move("move", PokemonType::Fire, 1, 1, 0)],
    );
    assert_eq!(invalid, Err(ValidationError::InvalidLevel { level: 0 }));
}

#[test]
fn value_objects_reject_every_invalid_boundary() {
    assert_eq!(PokemonId::new("  "), Err(ValidationError::EmptyPokemonId));
    assert_eq!(MoveId::new(""), Err(ValidationError::EmptyMoveId));
    assert_eq!(
        Accuracy::percent(0),
        Err(ValidationError::InvalidAccuracy { value: 0 })
    );
    assert_eq!(
        Accuracy::percent(101),
        Err(ValidationError::InvalidAccuracy { value: 101 })
    );
    assert_eq!(Accuracy::percent(100), Ok(Accuracy::Percent(100)));

    for (stats, stat) in [
        ((0, 1, 1, 1, 1), "attack"),
        ((1, 0, 1, 1, 1), "defense"),
        ((1, 1, 0, 1, 1), "special_attack"),
        ((1, 1, 1, 0, 1), "special_defense"),
        ((1, 1, 1, 1, 0), "speed"),
    ] {
        assert_eq!(
            BattleStats::new(stats.0, stats.1, stats.2, stats.3, stats.4),
            Err(ValidationError::ZeroStat { stat })
        );
    }
}

#[test]
fn move_constructor_rejects_every_invalid_field() {
    let make = |name: &str, power: u16, accuracy: Accuracy, max_pp: u8, current_pp: u8| {
        Move::new(
            MoveId::new("move").unwrap(),
            name,
            PokemonType::Normal,
            power,
            accuracy,
            max_pp,
            current_pp,
            0,
        )
    };

    assert_eq!(
        make(" ", 1, Accuracy::AlwaysHit, 1, 1),
        Err(ValidationError::EmptyMoveName)
    );
    assert_eq!(
        make("move", 0, Accuracy::AlwaysHit, 1, 1),
        Err(ValidationError::ZeroMovePower)
    );
    assert_eq!(
        make("move", 1, Accuracy::Percent(0), 1, 1),
        Err(ValidationError::InvalidAccuracy { value: 0 })
    );
    assert_eq!(
        make("move", 1, Accuracy::AlwaysHit, 0, 0),
        Err(ValidationError::ZeroMaxPp)
    );
    assert_eq!(
        make("move", 1, Accuracy::AlwaysHit, 1, 2),
        Err(ValidationError::CurrentPpExceedsMax { current: 2, max: 1 })
    );
}

#[test]
fn pokemon_constructor_rejects_every_invalid_field() {
    let valid_move = || battle_move("valid-move", PokemonType::Normal, 1, 1, 0);
    let make = |name: &str,
                level: u8,
                secondary: Option<PokemonType>,
                max_hp: u32,
                current_hp: u32,
                moves: Vec<Move>| {
        Pokemon::new(
            PokemonId::new("pokemon").unwrap(),
            name,
            level,
            PokemonType::Normal,
            secondary,
            max_hp,
            current_hp,
            BattleStats::new(1, 1, 1, 1, 1).unwrap(),
            moves,
        )
    };

    assert_eq!(
        make(" ", 1, None, 1, 1, vec![valid_move()]),
        Err(ValidationError::EmptyPokemonName)
    );
    assert_eq!(
        make("pokemon", 101, None, 1, 1, vec![valid_move()]),
        Err(ValidationError::InvalidLevel { level: 101 })
    );
    assert_eq!(
        make(
            "pokemon",
            1,
            Some(PokemonType::Normal),
            1,
            1,
            vec![valid_move()]
        ),
        Err(ValidationError::DuplicatePokemonType {
            primary_type: PokemonType::Normal
        })
    );
    assert_eq!(
        make("pokemon", 1, None, 0, 0, vec![valid_move()]),
        Err(ValidationError::ZeroMaxHp)
    );
    assert_eq!(
        make("pokemon", 1, None, 1, 2, vec![valid_move()]),
        Err(ValidationError::CurrentHpExceedsMax { current: 2, max: 1 })
    );
    assert_eq!(
        make("pokemon", 1, None, 1, 1, vec![]),
        Err(ValidationError::InvalidMoveCount { count: 0 })
    );
    assert_eq!(
        make("pokemon", 1, None, 1, 1, vec![valid_move(); MAX_MOVES + 1]),
        Err(ValidationError::InvalidMoveCount {
            count: MAX_MOVES + 1
        })
    );
    assert_eq!(
        make("pokemon", 1, None, 1, 1, vec![valid_move(), valid_move()]),
        Err(ValidationError::DuplicateMoveId {
            id: MoveId::new("valid-move").unwrap()
        })
    );
    assert!(
        make(
            "pokemon",
            1,
            None,
            1,
            1,
            vec![
                battle_move("first-move", PokemonType::Normal, 1, 1, 0),
                battle_move("second-move", PokemonType::Normal, 1, 1, 0),
            ],
        )
        .is_ok()
    );
}

#[test]
fn model_accessors_return_the_validated_values() {
    let battle_move = Move::new(
        MoveId::new("ember").unwrap(),
        "Ember",
        PokemonType::Fire,
        40,
        Accuracy::Percent(100),
        25,
        24,
        1,
    )
    .unwrap();
    let pokemon = Pokemon::new(
        PokemonId::new("charizard").unwrap(),
        "Charizard",
        50,
        PokemonType::Fire,
        Some(PokemonType::Flying),
        150,
        149,
        BattleStats::new(80, 81, 109, 85, 100).unwrap(),
        vec![battle_move.clone()],
    )
    .unwrap();

    assert_eq!(PokemonId::new("charizard").unwrap().as_str(), "charizard");
    assert_eq!(MoveId::new("ember").unwrap().as_str(), "ember");
    assert_eq!(battle_move.name(), "Ember");
    assert_eq!(battle_move.category(), MoveCategory::Special);
    assert_eq!(battle_move.max_pp(), 25);
    assert_eq!(pokemon.name(), "Charizard");
    assert_eq!(pokemon.max_hp(), 150);
    assert_eq!(pokemon.current_hp(), 149);
    assert_eq!(pokemon.stats().special_attack(), 109);
    assert_eq!(pokemon.stats().special_defense(), 85);
}

#[test]
fn team_rejects_duplicate_pokemon_ids() {
    let lead = pokemon(
        "duplicate",
        PokemonType::Normal,
        None,
        1,
        1,
        1,
        1,
        1,
        1,
        vec![battle_move("duplicate-move", PokemonType::Normal, 1, 1, 0)],
    );
    let members = (0..TEAM_SIZE).map(|_| lead.clone()).collect();

    assert_eq!(
        Team::new(members),
        Err(ValidationError::DuplicatePokemonId {
            id: PokemonId::new("duplicate").unwrap()
        })
    );
}

#[test]
fn battle_constructor_rejects_invalid_team_relationships() {
    let valid_lead = pokemon(
        "valid-lead",
        PokemonType::Normal,
        None,
        1,
        1,
        1,
        1,
        1,
        1,
        vec![battle_move("valid-lead-move", PokemonType::Normal, 1, 1, 0)],
    );
    assert_eq!(
        Battle::new(
            fainted_team("fainted-one"),
            team("valid-two", valid_lead.clone()),
            1
        ),
        Err(BattleError::NoLivingPokemon { side: Side::One })
    );
    assert_eq!(
        Battle::new(
            team("valid-one", valid_lead.clone()),
            fainted_team("fainted-two"),
            1
        ),
        Err(BattleError::NoLivingPokemon { side: Side::Two })
    );

    let shared_one = pokemon(
        "shared",
        PokemonType::Normal,
        None,
        1,
        1,
        1,
        1,
        1,
        1,
        vec![battle_move("shared-one-move", PokemonType::Normal, 1, 1, 0)],
    );
    let shared_two = pokemon(
        "shared",
        PokemonType::Normal,
        None,
        1,
        1,
        1,
        1,
        1,
        1,
        vec![battle_move("shared-two-move", PokemonType::Normal, 1, 1, 0)],
    );
    assert_eq!(
        Battle::new(
            team("shared-one", shared_one),
            team("shared-two", shared_two),
            1
        ),
        Err(BattleError::DuplicatePokemonId {
            id: PokemonId::new("shared").unwrap()
        })
    );
}

#[test]
fn every_illegal_action_reports_a_specific_reason_without_mutation() {
    let mut battle = basic_battle(9, 80, 60);
    let invalid_move = Action::UseMove(move_slot(1));
    let switch_to_active = Action::Switch(team_slot(0));
    for (action, reason) in [
        (invalid_move, IllegalActionReason::MoveDoesNotExist),
        (switch_to_active, IllegalActionReason::SwitchToActive),
    ] {
        let before = battle.clone();
        assert_eq!(
            battle.submit(BattleCommand::new(Side::One, action)),
            Err(BattleError::ActionNotLegal {
                side: Side::One,
                action,
                reason,
            })
        );
        assert_eq!(battle, before);
    }

    let empty = pokemon(
        "empty-action",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move(
            "empty-action-move",
            PokemonType::Normal,
            40,
            0,
            0,
        )],
    );
    let mut empty_battle = Battle::new(
        team("empty-action-team", empty),
        team(
            "empty-action-foe",
            pokemon(
                "empty-action-foe-lead",
                PokemonType::Normal,
                None,
                100,
                1,
                1,
                1,
                1,
                1,
                vec![battle_move(
                    "empty-action-foe-move",
                    PokemonType::Normal,
                    1,
                    1,
                    0,
                )],
            ),
        ),
        1,
    )
    .unwrap();
    assert_eq!(
        empty_battle.submit(BattleCommand::new(Side::One, Action::UseMove(move_slot(0)))),
        Err(BattleError::ActionNotLegal {
            side: Side::One,
            action: Action::UseMove(move_slot(0)),
            reason: IllegalActionReason::MoveHasNoPp,
        })
    );

    let switch_target_fainted = pokemon(
        "switch-target-lead",
        PokemonType::Normal,
        None,
        100,
        1,
        1,
        1,
        1,
        1,
        vec![battle_move(
            "switch-target-move",
            PokemonType::Normal,
            1,
            1,
            0,
        )],
    );
    let mut switch_battle = Battle::new(
        team_with_only_lead_alive("switch-target", switch_target_fainted),
        team(
            "switch-foe",
            pokemon(
                "switch-foe-lead",
                PokemonType::Normal,
                None,
                100,
                1,
                1,
                1,
                1,
                1,
                vec![battle_move("switch-foe-move", PokemonType::Normal, 1, 1, 0)],
            ),
        ),
        1,
    )
    .unwrap();
    assert_eq!(
        switch_battle.submit(BattleCommand::new(Side::One, Action::Switch(team_slot(1)))),
        Err(BattleError::ActionNotLegal {
            side: Side::One,
            action: Action::Switch(team_slot(1)),
            reason: IllegalActionReason::SwitchTargetFainted,
        })
    );
}

#[test]
fn forced_replacement_rejects_commands_from_the_wrong_side_and_non_switch_actions() {
    let killer = pokemon(
        "forced-killer",
        PokemonType::Normal,
        None,
        100,
        500,
        1,
        1,
        1,
        100,
        vec![battle_move("forced-ko", PokemonType::Normal, 500, 1, 0)],
    );
    let victim = pokemon(
        "forced-victim",
        PokemonType::Normal,
        None,
        1,
        1,
        1,
        1,
        1,
        1,
        vec![battle_move(
            "forced-victim-move",
            PokemonType::Normal,
            1,
            1,
            0,
        )],
    );
    let mut battle =
        Battle::new(team("forced-one", killer), team("forced-two", victim), 4).unwrap();
    submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );

    for (side, action) in [
        (Side::One, Action::UseMove(move_slot(0))),
        (Side::Two, Action::Struggle),
    ] {
        assert_eq!(
            battle.submit(BattleCommand::new(side, action)),
            Err(BattleError::ActionNotLegal {
                side,
                action,
                reason: IllegalActionReason::WrongPhase,
            })
        );
    }
    assert_eq!(
        battle.submit(BattleCommand::new(Side::Two, Action::Switch(team_slot(0)))),
        Err(BattleError::ActionNotLegal {
            side: Side::Two,
            action: Action::Switch(team_slot(0)),
            reason: IllegalActionReason::SwitchToActive,
        })
    );
}

#[test]
fn seeded_rolls_cover_hit_miss_critical_and_both_tie_orders() {
    let mut saw_hit = false;
    let mut saw_miss = false;
    let mut saw_critical = false;
    let mut saw_noncritical = false;
    let mut saw_one_first = false;
    let mut saw_two_first = false;

    for seed in 0..512 {
        let inaccurate = pokemon(
            "inaccurate",
            PokemonType::Normal,
            None,
            10_000,
            100,
            100,
            100,
            100,
            50,
            vec![battle_move_with_accuracy(
                "coin-flip",
                PokemonType::Normal,
                1,
                Accuracy::Percent(50),
                1,
                0,
            )],
        );
        let peer = pokemon(
            "peer",
            PokemonType::Normal,
            None,
            10_000,
            100,
            100,
            100,
            100,
            50,
            vec![battle_move("peer-move", PokemonType::Normal, 1, 1, 0)],
        );
        let mut battle =
            Battle::new(team("roll-one", inaccurate), team("roll-two", peer), seed).unwrap();
        let events = submit_turn(
            &mut battle,
            Action::UseMove(move_slot(0)),
            Action::UseMove(move_slot(0)),
        );
        let first = events.iter().find_map(|event| match event {
            BattleEvent::MoveUsed { side, .. } => Some(*side),
            _ => None,
        });
        saw_one_first |= first == Some(Side::One);
        saw_two_first |= first == Some(Side::Two);
        let one_missed = events.iter().any(|event| {
            matches!(
                event,
                BattleEvent::Missed {
                    side: Side::One,
                    ..
                }
            )
        });
        let one_critical = events.iter().any(|event| {
            matches!(
                event,
                BattleEvent::Critical {
                    side: Side::One,
                    ..
                }
            )
        });
        saw_miss |= one_missed;
        saw_hit |= !one_missed;
        saw_critical |= !one_missed && one_critical;
        saw_noncritical |= !one_missed && !one_critical;

        if saw_hit && saw_miss && saw_critical && saw_noncritical && saw_one_first && saw_two_first
        {
            break;
        }
    }

    assert!(saw_hit && saw_miss && saw_critical && saw_noncritical);
    assert!(saw_one_first && saw_two_first);
}

#[test]
fn opponent_does_not_act_after_faster_struggle_user_faints_from_recoil() {
    let struggler = pokemon(
        "recoil-faint-user",
        PokemonType::Normal,
        None,
        1,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move("recoil-empty", PokemonType::Normal, 50, 0, 0)],
    );
    let opponent = pokemon(
        "recoil-opponent",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move("queued-move", PokemonType::Normal, 40, 1, 0)],
    );
    let mut battle = Battle::new(
        team("recoil-user-team", struggler),
        team("recoil-opponent-team", opponent),
        31,
    )
    .unwrap();

    let events = submit_turn(&mut battle, Action::Struggle, Action::UseMove(move_slot(0)));

    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, BattleEvent::MoveUsed { .. }))
            .count(),
        1
    );
    assert_eq!(battle.active(Side::Two).moves()[0].current_pp(), 1);
    assert_eq!(
        battle.phase(),
        BattlePhase::ForcedReplacement(ReplacementSides::One)
    );
}

#[test]
fn immune_hits_never_report_a_visible_critical() {
    for seed in 0..512 {
        let attacker = pokemon(
            "immune-event-attacker",
            PokemonType::Normal,
            None,
            100,
            100,
            100,
            100,
            100,
            100,
            vec![battle_move(
                "immune-normal-hit",
                PokemonType::Normal,
                40,
                1,
                0,
            )],
        );
        let defender = pokemon(
            "immune-event-defender",
            PokemonType::Ghost,
            None,
            100,
            100,
            100,
            100,
            100,
            1,
            vec![battle_move("immune-peer-hit", PokemonType::Ghost, 1, 1, 0)],
        );
        let mut battle = Battle::new(
            team("immune-event-one", attacker),
            team("immune-event-two", defender),
            seed,
        )
        .unwrap();

        let events = submit_turn(
            &mut battle,
            Action::UseMove(move_slot(0)),
            Action::UseMove(move_slot(0)),
        );

        assert!(!events.iter().any(|event| matches!(
            event,
            BattleEvent::Critical {
                side: Side::One,
                ..
            }
        )));
    }
}

#[test]
fn gen_three_categories_and_type_chart_cover_key_edges() {
    assert_eq!(
        damage_category(PokemonType::Ghost),
        DamageCategory::Physical
    );
    assert_eq!(damage_category(PokemonType::Dark), DamageCategory::Special);
    assert_eq!(
        type_effectiveness(PokemonType::Normal, PokemonType::Ghost, None),
        TypeEffectiveness::Immune
    );
    assert_eq!(
        type_effectiveness(
            PokemonType::Ice,
            PokemonType::Dragon,
            Some(PokemonType::Flying)
        ),
        TypeEffectiveness::Quadruple
    );
    assert_eq!(
        type_effectiveness(
            PokemonType::Fire,
            PokemonType::Water,
            Some(PokemonType::Dragon)
        ),
        TypeEffectiveness::Quarter
    );
    assert_eq!(
        type_effectiveness(
            PokemonType::Ground,
            PokemonType::Electric,
            Some(PokemonType::Flying)
        ),
        TypeEffectiveness::Immune
    );
    assert_eq!(
        type_effectiveness(PokemonType::Ghost, PokemonType::Steel, None),
        TypeEffectiveness::Half
    );
    assert_eq!(
        type_effectiveness(PokemonType::Fire, PokemonType::Grass, None),
        TypeEffectiveness::Double
    );
}

#[test]
fn identical_seed_state_and_commands_replay_exactly() {
    let mut first = basic_battle(42, 80, 60);
    let mut second = basic_battle(42, 80, 60);
    for _ in 0..3 {
        submit_turn(
            &mut first,
            Action::UseMove(move_slot(0)),
            Action::UseMove(move_slot(0)),
        );
        submit_turn(
            &mut second,
            Action::UseMove(move_slot(0)),
            Action::UseMove(move_slot(0)),
        );
    }
    assert_eq!(first, second);
    assert_eq!(first.events(), second.events());
}

#[test]
fn approved_replay_fixture_locks_the_complete_seeded_event_log() {
    let mut battle = basic_battle(42, 80, 60);

    let events = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );

    let one_move = UsedMove::Move {
        slot: move_slot(0),
        id: MoveId::new("tackle-one").unwrap(),
    };
    let two_move = UsedMove::Move {
        slot: move_slot(0),
        id: MoveId::new("tackle-two").unwrap(),
    };
    assert_eq!(
        events,
        vec![
            BattleEvent::CommandAccepted {
                side: Side::One,
                action: Action::UseMove(move_slot(0)),
            },
            BattleEvent::CommandAccepted {
                side: Side::Two,
                action: Action::UseMove(move_slot(0)),
            },
            BattleEvent::TurnStarted { turn: 1 },
            BattleEvent::MoveUsed {
                side: Side::One,
                pokemon: PokemonId::new("one-lead").unwrap(),
                used_move: one_move.clone(),
            },
            BattleEvent::PpSpent {
                side: Side::One,
                pokemon: PokemonId::new("one-lead").unwrap(),
                move_slot: move_slot(0),
                remaining: 9,
            },
            BattleEvent::Effectiveness {
                side: Side::One,
                target_side: Side::Two,
                target: PokemonId::new("two-lead").unwrap(),
                effectiveness: TypeEffectiveness::Normal,
            },
            BattleEvent::Damage {
                source: DamageSource::Move {
                    side: Side::One,
                    pokemon: PokemonId::new("one-lead").unwrap(),
                    used_move: one_move,
                },
                target_side: Side::Two,
                target: PokemonId::new("two-lead").unwrap(),
                amount: 24,
                remaining_hp: 176,
            },
            BattleEvent::MoveUsed {
                side: Side::Two,
                pokemon: PokemonId::new("two-lead").unwrap(),
                used_move: two_move.clone(),
            },
            BattleEvent::PpSpent {
                side: Side::Two,
                pokemon: PokemonId::new("two-lead").unwrap(),
                move_slot: move_slot(0),
                remaining: 9,
            },
            BattleEvent::Effectiveness {
                side: Side::Two,
                target_side: Side::One,
                target: PokemonId::new("one-lead").unwrap(),
                effectiveness: TypeEffectiveness::Normal,
            },
            BattleEvent::Damage {
                source: DamageSource::Move {
                    side: Side::Two,
                    pokemon: PokemonId::new("two-lead").unwrap(),
                    used_move: two_move,
                },
                target_side: Side::One,
                target: PokemonId::new("one-lead").unwrap(),
                amount: 24,
                remaining_hp: 176,
            },
        ]
    );
}

#[test]
fn rejected_command_is_fully_transactional() {
    let mut battle = basic_battle(7, 80, 60);
    let before = battle.clone();
    assert_eq!(
        battle
            .submit(BattleCommand::new(Side::One, Action::Struggle))
            .unwrap_err(),
        BattleError::ActionNotLegal {
            side: Side::One,
            action: Action::Struggle,
            reason: IllegalActionReason::StruggleNotRequired,
        }
    );
    assert_eq!(battle, before);

    battle
        .submit(BattleCommand::new(Side::One, Action::UseMove(move_slot(0))))
        .unwrap();
    let pending = battle.clone();
    assert!(
        battle
            .submit(BattleCommand::new(Side::One, Action::Switch(team_slot(1))))
            .is_err()
    );
    assert_eq!(battle, pending);
}

#[test]
fn move_priority_precedes_speed_and_equal_speed_uses_seeded_tie_break() {
    let slow_priority = pokemon(
        "priority",
        PokemonType::Normal,
        None,
        100,
        500,
        10,
        10,
        10,
        1,
        vec![battle_move("quick", PokemonType::Normal, 500, 1, 1)],
    );
    let fast = pokemon(
        "fast",
        PokemonType::Normal,
        None,
        100,
        500,
        10,
        10,
        10,
        500,
        vec![battle_move("normal", PokemonType::Normal, 500, 1, 0)],
    );
    let mut battle = Battle::new(team("p", slow_priority), team("f", fast), 1).unwrap();
    let events = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(
        events.iter().find_map(|event| match event {
            BattleEvent::MoveUsed { side, .. } => Some(*side),
            _ => None,
        }),
        Some(Side::One)
    );

    let mut tie_one = basic_battle(1234, 60, 60);
    let mut tie_two = basic_battle(1234, 60, 60);
    submit_turn(
        &mut tie_one,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    submit_turn(
        &mut tie_two,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(tie_one.events(), tie_two.events());
}

#[test]
fn speed_orders_equal_priority_moves() {
    let mut battle = basic_battle(91, 50, 100);
    let events = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(
        events.iter().find_map(|event| match event {
            BattleEvent::MoveUsed { side, .. } => Some(*side),
            _ => None,
        }),
        Some(Side::Two)
    );
}

#[test]
fn switch_resolves_before_move_and_move_hits_new_active() {
    let mut battle = basic_battle(99, 1, 100);
    let old_id = battle.active(Side::One).id().clone();
    let new_id = battle.team(Side::One).member(team_slot(1)).id().clone();
    let events = submit_turn(
        &mut battle,
        Action::Switch(team_slot(1)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(battle.active(Side::One).id(), &new_id);
    assert_eq!(
        battle.team(Side::One).member(team_slot(0)).current_hp(),
        200
    );
    assert!(battle.active(Side::One).current_hp() < 100);
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::Switched { pokemon, .. } if pokemon == &new_id
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        BattleEvent::Damage { target, .. } if target == &old_id
    )));
}

#[test]
fn depleted_pp_unlocks_struggle_and_applies_recoil() {
    let empty = pokemon(
        "empty",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move("empty-move", PokemonType::Normal, 40, 0, 0)],
    );
    let passive = pokemon(
        "passive",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move("passive-move", PokemonType::Normal, 1, 1, 0)],
    );
    let mut battle = Battle::new(team("e", empty), team("p", passive), 12).unwrap();
    assert!(battle.legal_actions(Side::One).contains(&Action::Struggle));
    assert!(
        !battle
            .legal_actions(Side::One)
            .contains(&Action::UseMove(move_slot(0)))
    );
    let before = battle.active(Side::One).current_hp();
    let events = submit_turn(&mut battle, Action::Struggle, Action::UseMove(move_slot(0)));
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::Damage { source: DamageSource::Recoil { .. }, amount, .. } if *amount >= 1
    )));
    assert!(battle.active(Side::One).current_hp() < before);
    assert_eq!(battle.active(Side::One).moves()[0].current_pp(), 0);
}

#[test]
fn knockout_requires_replacement_without_extra_attack() {
    let killer = pokemon(
        "killer",
        PokemonType::Normal,
        None,
        100,
        500,
        10,
        10,
        10,
        500,
        vec![battle_move("ko", PokemonType::Normal, 500, 2, 0)],
    );
    let victim = pokemon(
        "victim",
        PokemonType::Normal,
        None,
        10,
        10,
        10,
        10,
        10,
        1,
        vec![battle_move("late", PokemonType::Normal, 500, 2, 0)],
    );
    let mut battle = Battle::new(team("k", killer), team("v", victim), 3).unwrap();
    submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(
        battle.phase(),
        BattlePhase::ForcedReplacement(ReplacementSides::Two)
    );
    assert_eq!(battle.legal_actions(Side::One), Vec::new());
    assert!(
        battle
            .legal_actions(Side::Two)
            .iter()
            .all(|action| matches!(action, Action::Switch(_)))
    );
    let turn = battle.turn_number();
    let outcome = battle
        .submit(BattleCommand::new(Side::Two, Action::Switch(team_slot(1))))
        .unwrap();
    assert_eq!(outcome.phase(), BattlePhase::Turn);
    assert_eq!(battle.phase(), BattlePhase::Turn);
    assert_eq!(battle.turn_number(), turn);
    assert_eq!(battle.active_slot(Side::Two), team_slot(1));
    assert!(
        !outcome
            .events()
            .iter()
            .any(|event| matches!(event, BattleEvent::MoveUsed { .. }))
    );
}

#[test]
fn struggle_can_knock_out_both_final_pokemon_for_a_draw() {
    let mut one_members = Vec::new();
    let mut two_members = Vec::new();
    for index in 0..TEAM_SIZE {
        let current = u32::from(index == 0);
        one_members.push(pokemon_with_hp(
            &format!("one-{index}"),
            PokemonType::Normal,
            None,
            1,
            current,
            100,
            1,
            1,
            1,
            100,
            vec![battle_move(
                &format!("one-move-{index}"),
                PokemonType::Normal,
                50,
                0,
                0,
            )],
        ));
        two_members.push(pokemon_with_hp(
            &format!("two-{index}"),
            PokemonType::Normal,
            None,
            1,
            current,
            100,
            1,
            1,
            1,
            1,
            vec![battle_move(
                &format!("two-move-{index}"),
                PokemonType::Normal,
                50,
                0,
                0,
            )],
        ));
    }
    let mut battle = Battle::new(
        Team::new(one_members).unwrap(),
        Team::new(two_members).unwrap(),
        5,
    )
    .unwrap();
    submit_turn(&mut battle, Action::Struggle, Action::Struggle);
    assert_eq!(battle.phase(), BattlePhase::Finished(BattleOutcome::Draw));
    assert!(battle.legal_actions(Side::One).is_empty());
    assert!(battle.legal_actions(Side::Two).is_empty());
    let before = battle.clone();
    assert!(
        battle
            .submit(BattleCommand::new(Side::One, Action::Struggle))
            .is_err()
    );
    assert_eq!(battle, before);
}

#[test]
fn final_knockout_reports_a_winner() {
    let winner = pokemon(
        "winner",
        PokemonType::Normal,
        None,
        100,
        500,
        10,
        10,
        10,
        100,
        vec![battle_move("winning-hit", PokemonType::Normal, 500, 1, 0)],
    );
    let mut losing_members = Vec::new();
    for index in 0..TEAM_SIZE {
        losing_members.push(pokemon_with_hp(
            &format!("loser-{index}"),
            PokemonType::Normal,
            None,
            1,
            u32::from(index == 0),
            1,
            1,
            1,
            1,
            1,
            vec![battle_move(
                &format!("losing-move-{index}"),
                PokemonType::Normal,
                1,
                1,
                0,
            )],
        ));
    }
    let mut battle = Battle::new(
        team("winner-bench", winner),
        Team::new(losing_members).unwrap(),
        10,
    )
    .unwrap();
    let events = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(
        battle.phase(),
        BattlePhase::Finished(BattleOutcome::Winner(Side::One))
    );
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::BattleFinished {
            outcome: BattleOutcome::Winner(Side::One)
        }
    )));
}

#[test]
fn faster_side_two_can_win_the_battle() {
    let one = pokemon(
        "side-one-final",
        PokemonType::Normal,
        None,
        1,
        1,
        1,
        1,
        1,
        1,
        vec![battle_move(
            "side-one-final-move",
            PokemonType::Normal,
            1,
            1,
            0,
        )],
    );
    let two = pokemon(
        "side-two-winner",
        PokemonType::Normal,
        None,
        100,
        500,
        1,
        1,
        1,
        100,
        vec![battle_move(
            "side-two-winning-move",
            PokemonType::Normal,
            500,
            1,
            0,
        )],
    );
    let mut battle = Battle::new(
        team_with_only_lead_alive("side-one-final-team", one),
        team("side-two-winning-team", two),
        15,
    )
    .unwrap();

    submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );

    assert_eq!(
        battle.phase(),
        BattlePhase::Finished(BattleOutcome::Winner(Side::Two))
    );
}

#[test]
fn side_one_can_be_the_only_side_forced_to_replace() {
    let one = pokemon(
        "side-one-victim",
        PokemonType::Normal,
        None,
        1,
        1,
        1,
        1,
        1,
        1,
        vec![battle_move(
            "side-one-victim-move",
            PokemonType::Normal,
            1,
            1,
            0,
        )],
    );
    let two = pokemon(
        "side-two-killer",
        PokemonType::Normal,
        None,
        100,
        500,
        1,
        1,
        1,
        100,
        vec![battle_move(
            "side-two-killer-move",
            PokemonType::Normal,
            500,
            1,
            0,
        )],
    );
    let mut battle = Battle::new(team("replace-one", one), team("replace-two", two), 22).unwrap();

    submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );

    assert_eq!(
        battle.phase(),
        BattlePhase::ForcedReplacement(ReplacementSides::One)
    );
    assert!(battle.phase().requires_replacement(Side::One));
    assert!(!battle.phase().requires_replacement(Side::Two));
    assert!(!BattlePhase::Turn.requires_replacement(Side::One));
    battle
        .submit(BattleCommand::new(Side::One, Action::Switch(team_slot(1))))
        .unwrap();
    assert_eq!(battle.phase(), BattlePhase::Turn);
}

#[test]
fn both_sides_must_replace_before_normal_turn_resumes() {
    let one = pokemon(
        "double-ko-one",
        PokemonType::Normal,
        None,
        1,
        100,
        1,
        1,
        1,
        50,
        vec![battle_move(
            "double-ko-one-move",
            PokemonType::Normal,
            50,
            0,
            0,
        )],
    );
    let two = pokemon(
        "double-ko-two",
        PokemonType::Normal,
        None,
        1,
        100,
        1,
        1,
        1,
        50,
        vec![battle_move(
            "double-ko-two-move",
            PokemonType::Normal,
            50,
            0,
            0,
        )],
    );
    let mut battle = Battle::new(team("double-one", one), team("double-two", two), 5).unwrap();
    submit_turn(&mut battle, Action::Struggle, Action::Struggle);
    assert_eq!(
        battle.phase(),
        BattlePhase::ForcedReplacement(ReplacementSides::Both)
    );

    let mut reverse_order = battle.clone();
    let two_first = reverse_order
        .submit(BattleCommand::new(Side::Two, Action::Switch(team_slot(1))))
        .unwrap();
    assert!(two_first.is_waiting_for_opponent());
    assert_eq!(
        reverse_order.phase(),
        BattlePhase::ForcedReplacement(ReplacementSides::Both)
    );

    let one = battle
        .submit(BattleCommand::new(Side::One, Action::Switch(team_slot(1))))
        .unwrap();
    assert!(one.is_waiting_for_opponent());
    assert!(battle.legal_actions(Side::One).is_empty());
    assert!(!battle.legal_actions(Side::Two).is_empty());
    let two = battle
        .submit(BattleCommand::new(Side::Two, Action::Switch(team_slot(1))))
        .unwrap();
    assert_eq!(battle.phase(), BattlePhase::Turn);
    assert_eq!(battle.active_slot(Side::One), team_slot(1));
    assert_eq!(battle.active_slot(Side::Two), team_slot(1));
    assert_eq!(
        two.events()
            .iter()
            .filter(|event| matches!(event, BattleEvent::Switched { .. }))
            .count(),
        2
    );
}

#[test]
fn approved_damage_vectors_lock_formula_and_modifier_order() {
    struct Vector {
        attacker: Pokemon,
        defender: Pokemon,
        power: u16,
        move_type: Option<PokemonType>,
        category: DamageCategory,
        critical: bool,
        random_percent: u8,
        expected: u64,
    }

    let neutral_move = |id: &str| vec![battle_move(id, PokemonType::Normal, 1, 1, 0)];
    let vectors = [
        Vector {
            attacker: pokemon(
                "fire-attacker",
                PokemonType::Fire,
                None,
                100,
                120,
                100,
                120,
                100,
                100,
                neutral_move("fire-attacker-move"),
            ),
            defender: pokemon(
                "grass-steel-defender",
                PokemonType::Grass,
                Some(PokemonType::Steel),
                100,
                100,
                100,
                100,
                100,
                100,
                neutral_move("grass-steel-defender-move"),
            ),
            power: 100,
            move_type: Some(PokemonType::Fire),
            category: DamageCategory::Special,
            critical: false,
            random_percent: 100,
            expected: 324,
        },
        Vector {
            attacker: pokemon(
                "truncation-attacker",
                PokemonType::Fire,
                None,
                100,
                120,
                100,
                120,
                100,
                100,
                neutral_move("truncation-attacker-move"),
            ),
            defender: pokemon(
                "water-dragon-defender",
                PokemonType::Water,
                Some(PokemonType::Dragon),
                100,
                100,
                100,
                100,
                100,
                100,
                neutral_move("water-dragon-defender-move"),
            ),
            power: 100,
            move_type: Some(PokemonType::Fire),
            category: DamageCategory::Special,
            critical: false,
            random_percent: 85,
            expected: 17,
        },
        Vector {
            attacker: pokemon(
                "critical-attacker",
                PokemonType::Normal,
                None,
                100,
                100,
                100,
                1,
                100,
                100,
                neutral_move("critical-attacker-move"),
            ),
            defender: pokemon(
                "critical-defender",
                PokemonType::Normal,
                None,
                100,
                100,
                100,
                100,
                100,
                100,
                neutral_move("critical-defender-move"),
            ),
            power: 50,
            move_type: None,
            category: DamageCategory::Physical,
            critical: true,
            random_percent: 85,
            expected: 40,
        },
        Vector {
            attacker: pokemon(
                "immune-attacker",
                PokemonType::Normal,
                None,
                100,
                100,
                100,
                100,
                100,
                100,
                neutral_move("immune-attacker-move"),
            ),
            defender: pokemon(
                "immune-defender",
                PokemonType::Ghost,
                None,
                100,
                100,
                100,
                100,
                100,
                100,
                neutral_move("immune-defender-move"),
            ),
            power: 100,
            move_type: Some(PokemonType::Normal),
            category: DamageCategory::Physical,
            critical: false,
            random_percent: 100,
            expected: 0,
        },
        Vector {
            attacker: pokemon(
                "secondary-stab-attacker",
                PokemonType::Flying,
                Some(PokemonType::Fire),
                100,
                100,
                100,
                100,
                100,
                100,
                neutral_move("secondary-stab-attacker-move"),
            ),
            defender: pokemon(
                "secondary-stab-defender",
                PokemonType::Normal,
                None,
                100,
                100,
                100,
                100,
                100,
                100,
                neutral_move("secondary-stab-defender-move"),
            ),
            power: 50,
            move_type: Some(PokemonType::Fire),
            category: DamageCategory::Special,
            critical: false,
            random_percent: 100,
            expected: 36,
        },
        Vector {
            attacker: pokemon(
                "no-stab-attacker",
                PokemonType::Normal,
                None,
                100,
                100,
                100,
                100,
                100,
                100,
                neutral_move("no-stab-attacker-move"),
            ),
            defender: pokemon(
                "no-stab-defender",
                PokemonType::Normal,
                None,
                100,
                100,
                100,
                100,
                100,
                100,
                neutral_move("no-stab-defender-move"),
            ),
            power: 50,
            move_type: Some(PokemonType::Fire),
            category: DamageCategory::Special,
            critical: false,
            random_percent: 100,
            expected: 24,
        },
    ];

    for vector in vectors {
        assert_eq!(
            super::rules::calculate_damage(
                &vector.attacker,
                &vector.defender,
                vector.power,
                vector.move_type,
                vector.category,
                vector.critical,
                vector.random_percent,
            ),
            vector.expected
        );
    }
}

#[test]
fn complete_six_versus_six_story_reaches_winner_after_five_replacements() {
    let sweeper = pokemon(
        "sweeper",
        PokemonType::Normal,
        None,
        500,
        500,
        100,
        1,
        100,
        500,
        vec![battle_move("sweep", PokemonType::Normal, 500, 6, 0)],
    );
    let mut winner_members = vec![sweeper];
    let mut loser_members = Vec::new();
    for index in 0..TEAM_SIZE {
        if index > 0 {
            winner_members.push(pokemon(
                &format!("winner-bench-{index}"),
                PokemonType::Normal,
                None,
                100,
                1,
                100,
                1,
                100,
                1,
                vec![battle_move(
                    &format!("winner-bench-move-{index}"),
                    PokemonType::Normal,
                    1,
                    1,
                    0,
                )],
            ));
        }
        loser_members.push(pokemon(
            &format!("loser-{index}"),
            PokemonType::Normal,
            None,
            1,
            1,
            1,
            1,
            1,
            1,
            vec![battle_move(
                &format!("loser-move-{index}"),
                PokemonType::Normal,
                1,
                1,
                0,
            )],
        ));
    }
    let mut battle = Battle::new(
        Team::new(winner_members).unwrap(),
        Team::new(loser_members).unwrap(),
        88,
    )
    .unwrap();

    for defeated in 0..TEAM_SIZE {
        let events = submit_turn(
            &mut battle,
            Action::UseMove(move_slot(0)),
            Action::UseMove(move_slot(0)),
        );
        assert!(events.iter().any(|event| matches!(
            event,
            BattleEvent::Fainted {
                side: Side::Two,
                ..
            }
        )));
        if defeated + 1 < TEAM_SIZE {
            assert_eq!(
                battle.phase(),
                BattlePhase::ForcedReplacement(ReplacementSides::Two)
            );
            battle
                .submit(BattleCommand::new(
                    Side::Two,
                    Action::Switch(team_slot(defeated + 1)),
                ))
                .unwrap();
        }
    }

    assert_eq!(
        battle.phase(),
        BattlePhase::Finished(BattleOutcome::Winner(Side::One))
    );
    assert_eq!(battle.active(Side::One).moves()[0].current_pp(), 0);
    assert_eq!(
        battle
            .events()
            .iter()
            .filter(|event| matches!(event, BattleEvent::ForcedReplacement { side: Side::Two }))
            .count(),
        5
    );
}
