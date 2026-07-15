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
fn battle_move_with_category_and_effect(
    id: &str,
    move_type: PokemonType,
    category: MoveCategory,
    power: u16,
    accuracy: Accuracy,
    pp: u8,
    effect: MoveEffect,
) -> Move {
    Move::new_with_category_and_effect(
        MoveId::new(id).unwrap(),
        id,
        move_type,
        category,
        power,
        accuracy,
        pp,
        pp,
        0,
        effect,
    )
    .unwrap()
}

fn stage_effect(target: EffectTarget, stat: BattleStat, amount: i8) -> MoveEffect {
    MoveEffect::change_stages(target, StageChanges::single(stat, amount).unwrap())
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

#[allow(clippy::too_many_arguments)]
fn pokemon_with_ability(
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
    ability: Ability,
) -> Pokemon {
    Pokemon::new_with_ability(
        PokemonId::new(id).unwrap(),
        id,
        50,
        primary,
        secondary,
        hp,
        hp,
        BattleStats::new(attack, defense, special_attack, special_defense, speed).unwrap(),
        moves,
        ability,
    )
    .unwrap()
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
fn battle_and_shell_armor_turn_a_successful_critical_roll_into_an_observable_ability_trigger() {
    for ability in [Ability::BattleArmor, Ability::ShellArmor] {
        let mut prevented_critical = false;
        for seed in 0..256 {
            let attacker = pokemon(
                "armor-attacker",
                PokemonType::Normal,
                None,
                10_000,
                100,
                100,
                100,
                100,
                100,
                vec![battle_move("armor-hit", PokemonType::Normal, 40, 1, 0)],
            );
            let defender = pokemon_with_ability(
                "armor-defender",
                PokemonType::Normal,
                None,
                10_000,
                100,
                100,
                100,
                100,
                1,
                vec![battle_move("armor-peer-hit", PokemonType::Normal, 1, 1, 0)],
                ability,
            );
            let mut battle = Battle::new(
                team("armor-one", attacker),
                team("armor-two", defender),
                seed,
            )
            .unwrap();
            let events = submit_turn(
                &mut battle,
                Action::UseMove(move_slot(0)),
                Action::UseMove(move_slot(0)),
            );
            let activated = events.iter().any(|event| {
                matches!(
                    event,
                    BattleEvent::AbilityActivated {
                        side: Side::Two,
                        ability: event_ability,
                        ..
                    } if *event_ability == ability
                )
            });
            if activated {
                assert!(!events.iter().any(|event| {
                    matches!(
                        event,
                        BattleEvent::Critical {
                            side: Side::One,
                            ..
                        }
                    )
                }));
                prevented_critical = true;
                break;
            }
        }
        assert!(
            prevented_critical,
            "{ability:?} never observed a critical roll"
        );
    }
}

#[test]
fn damaging_flinch_blocks_a_later_action_unless_inner_focus_or_substitute_blocks_it() {
    let flinch = MoveEffect::flinch_target(100).unwrap();
    let flinch_move = battle_move_with_category_and_effect(
        "flinch-hit",
        PokemonType::Normal,
        MoveCategory::Physical,
        40,
        Accuracy::AlwaysHit,
        1,
        flinch,
    );
    let reply = battle_move("reply", PokemonType::Normal, 1, 1, 0);
    let user = pokemon(
        "flinch-user",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![flinch_move.clone()],
    );
    let target = pokemon(
        "flinch-target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![reply.clone()],
    );
    let mut battle = Battle::new(team("flinch-one", user), team("flinch-two", target), 89).unwrap();
    let events = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::Flinched {
            side: Side::Two,
            ..
        }
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        BattleEvent::MoveUsed {
            side: Side::Two,
            ..
        }
    )));

    let inner_focus_target = pokemon_with_ability(
        "inner-focus-target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![reply.clone()],
        Ability::InnerFocus,
    );
    let mut inner_focus_battle = Battle::new(
        team(
            "inner-focus-one",
            pokemon(
                "inner-focus-user",
                PokemonType::Normal,
                None,
                100,
                100,
                100,
                100,
                100,
                100,
                vec![flinch_move.clone()],
            ),
        ),
        team("inner-focus-two", inner_focus_target),
        97,
    )
    .unwrap();
    let inner_focus_events = submit_turn(
        &mut inner_focus_battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert!(inner_focus_events.iter().any(|event| matches!(
        event,
        BattleEvent::AbilityActivated {
            side: Side::Two,
            ability: Ability::InnerFocus,
            ..
        }
    )));
    assert!(inner_focus_events.iter().any(|event| matches!(
        event,
        BattleEvent::MoveUsed {
            side: Side::Two,
            ..
        }
    )));

    let substitute = battle_move_with_category_and_effect(
        "substitute",
        PokemonType::Normal,
        MoveCategory::Status,
        0,
        Accuracy::AlwaysHit,
        1,
        MoveEffect::create_substitute(),
    );
    let mut substitute_battle = Battle::new(
        team(
            "substitute-flinch-one",
            pokemon(
                "substitute-flinch-user",
                PokemonType::Normal,
                None,
                100,
                100,
                100,
                100,
                100,
                100,
                vec![
                    battle_move("wait", PokemonType::Normal, 1, 1, 0),
                    flinch_move,
                ],
            ),
        ),
        team(
            "substitute-flinch-two",
            pokemon(
                "substitute-flinch-target",
                PokemonType::Normal,
                None,
                100,
                100,
                100,
                100,
                100,
                1,
                vec![substitute, reply],
            ),
        ),
        101,
    )
    .unwrap();
    submit_turn(
        &mut substitute_battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    let substitute_events = submit_turn(
        &mut substitute_battle,
        Action::UseMove(move_slot(1)),
        Action::UseMove(move_slot(1)),
    );
    assert!(!substitute_events.iter().any(|event| matches!(
        event,
        BattleEvent::Flinched {
            side: Side::Two,
            ..
        }
    )));
    assert!(substitute_events.iter().any(|event| matches!(
        event,
        BattleEvent::MoveUsed {
            side: Side::Two,
            ..
        }
    )));
}

#[test]
fn shield_dust_blocks_damaging_secondary_effects_but_not_status_moves() {
    let damaging_burn = MoveEffect::inflict_major_status(MajorStatusKind::Burn, 100).unwrap();
    let burn_move = battle_move_with_category_and_effect(
        "burn-hit",
        PokemonType::Fire,
        MoveCategory::Special,
        40,
        Accuracy::AlwaysHit,
        1,
        damaging_burn,
    );
    let shield_dust_target = pokemon_with_ability(
        "shield-dust-target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move("reply", PokemonType::Normal, 1, 1, 0)],
        Ability::ShieldDust,
    );
    let user = pokemon(
        "shield-dust-user",
        PokemonType::Fire,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![burn_move],
    );
    let mut battle = Battle::new(
        team("shield-dust-one", user),
        team("shield-dust-two", shield_dust_target),
        109,
    )
    .unwrap();
    let events = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::AbilityActivated {
            side: Side::Two,
            ability: Ability::ShieldDust,
            ..
        }
    )));
    assert_eq!(battle.active(Side::Two).major_status(), None);

    let status_burn = battle_move_with_category_and_effect(
        "will-o-wisp",
        PokemonType::Fire,
        MoveCategory::Status,
        0,
        Accuracy::AlwaysHit,
        1,
        MoveEffect::inflict_major_status(MajorStatusKind::Burn, 100).unwrap(),
    );
    let status_user = pokemon(
        "shield-dust-status-user",
        PokemonType::Fire,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![status_burn],
    );
    let status_target = pokemon_with_ability(
        "shield-dust-status-target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move("status-reply", PokemonType::Normal, 1, 1, 0)],
        Ability::ShieldDust,
    );
    let mut status_battle = Battle::new(
        team("shield-dust-status-one", status_user),
        team("shield-dust-status-two", status_target),
        113,
    )
    .unwrap();
    submit_turn(
        &mut status_battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert!(matches!(
        status_battle.active(Side::Two).major_status(),
        Some(MajorStatus::Burn)
    ));
}

#[test]
fn pressure_spends_extra_pp_only_for_moves_targeting_the_opponent() {
    let attacker = pokemon(
        "pressure-attacker",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move("pressure-hit", PokemonType::Normal, 40, 2, 0)],
    );
    let pressure_target = pokemon_with_ability(
        "pressure-target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move("pressure-reply", PokemonType::Normal, 1, 1, 0)],
        Ability::Pressure,
    );
    let mut battle = Battle::new(
        team("pressure-one", attacker),
        team("pressure-two", pressure_target),
        127,
    )
    .unwrap();
    let events = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(battle.active(Side::One).moves()[0].current_pp(), 0);
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::AbilityActivated {
            side: Side::Two,
            ability: Ability::Pressure,
            ..
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::PpSpent {
            side: Side::One,
            remaining: 0,
            ..
        }
    )));

    let self_boost = battle_move_with_category_and_effect(
        "pressure-self-boost",
        PokemonType::Normal,
        MoveCategory::Status,
        0,
        Accuracy::AlwaysHit,
        2,
        stage_effect(EffectTarget::User, BattleStat::Attack, 1),
    );
    let self_user = pokemon(
        "pressure-self-user",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![self_boost],
    );
    let self_target = pokemon_with_ability(
        "pressure-self-target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move(
            "pressure-self-reply",
            PokemonType::Normal,
            1,
            1,
            0,
        )],
        Ability::Pressure,
    );
    let mut self_battle = Battle::new(
        team("pressure-self-one", self_user),
        team("pressure-self-two", self_target),
        131,
    )
    .unwrap();
    let self_events = submit_turn(
        &mut self_battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(self_battle.active(Side::One).moves()[0].current_pp(), 1);
    assert!(!self_events.iter().any(|event| matches!(
        event,
        BattleEvent::AbilityActivated {
            ability: Ability::Pressure,
            ..
        }
    )));
}

#[test]
fn speed_boost_raises_speed_at_each_end_of_turn_while_the_user_is_active() {
    let speed_boost_user = pokemon_with_ability(
        "speed-boost-user",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move(
            "speed-boost-wait",
            PokemonType::Normal,
            1,
            2,
            0,
        )],
        Ability::SpeedBoost,
    );
    let target = pokemon(
        "speed-boost-target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move(
            "speed-boost-reply",
            PokemonType::Normal,
            1,
            2,
            0,
        )],
    );
    let mut battle = Battle::new(
        team("speed-boost-one", speed_boost_user),
        team("speed-boost-two", target),
        137,
    )
    .unwrap();
    let first_events = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(battle.active(Side::One).stages().get(BattleStat::Speed), 1);
    assert!(first_events.iter().any(|event| matches!(
        event,
        BattleEvent::AbilityActivated {
            side: Side::One,
            ability: Ability::SpeedBoost,
            ..
        }
    )));
    assert!(first_events.iter().any(|event| matches!(
        event,
        BattleEvent::StatStageChanged {
            side: Side::One,
            stat: BattleStat::Speed,
            change: 1,
            stage: 1,
            ..
        }
    )));

    submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(battle.active(Side::One).stages().get(BattleStat::Speed), 2);
}

#[test]
fn serene_grace_doubles_a_damaging_secondary_effect_chance() {
    let flinch = MoveEffect::flinch_target(50).unwrap();
    let mut found_doubled_roll = false;
    for seed in 0..256 {
        let make_user = |id: &str, ability: Option<Ability>| match ability {
            Some(ability) => pokemon_with_ability(
                id,
                PokemonType::Normal,
                None,
                100,
                100,
                100,
                100,
                100,
                100,
                vec![battle_move_with_category_and_effect(
                    "grace-flinch",
                    PokemonType::Normal,
                    MoveCategory::Physical,
                    40,
                    Accuracy::AlwaysHit,
                    1,
                    flinch,
                )],
                ability,
            ),
            None => pokemon(
                id,
                PokemonType::Normal,
                None,
                100,
                100,
                100,
                100,
                100,
                100,
                vec![battle_move_with_category_and_effect(
                    "plain-flinch",
                    PokemonType::Normal,
                    MoveCategory::Physical,
                    40,
                    Accuracy::AlwaysHit,
                    1,
                    flinch,
                )],
            ),
        };
        let make_target = |id: &str| {
            pokemon(
                id,
                PokemonType::Normal,
                None,
                100,
                100,
                100,
                100,
                100,
                1,
                vec![battle_move("grace-reply", PokemonType::Normal, 1, 1, 0)],
            )
        };
        let mut plain = Battle::new(
            team("plain-grace", make_user("plain-user", None)),
            team("plain-target", make_target("plain-target")),
            seed,
        )
        .unwrap();
        let plain_events = submit_turn(
            &mut plain,
            Action::UseMove(move_slot(0)),
            Action::UseMove(move_slot(0)),
        );
        let mut graceful = Battle::new(
            team(
                "grace-user",
                make_user("grace-user", Some(Ability::SereneGrace)),
            ),
            team("grace-target", make_target("grace-target")),
            seed,
        )
        .unwrap();
        let grace_events = submit_turn(
            &mut graceful,
            Action::UseMove(move_slot(0)),
            Action::UseMove(move_slot(0)),
        );
        let plain_flinched = plain_events.iter().any(|event| {
            matches!(
                event,
                BattleEvent::Flinched {
                    side: Side::Two,
                    ..
                }
            )
        });
        let grace_flinched = grace_events.iter().any(|event| {
            matches!(
                event,
                BattleEvent::Flinched {
                    side: Side::Two,
                    ..
                }
            )
        });
        if !plain_flinched && grace_flinched {
            assert!(grace_events.iter().any(|event| matches!(
                event,
                BattleEvent::AbilityActivated {
                    side: Side::One,
                    ability: Ability::SereneGrace,
                    ..
                }
            )));
            found_doubled_roll = true;
            break;
        }
    }
    assert!(found_doubled_roll);
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
fn move_category_controls_whether_damage_uses_attack_or_special_attack() {
    let physical = battle_move_with_category_and_effect(
        "physical-fire",
        PokemonType::Fire,
        MoveCategory::Physical,
        60,
        Accuracy::AlwaysHit,
        1,
        MoveEffect::None,
    );
    let special = battle_move_with_category_and_effect(
        "special-fire",
        PokemonType::Fire,
        MoveCategory::Special,
        60,
        Accuracy::AlwaysHit,
        1,
        MoveEffect::None,
    );
    let attacker = pokemon(
        "split-attacker",
        PokemonType::Normal,
        None,
        1_000,
        20,
        100,
        300,
        100,
        100,
        vec![physical, special],
    );
    let defender = pokemon(
        "split-defender",
        PokemonType::Normal,
        None,
        1_000,
        100,
        200,
        100,
        20,
        1,
        vec![battle_move("reply", PokemonType::Normal, 1, 1, 0)],
    );

    let mut physical_battle = Battle::new(
        team("physical", attacker.clone()),
        team("defender", defender.clone()),
        1,
    )
    .unwrap();
    let physical_events = submit_turn(
        &mut physical_battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    let physical_damage = physical_events
        .iter()
        .find_map(|event| match event {
            BattleEvent::Damage {
                source:
                    DamageSource::Move {
                        side: Side::One, ..
                    },
                amount,
                ..
            } => Some(*amount),
            _ => None,
        })
        .unwrap();

    let mut special_battle =
        Battle::new(team("special", attacker), team("defender", defender), 1).unwrap();
    let special_events = submit_turn(
        &mut special_battle,
        Action::UseMove(move_slot(1)),
        Action::UseMove(move_slot(0)),
    );
    let special_damage = special_events
        .iter()
        .find_map(|event| match event {
            BattleEvent::Damage {
                source:
                    DamageSource::Move {
                        side: Side::One, ..
                    },
                amount,
                ..
            } => Some(*amount),
            _ => None,
        })
        .unwrap();

    assert!(special_damage > physical_damage);
}

#[test]
fn status_moves_apply_status_without_damage_and_spend_pp() {
    let sleep = battle_move_with_category_and_effect(
        "sleep-powder",
        PokemonType::Grass,
        MoveCategory::Status,
        0,
        Accuracy::AlwaysHit,
        2,
        MoveEffect::inflict_major_status(MajorStatusKind::Sleep, 100).unwrap(),
    );
    let attacker = pokemon(
        "sleeper",
        PokemonType::Grass,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![sleep],
    );
    let defender = pokemon(
        "target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move("reply", PokemonType::Normal, 1, 2, 0)],
    );
    let mut battle = Battle::new(team("sleeper", attacker), team("target", defender), 4).unwrap();
    let events = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );

    assert_eq!(battle.active(Side::One).moves()[0].current_pp(), 1);
    assert!(matches!(
        battle.active(Side::Two).major_status(),
        Some(MajorStatus::Sleep { .. })
    ));
    assert!(!events.iter().any(|event| matches!(
        event,
        BattleEvent::Damage {
            source: DamageSource::Move {
                side: Side::One,
                ..
            },
            ..
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::StatusApplied {
            side: Side::Two,
            status: MajorStatus::Sleep { .. },
            ..
        }
    )));
}

#[test]
fn protect_blocks_targeted_moves_for_one_turn_then_expires() {
    let protector = pokemon(
        "protector",
        PokemonType::Normal,
        None,
        200,
        100,
        100,
        100,
        100,
        100,
        vec![
            battle_move_with_category_and_effect(
                "protect",
                PokemonType::Normal,
                MoveCategory::Status,
                0,
                Accuracy::AlwaysHit,
                3,
                MoveEffect::protect_user(),
            ),
            battle_move("counter", PokemonType::Normal, 1, 1, 0),
        ],
    );
    let attacker = pokemon(
        "attacker",
        PokemonType::Normal,
        None,
        200,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move("tackle", PokemonType::Normal, 40, 2, 0)],
    );
    let mut battle =
        Battle::new(team("protector", protector), team("attacker", attacker), 7).unwrap();

    let first_turn = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(battle.active(Side::One).current_hp(), 200);
    assert!(first_turn.iter().any(|event| matches!(
        event,
        BattleEvent::ProtectionActivated {
            side: Side::One,
            ..
        }
    )));
    assert!(first_turn.iter().any(|event| matches!(
        event,
        BattleEvent::MoveBlocked {
            side: Side::Two,
            target_side: Side::One,
            ..
        }
    )));
    assert!(!first_turn.iter().any(|event| matches!(
        event,
        BattleEvent::Damage {
            target_side: Side::One,
            source: DamageSource::Move { .. },
            ..
        }
    )));

    let second_turn = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(1)),
        Action::UseMove(move_slot(0)),
    );
    assert!(second_turn.iter().any(|event| matches!(
        event,
        BattleEvent::Damage {
            target_side: Side::One,
            source: DamageSource::Move { side: Side::Two, .. },
            amount,
            ..
        } if *amount > 0
    )));
}

#[test]
fn consecutive_protect_has_a_seeded_failure_path() {
    let protect = battle_move_with_category_and_effect(
        "protect",
        PokemonType::Normal,
        MoveCategory::Status,
        0,
        Accuracy::AlwaysHit,
        2,
        MoveEffect::protect_user(),
    );
    let user = pokemon(
        "user",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![protect],
    );
    let target = pokemon(
        "target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move("reply", PokemonType::Normal, 1, 2, 0)],
    );
    let saw_failure = (1..=8).any(|seed| {
        let mut battle = Battle::new(
            team("user", user.clone()),
            team("target", target.clone()),
            seed,
        )
        .unwrap();
        submit_turn(
            &mut battle,
            Action::UseMove(move_slot(0)),
            Action::UseMove(move_slot(0)),
        );
        let events = submit_turn(
            &mut battle,
            Action::UseMove(move_slot(0)),
            Action::UseMove(move_slot(0)),
        );
        events.iter().any(|event| {
            matches!(
                event,
                BattleEvent::ProtectionFailed {
                    side: Side::One,
                    ..
                }
            )
        })
    });
    assert!(saw_failure);
}

#[test]
fn sandstorm_lasts_five_turns_and_only_damages_non_immune_pokemon() {
    let sandstorm = battle_move_with_category_and_effect(
        "sandstorm",
        PokemonType::Rock,
        MoveCategory::Status,
        0,
        Accuracy::AlwaysHit,
        1,
        MoveEffect::start_weather(Weather::Sandstorm),
    );
    let rock_user = pokemon(
        "rock-user",
        PokemonType::Rock,
        None,
        1_000,
        100,
        100,
        100,
        100,
        100,
        vec![sandstorm, battle_move("wait", PokemonType::Rock, 1, 4, 0)],
    );
    let normal_target = pokemon(
        "normal-target",
        PokemonType::Normal,
        None,
        1_000,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move("wait", PokemonType::Normal, 1, 5, 0)],
    );
    let mut battle = Battle::new(
        team("rock-user", rock_user),
        team("normal-target", normal_target),
        13,
    )
    .unwrap();

    let first_turn = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(
        battle.weather(),
        Some(WeatherState::with_turns(Weather::Sandstorm, 4))
    );
    assert!(first_turn.iter().any(|event| matches!(
        event,
        BattleEvent::WeatherStarted {
            weather: Weather::Sandstorm,
            turns_remaining: Some(5),
        }
    )));
    assert!(first_turn.iter().any(|event| matches!(
        event,
        BattleEvent::Damage {
            source: DamageSource::Weather {
                weather: Weather::Sandstorm,
            },
            target_side: Side::Two,
            amount: 62,
            ..
        }
    )));
    assert!(!first_turn.iter().any(|event| matches!(
        event,
        BattleEvent::Damage {
            source: DamageSource::Weather { .. },
            target_side: Side::One,
            ..
        }
    )));

    let mut final_turn = Vec::new();
    for _ in 0..4 {
        final_turn = submit_turn(
            &mut battle,
            Action::UseMove(move_slot(1)),
            Action::UseMove(move_slot(0)),
        );
    }
    assert_eq!(battle.weather(), None);
    assert!(final_turn.iter().any(|event| matches!(
        event,
        BattleEvent::WeatherEnded {
            weather: Weather::Sandstorm,
        }
    )));
}

#[test]
fn thunder_accuracy_uses_the_current_weather() {
    assert_eq!(
        crate::rules::weather_adjusted_accuracy(
            Some(WeatherAccuracyModifier::Thunder),
            Accuracy::Percent(70),
            None,
        ),
        Accuracy::Percent(70)
    );
    assert_eq!(
        crate::rules::weather_adjusted_accuracy(
            Some(WeatherAccuracyModifier::Thunder),
            Accuracy::Percent(70),
            Some(Weather::Rain),
        ),
        Accuracy::AlwaysHit
    );
    assert_eq!(
        crate::rules::weather_adjusted_accuracy(
            Some(WeatherAccuracyModifier::Thunder),
            Accuracy::Percent(70),
            Some(Weather::Sun),
        ),
        Accuracy::Percent(50)
    );
}

#[test]
fn weather_ball_changes_power_type_and_gen_three_category() {
    assert_eq!(
        crate::rules::weather_adjusted_move(
            Some(WeatherMoveModifier::WeatherBall),
            50,
            PokemonType::Normal,
            MoveCategory::Special,
            None,
        ),
        (50, PokemonType::Normal, MoveCategory::Special)
    );
    assert_eq!(
        crate::rules::weather_adjusted_move(
            Some(WeatherMoveModifier::WeatherBall),
            50,
            PokemonType::Normal,
            MoveCategory::Special,
            Some(Weather::Rain),
        ),
        (100, PokemonType::Water, MoveCategory::Special)
    );
    assert_eq!(
        crate::rules::weather_adjusted_move(
            Some(WeatherMoveModifier::WeatherBall),
            50,
            PokemonType::Normal,
            MoveCategory::Special,
            Some(Weather::Sandstorm),
        ),
        (100, PokemonType::Rock, MoveCategory::Physical)
    );
}

#[test]
fn weather_abilities_start_permanent_weather_and_apply_passive_modifiers() {
    let drizzle = pokemon_with_ability(
        "drizzle-user",
        PokemonType::Water,
        None,
        100,
        100,
        100,
        100,
        100,
        40,
        vec![battle_move("wait", PokemonType::Normal, 1, 1, 0)],
        Ability::Drizzle,
    );
    let drought = pokemon_with_ability(
        "drought-user",
        PokemonType::Fire,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move("wait", PokemonType::Normal, 1, 1, 0)],
        Ability::Drought,
    );
    let battle = Battle::new(team("drizzle", drizzle), team("drought", drought), 31).unwrap();
    assert_eq!(
        battle.weather(),
        Some(WeatherState::permanent(Weather::Sun))
    );
    assert!(battle.events().iter().any(|event| matches!(
        event,
        BattleEvent::WeatherStarted {
            weather: Weather::Sun,
            turns_remaining: None,
        }
    )));

    let swift_swimmer = pokemon_with_ability(
        "swift-swimmer",
        PokemonType::Water,
        None,
        100,
        100,
        100,
        100,
        100,
        40,
        vec![battle_move("wait", PokemonType::Normal, 1, 1, 0)],
        Ability::SwiftSwim,
    );
    assert_eq!(swift_swimmer.effective_speed_in_weather(None), 40);
    assert_eq!(
        swift_swimmer.effective_speed_in_weather(Some(Weather::Rain)),
        80
    );

    let sand_veil = pokemon_with_ability(
        "sand-veil",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        40,
        vec![battle_move("wait", PokemonType::Normal, 1, 1, 0)],
        Ability::SandVeil,
    );
    let mut sand_battle = Battle::new(
        team(
            "sand-stream",
            pokemon_with_ability(
                "sand-stream-user",
                PokemonType::Rock,
                None,
                100,
                100,
                100,
                100,
                100,
                100,
                vec![battle_move_with_category_and_effect(
                    "wait",
                    PokemonType::Normal,
                    MoveCategory::Status,
                    0,
                    Accuracy::AlwaysHit,
                    1,
                    MoveEffect::None,
                )],
                Ability::SandStream,
            ),
        ),
        team("sand-veil", sand_veil),
        37,
    )
    .unwrap();
    submit_turn(
        &mut sand_battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(sand_battle.active(Side::Two).current_hp(), 100);

    let mut rain_dish = pokemon_with_ability(
        "rain-dish",
        PokemonType::Water,
        None,
        100,
        100,
        100,
        100,
        100,
        40,
        vec![battle_move_with_category_and_effect(
            "wait",
            PokemonType::Normal,
            MoveCategory::Status,
            0,
            Accuracy::AlwaysHit,
            3,
            MoveEffect::None,
        )],
        Ability::RainDish,
    );
    rain_dish.apply_damage(48);
    let drizzle = pokemon_with_ability(
        "drizzle",
        PokemonType::Water,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move_with_category_and_effect(
            "wait",
            PokemonType::Normal,
            MoveCategory::Status,
            0,
            Accuracy::AlwaysHit,
            3,
            MoveEffect::None,
        )],
        Ability::Drizzle,
    );
    let mut rain_battle =
        Battle::new(team("rain-dish", rain_dish), team("drizzle", drizzle), 41).unwrap();
    let rain_events = submit_turn(
        &mut rain_battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(rain_battle.active(Side::One).current_hp(), 58);
    assert!(rain_events.iter().any(|event| matches!(
        event,
        BattleEvent::AbilityActivated {
            side: Side::One,
            ability: Ability::RainDish,
            ..
        }
    )));
}

#[test]
fn cloud_nine_and_air_lock_suppress_weather_effects_without_ending_weather() {
    for ability in [Ability::CloudNine, Ability::AirLock] {
        let suppressor = pokemon_with_ability(
            "weather-suppressor",
            PokemonType::Normal,
            None,
            100,
            100,
            100,
            100,
            100,
            80,
            vec![battle_move_with_category_and_effect(
                "wait",
                PokemonType::Normal,
                MoveCategory::Status,
                0,
                Accuracy::AlwaysHit,
                1,
                MoveEffect::None,
            )],
            ability,
        );
        let sandstorm_user = pokemon(
            "sandstorm-user",
            PokemonType::Rock,
            None,
            100,
            100,
            100,
            100,
            100,
            10,
            vec![battle_move_with_category_and_effect(
                "sandstorm",
                PokemonType::Rock,
                MoveCategory::Status,
                0,
                Accuracy::AlwaysHit,
                1,
                MoveEffect::start_weather(Weather::Sandstorm),
            )],
        );
        let mut battle = Battle::new(
            team("weather-suppressor", suppressor),
            team("sandstorm-user", sandstorm_user),
            43,
        )
        .unwrap();

        let events = submit_turn(
            &mut battle,
            Action::UseMove(move_slot(0)),
            Action::UseMove(move_slot(0)),
        );

        assert_eq!(
            battle.weather(),
            Some(WeatherState::with_turns(Weather::Sandstorm, 4))
        );
        assert_eq!(battle.active(Side::One).current_hp(), 100);
        assert!(!events.iter().any(|event| matches!(
            event,
            BattleEvent::Damage {
                source: DamageSource::Weather { .. },
                ..
            }
        )));
    }

    let rain_dance = battle_move_with_category_and_effect(
        "rain-dance",
        PokemonType::Water,
        MoveCategory::Status,
        0,
        Accuracy::AlwaysHit,
        1,
        MoveEffect::start_weather(Weather::Rain),
    );
    let suppressor = pokemon_with_ability(
        "cloud-nine",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        80,
        vec![
            rain_dance,
            battle_move_with_category_and_effect(
                "wait",
                PokemonType::Normal,
                MoveCategory::Status,
                0,
                Accuracy::AlwaysHit,
                1,
                MoveEffect::None,
            ),
        ],
        Ability::CloudNine,
    );
    let swift_swimmer = pokemon_with_ability(
        "swift-swimmer",
        PokemonType::Water,
        None,
        100,
        100,
        100,
        100,
        100,
        50,
        vec![battle_move_with_category_and_effect(
            "wait",
            PokemonType::Normal,
            MoveCategory::Status,
            0,
            Accuracy::AlwaysHit,
            2,
            MoveEffect::None,
        )],
        Ability::SwiftSwim,
    );
    let mut battle = Battle::new(
        team("cloud-nine", suppressor),
        team("swift-swimmer", swift_swimmer),
        47,
    )
    .unwrap();
    submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    let events = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(1)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(
        events.iter().find_map(|event| match event {
            BattleEvent::MoveUsed { side, .. } => Some(*side),
            _ => None,
        }),
        Some(Side::One)
    );
}

#[test]
fn low_hp_type_boost_abilities_increase_only_their_matching_type() {
    let target = pokemon(
        "target",
        PokemonType::Normal,
        None,
        1_000,
        100,
        100,
        100,
        100,
        50,
        vec![battle_move("wait", PokemonType::Normal, 1, 1, 0)],
    );
    for (ability, move_type, category) in [
        (Ability::Blaze, PokemonType::Fire, DamageCategory::Special),
        (
            Ability::Overgrow,
            PokemonType::Grass,
            DamageCategory::Special,
        ),
        (Ability::Swarm, PokemonType::Bug, DamageCategory::Physical),
        (
            Ability::Torrent,
            PokemonType::Water,
            DamageCategory::Special,
        ),
    ] {
        let mut boosted = pokemon_with_ability(
            "boosted",
            PokemonType::Normal,
            None,
            100,
            100,
            100,
            100,
            100,
            50,
            vec![battle_move("attack", move_type, 60, 1, 0)],
            ability,
        );
        boosted.apply_damage(67);
        let normal_damage = crate::rules::calculate_damage(
            &boosted,
            &target,
            60,
            Some(move_type),
            category,
            false,
            100,
            None,
        );
        let unmatched_damage = crate::rules::calculate_damage(
            &boosted,
            &target,
            60,
            Some(PokemonType::Electric),
            category,
            false,
            100,
            None,
        );
        let mut healthy = boosted.clone();
        healthy.heal(67);
        let baseline_damage = crate::rules::calculate_damage(
            &healthy,
            &target,
            60,
            Some(move_type),
            category,
            false,
            100,
            None,
        );

        assert_eq!(normal_damage, baseline_damage * 3 / 2);
        assert_eq!(unmatched_damage, baseline_damage);
    }

    let mut blaze_user = pokemon_with_ability(
        "blaze-user",
        PokemonType::Fire,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move("ember", PokemonType::Fire, 60, 1, 0)],
        Ability::Blaze,
    );
    blaze_user.apply_damage(67);
    let target = pokemon(
        "target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        10,
        vec![battle_move_with_category_and_effect(
            "wait",
            PokemonType::Normal,
            MoveCategory::Status,
            0,
            Accuracy::AlwaysHit,
            1,
            MoveEffect::None,
        )],
    );
    let mut battle =
        Battle::new(team("blaze-user", blaze_user), team("target", target), 53).unwrap();
    let events = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::AbilityActivated {
            side: Side::One,
            ability: Ability::Blaze,
            ..
        }
    )));
}

#[test]
fn status_and_stat_abilities_apply_their_gen_three_modifiers() {
    let move_set = vec![battle_move("tackle", PokemonType::Normal, 60, 1, 0)];
    let normal = pokemon(
        "normal",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        50,
        move_set.clone(),
    );
    let huge_power = pokemon_with_ability(
        "huge-power",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        50,
        move_set.clone(),
        Ability::HugePower,
    );
    let pure_power = pokemon_with_ability(
        "pure-power",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        50,
        move_set.clone(),
        Ability::PurePower,
    );
    let mut guts = pokemon_with_ability(
        "guts",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        50,
        move_set.clone(),
        Ability::Guts,
    );
    let mut burned = normal.clone();
    let mut marvel_scale = pokemon_with_ability(
        "marvel-scale",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        50,
        move_set.clone(),
        Ability::MarvelScale,
    );
    assert!(guts.inflict_major_status(MajorStatus::Burn));
    assert!(burned.inflict_major_status(MajorStatus::Burn));
    assert!(marvel_scale.inflict_major_status(MajorStatus::Paralysis));

    assert_eq!(huge_power.physical_attack(), 200);
    assert_eq!(pure_power.physical_attack(), 200);
    assert_eq!(burned.physical_attack(), 50);
    assert_eq!(guts.physical_attack(), 150);
    assert_eq!(marvel_scale.effective_defense(), 150);

    let attacker = pokemon(
        "attacker",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        50,
        vec![battle_move("flame", PokemonType::Fire, 60, 1, 0)],
    );
    let thick_fat = pokemon_with_ability(
        "thick-fat",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        50,
        move_set,
        Ability::ThickFat,
    );
    let normal_damage = crate::rules::calculate_damage(
        &attacker,
        &normal,
        60,
        Some(PokemonType::Fire),
        DamageCategory::Special,
        false,
        100,
        None,
    );
    let thick_fat_damage = crate::rules::calculate_damage(
        &attacker,
        &thick_fat,
        60,
        Some(PokemonType::Fire),
        DamageCategory::Special,
        false,
        100,
        None,
    );
    assert_eq!(thick_fat_damage, normal_damage / 2);

    let mut guts_user = guts;
    let marvel_target = marvel_scale;
    guts_user.apply_damage(0);
    let mut battle =
        Battle::new(team("guts", guts_user), team("marvel", marvel_target), 59).unwrap();
    let events = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::AbilityActivated {
            side: Side::One,
            ability: Ability::Guts,
            ..
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::AbilityActivated {
            side: Side::Two,
            ability: Ability::MarvelScale,
            ..
        }
    )));

    let mut thick_fat_battle = Battle::new(
        team("fire-user", attacker),
        team("thick-fat", thick_fat),
        61,
    )
    .unwrap();
    let events = submit_turn(
        &mut thick_fat_battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::AbilityActivated {
            side: Side::Two,
            ability: Ability::ThickFat,
            ..
        }
    )));
}

#[test]
fn shadow_tag_and_arena_trap_prevent_only_the_switches_they_should() {
    let trapped = pokemon(
        "trapped",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move("wait", PokemonType::Normal, 1, 1, 0)],
    );
    let shadow_tag = pokemon_with_ability(
        "shadow-tag",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        10,
        vec![battle_move("wait", PokemonType::Normal, 1, 1, 0)],
        Ability::ShadowTag,
    );
    let mut battle = Battle::new(
        team("trapped", trapped.clone()),
        team("shadow-tag", shadow_tag),
        67,
    )
    .unwrap();
    assert!(
        battle
            .legal_actions(Side::One)
            .iter()
            .all(|action| !matches!(action, Action::Switch(_) | Action::Run))
    );
    let before = battle.clone();
    assert_eq!(
        battle.submit(BattleCommand::new(Side::One, Action::Switch(team_slot(1)))),
        Err(BattleError::ActionNotLegal {
            side: Side::One,
            action: Action::Switch(team_slot(1)),
            reason: IllegalActionReason::SwitchPrevented,
        })
    );
    assert_eq!(battle, before);
    assert_eq!(
        battle.submit(BattleCommand::new(Side::One, Action::Run)),
        Err(BattleError::ActionNotLegal {
            side: Side::One,
            action: Action::Run,
            reason: IllegalActionReason::SwitchPrevented,
        })
    );
    assert_eq!(battle, before);

    let own_shadow_tag = pokemon_with_ability(
        "own-shadow-tag",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move("wait", PokemonType::Normal, 1, 1, 0)],
        Ability::ShadowTag,
    );
    let battle = Battle::new(
        team("own-shadow-tag", own_shadow_tag),
        team("shadow-tag", battle.active(Side::Two).clone()),
        71,
    )
    .unwrap();
    assert!(
        battle
            .legal_actions(Side::One)
            .contains(&Action::Switch(team_slot(1)))
    );

    let arena_trap = pokemon_with_ability(
        "arena-trap",
        PokemonType::Ground,
        None,
        100,
        100,
        100,
        100,
        100,
        10,
        vec![battle_move("wait", PokemonType::Normal, 1, 1, 0)],
        Ability::ArenaTrap,
    );
    for (target, name) in [
        (
            pokemon(
                "flying-target",
                PokemonType::Flying,
                None,
                100,
                100,
                100,
                100,
                100,
                100,
                vec![battle_move("wait", PokemonType::Normal, 1, 1, 0)],
            ),
            "flying",
        ),
        (
            pokemon_with_ability(
                "levitate-target",
                PokemonType::Normal,
                None,
                100,
                100,
                100,
                100,
                100,
                100,
                vec![battle_move("wait", PokemonType::Normal, 1, 1, 0)],
                Ability::Levitate,
            ),
            "levitate",
        ),
    ] {
        let battle = Battle::new(
            team(name, target),
            team("arena-trap", arena_trap.clone()),
            73,
        )
        .unwrap();
        assert!(
            battle
                .legal_actions(Side::One)
                .contains(&Action::Switch(team_slot(1)))
        );
    }
}

#[test]
fn compound_eyes_and_hustle_modify_only_their_expected_stats_and_accuracy() {
    let compound_eyes = pokemon_with_ability(
        "compound-eyes",
        PokemonType::Bug,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move_with_category_and_effect(
            "status",
            PokemonType::Normal,
            MoveCategory::Status,
            0,
            Accuracy::Percent(80),
            1,
            MoveEffect::None,
        )],
        Ability::CompoundEyes,
    );
    let target = pokemon(
        "target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        10,
        vec![battle_move("wait", PokemonType::Normal, 1, 1, 0)],
    );
    let mut compound_battle = Battle::new(
        team("compound-eyes", compound_eyes),
        team("target", target.clone()),
        79,
    )
    .unwrap();
    let events = submit_turn(
        &mut compound_battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::AbilityActivated {
            side: Side::One,
            ability: Ability::CompoundEyes,
            ..
        }
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        BattleEvent::Missed {
            side: Side::One,
            ..
        }
    )));

    let hustle = pokemon_with_ability(
        "hustle",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![
            battle_move_with_category_and_effect(
                "physical",
                PokemonType::Normal,
                MoveCategory::Physical,
                60,
                Accuracy::Percent(100),
                1,
                MoveEffect::None,
            ),
            battle_move_with_category_and_effect(
                "special",
                PokemonType::Water,
                MoveCategory::Special,
                60,
                Accuracy::Percent(100),
                1,
                MoveEffect::None,
            ),
        ],
        Ability::Hustle,
    );
    assert_eq!(hustle.physical_attack(), 150);

    let physical_misses = (1..=128).any(|seed| {
        let mut battle = Battle::new(
            team("hustle", hustle.clone()),
            team("target", target.clone()),
            seed,
        )
        .unwrap();
        let events = submit_turn(
            &mut battle,
            Action::UseMove(move_slot(0)),
            Action::UseMove(move_slot(0)),
        );
        events.iter().any(|event| {
            matches!(
                event,
                BattleEvent::Missed {
                    side: Side::One,
                    ..
                }
            )
        }) && events.iter().any(|event| {
            matches!(
                event,
                BattleEvent::AbilityActivated {
                    side: Side::One,
                    ability: Ability::Hustle,
                    ..
                }
            )
        })
    });
    assert!(physical_misses);

    for seed in 1..=32 {
        let mut battle = Battle::new(
            team("hustle", hustle.clone()),
            team("target", target.clone()),
            seed,
        )
        .unwrap();
        let events = submit_turn(
            &mut battle,
            Action::UseMove(move_slot(1)),
            Action::UseMove(move_slot(0)),
        );
        assert!(!events.iter().any(|event| matches!(
            event,
            BattleEvent::Missed {
                side: Side::One,
                ..
            }
        )));
        assert!(!events.iter().any(|event| matches!(
            event,
            BattleEvent::AbilityActivated {
                side: Side::One,
                ability: Ability::Hustle,
                ..
            }
        )));
    }
}

#[test]
fn substitute_consumes_hp_absorbs_move_damage_and_blocks_targeted_status() {
    let substitute = battle_move_with_category_and_effect(
        "substitute",
        PokemonType::Normal,
        MoveCategory::Status,
        0,
        Accuracy::AlwaysHit,
        1,
        MoveEffect::create_substitute(),
    );
    let attacker = pokemon(
        "substitute-user",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![substitute],
    );
    let damaging_target = pokemon(
        "damaging-target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move(
            "break-substitute",
            PokemonType::Normal,
            100,
            1,
            0,
        )],
    );
    let mut damage_battle = Battle::new(
        team("substitute-user", attacker.clone()),
        team("damaging-target", damaging_target),
        37,
    )
    .unwrap();
    let events = submit_turn(
        &mut damage_battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(damage_battle.active(Side::One).current_hp(), 75);
    assert_eq!(damage_battle.active(Side::One).substitute_hp(), None);
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::SubstituteCreated {
            side: Side::One,
            substitute_hp: 25,
            current_hp: 75,
            ..
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::SubstituteDamaged {
            side: Side::One,
            remaining_hp: 0,
            ..
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::SubstituteBroke {
            side: Side::One,
            ..
        }
    )));

    let burn = MoveEffect::inflict_major_status(MajorStatusKind::Burn, 100).unwrap();
    let status_target = pokemon(
        "status-target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move_with_category_and_effect(
            "will-o-wisp",
            PokemonType::Fire,
            MoveCategory::Status,
            0,
            Accuracy::AlwaysHit,
            1,
            burn,
        )],
    );
    let mut status_battle = Battle::new(
        team("substitute-user", attacker),
        team("status-target", status_target),
        41,
    )
    .unwrap();
    let events = submit_turn(
        &mut status_battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(status_battle.active(Side::One).major_status(), None);
    assert_eq!(status_battle.active(Side::One).substitute_hp(), Some(25));
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::SubstituteBlocked {
            side: Side::Two,
            target_side: Side::One,
            ..
        }
    )));
}

#[test]
fn draining_and_recoil_moves_use_actual_damage_after_a_hit() {
    let drain = MoveEffect::drain_user(1, 2).unwrap();
    let draining_user = pokemon_with_hp(
        "draining-user",
        PokemonType::Grass,
        None,
        100,
        20,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move_with_category_and_effect(
            "mega-drain",
            PokemonType::Grass,
            MoveCategory::Special,
            40,
            Accuracy::AlwaysHit,
            1,
            drain,
        )],
    );
    let target = pokemon(
        "drain-target",
        PokemonType::Water,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move_with_category_and_effect(
            "wait",
            PokemonType::Normal,
            MoveCategory::Status,
            0,
            Accuracy::AlwaysHit,
            1,
            MoveEffect::None,
        )],
    );
    let mut drain_battle =
        Battle::new(team("drain", draining_user), team("target", target), 43).unwrap();
    let drain_events = submit_turn(
        &mut drain_battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    let dealt = drain_events
        .iter()
        .find_map(|event| match event {
            BattleEvent::Damage {
                source:
                    DamageSource::Move {
                        side: Side::One, ..
                    },
                amount,
                ..
            } => Some(*amount),
            _ => None,
        })
        .unwrap();
    let healed = drain_events
        .iter()
        .find_map(|event| match event {
            BattleEvent::Healed {
                side: Side::One,
                amount,
                ..
            } => Some(*amount),
            _ => None,
        })
        .unwrap();
    assert_eq!(healed, (dealt / 2).max(1));

    let substitute_drainer = pokemon_with_hp(
        "substitute-drainer",
        PokemonType::Grass,
        None,
        100,
        20,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move_with_category_and_effect(
            "mega-drain",
            PokemonType::Grass,
            MoveCategory::Special,
            40,
            Accuracy::AlwaysHit,
            1,
            drain,
        )],
    );
    let substitute_target = pokemon(
        "substitute-drain-target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move_with_category_and_effect(
            "substitute",
            PokemonType::Normal,
            MoveCategory::Status,
            0,
            Accuracy::AlwaysHit,
            1,
            MoveEffect::create_substitute(),
        )],
    );
    let mut substitute_battle = Battle::new(
        team("substitute-drainer", substitute_drainer),
        team("substitute-target", substitute_target),
        45,
    )
    .unwrap();
    let substitute_events = submit_turn(
        &mut substitute_battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(substitute_battle.active(Side::One).current_hp(), 20);
    assert!(!substitute_events.iter().any(|event| matches!(
        event,
        BattleEvent::Healed {
            side: Side::One,
            ..
        }
    )));

    let recoil = MoveEffect::recoil_user(1, 4).unwrap();
    let recoil_user = pokemon(
        "recoil-user",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move_with_category_and_effect(
            "take-down",
            PokemonType::Normal,
            MoveCategory::Physical,
            90,
            Accuracy::AlwaysHit,
            1,
            recoil,
        )],
    );
    let recoil_target = pokemon(
        "recoil-target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move_with_category_and_effect(
            "wait",
            PokemonType::Normal,
            MoveCategory::Status,
            0,
            Accuracy::AlwaysHit,
            1,
            MoveEffect::None,
        )],
    );
    let mut recoil_battle = Battle::new(
        team("recoil", recoil_user),
        team("target", recoil_target),
        47,
    )
    .unwrap();
    let recoil_events = submit_turn(
        &mut recoil_battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    let dealt = recoil_events
        .iter()
        .find_map(|event| match event {
            BattleEvent::Damage {
                source:
                    DamageSource::Move {
                        side: Side::One, ..
                    },
                amount,
                ..
            } => Some(*amount),
            _ => None,
        })
        .unwrap();
    let recoil = recoil_events
        .iter()
        .find_map(|event| match event {
            BattleEvent::Damage {
                source:
                    DamageSource::Recoil {
                        side: Side::One, ..
                    },
                amount,
                ..
            } => Some(*amount),
            _ => None,
        })
        .unwrap();
    assert_eq!(recoil, (dealt / 4).max(1));
}

#[test]
fn liquid_ooze_turns_drain_recovery_into_ability_damage() {
    let drain = MoveEffect::drain_user(1, 2).unwrap();
    let drainer = pokemon_with_hp(
        "drainer",
        PokemonType::Grass,
        None,
        100,
        40,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move_with_category_and_effect(
            "mega-drain",
            PokemonType::Grass,
            MoveCategory::Special,
            40,
            Accuracy::AlwaysHit,
            1,
            drain,
        )],
    );
    let liquid_ooze = pokemon_with_ability(
        "liquid-ooze",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move_with_category_and_effect(
            "wait",
            PokemonType::Normal,
            MoveCategory::Status,
            0,
            Accuracy::AlwaysHit,
            1,
            MoveEffect::None,
        )],
        Ability::LiquidOoze,
    );
    let mut battle = Battle::new(
        team("drainer", drainer),
        team("liquid-ooze", liquid_ooze),
        83,
    )
    .unwrap();
    let events = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    let dealt = events
        .iter()
        .find_map(|event| match event {
            BattleEvent::Damage {
                source:
                    DamageSource::Move {
                        side: Side::One, ..
                    },
                amount,
                ..
            } => Some(*amount),
            _ => None,
        })
        .unwrap();
    let ooze_damage = events
        .iter()
        .find_map(|event| match event {
            BattleEvent::Damage {
                source:
                    DamageSource::Ability {
                        side: Side::Two,
                        ability: Ability::LiquidOoze,
                        ..
                    },
                target_side: Side::One,
                amount,
                ..
            } => Some(*amount),
            _ => None,
        })
        .unwrap();
    assert_eq!(ooze_damage, (dealt / 2).max(1));
    assert_eq!(battle.active(Side::One).current_hp(), 40 - ooze_damage);
    assert!(!events.iter().any(|event| matches!(
        event,
        BattleEvent::Healed {
            side: Side::One,
            ..
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::AbilityActivated {
            side: Side::Two,
            ability: Ability::LiquidOoze,
            ..
        }
    )));
}

#[test]
fn flash_fire_blocks_fire_moves_boosts_fire_damage_and_resets_on_switch() {
    let flame_source = pokemon(
        "flame-source",
        PokemonType::Normal,
        None,
        200,
        100,
        100,
        100,
        100,
        100,
        vec![
            battle_move_with_category_and_effect(
                "ember",
                PokemonType::Fire,
                MoveCategory::Special,
                40,
                Accuracy::AlwaysHit,
                1,
                MoveEffect::None,
            ),
            battle_move_with_category_and_effect(
                "wait",
                PokemonType::Normal,
                MoveCategory::Status,
                0,
                Accuracy::AlwaysHit,
                2,
                MoveEffect::None,
            ),
        ],
    );
    let flash_fire = pokemon_with_ability(
        "flash-fire",
        PokemonType::Normal,
        None,
        200,
        100,
        100,
        100,
        100,
        90,
        vec![
            battle_move_with_category_and_effect(
                "ember",
                PokemonType::Fire,
                MoveCategory::Special,
                40,
                Accuracy::AlwaysHit,
                2,
                MoveEffect::None,
            ),
            battle_move_with_category_and_effect(
                "wait",
                PokemonType::Normal,
                MoveCategory::Status,
                0,
                Accuracy::AlwaysHit,
                1,
                MoveEffect::None,
            ),
        ],
        Ability::FlashFire,
    );
    let mut battle = Battle::new(
        team("flame-source", flame_source.clone()),
        team("flash-fire", flash_fire.clone()),
        89,
    )
    .unwrap();

    let activation = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(1)),
    );
    assert_eq!(battle.active(Side::Two).current_hp(), 200);
    assert!(activation.iter().any(|event| matches!(
        event,
        BattleEvent::AbilityActivated {
            side: Side::Two,
            ability: Ability::FlashFire,
            ..
        }
    )));

    let boosted = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(1)),
        Action::UseMove(move_slot(0)),
    );
    let boosted_damage = boosted
        .iter()
        .find_map(|event| match event {
            BattleEvent::Damage {
                source:
                    DamageSource::Move {
                        side: Side::Two, ..
                    },
                amount,
                ..
            } => Some(*amount),
            _ => None,
        })
        .unwrap();
    let normal_max_damage = crate::rules::calculate_damage(
        &flash_fire,
        &flame_source,
        40,
        Some(PokemonType::Fire),
        DamageCategory::Special,
        false,
        100,
        None,
    );
    assert!(u64::from(boosted_damage) > normal_max_damage);
    assert!(boosted.iter().any(|event| matches!(
        event,
        BattleEvent::AbilityActivated {
            side: Side::Two,
            ability: Ability::FlashFire,
            ..
        }
    )));

    submit_turn(
        &mut battle,
        Action::Switch(team_slot(1)),
        Action::Switch(team_slot(1)),
    );
    submit_turn(
        &mut battle,
        Action::Switch(team_slot(0)),
        Action::Switch(team_slot(0)),
    );
    let after_switch = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(1)),
        Action::UseMove(move_slot(0)),
    );
    assert!(!after_switch.iter().any(|event| matches!(
        event,
        BattleEvent::AbilityActivated {
            side: Side::Two,
            ability: Ability::FlashFire,
            ..
        }
    )));

    let status_fire = battle_move_with_category_and_effect(
        "will-o-wisp",
        PokemonType::Fire,
        MoveCategory::Status,
        0,
        Accuracy::AlwaysHit,
        1,
        MoveEffect::inflict_major_status(MajorStatusKind::Burn, 100).unwrap(),
    );
    let status_user = pokemon(
        "status-user",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![status_fire],
    );
    let mut status_battle = Battle::new(
        team("status-user", status_user),
        team("flash-fire", flash_fire),
        97,
    )
    .unwrap();
    let status_events = submit_turn(
        &mut status_battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(1)),
    );
    assert_eq!(status_battle.active(Side::Two).major_status(), None);
    assert!(status_events.iter().any(|event| matches!(
        event,
        BattleEvent::AbilityActivated {
            side: Side::Two,
            ability: Ability::FlashFire,
            ..
        }
    )));
}

#[test]
fn rock_head_blocks_mapped_move_recoil_but_not_struggle_recoil() {
    let recoil = MoveEffect::recoil_user(1, 4).unwrap();
    let user = pokemon_with_ability(
        "rock-head-user",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move_with_category_and_effect(
            "take-down",
            PokemonType::Normal,
            MoveCategory::Physical,
            90,
            Accuracy::AlwaysHit,
            1,
            recoil,
        )],
        Ability::RockHead,
    );
    let target = pokemon(
        "rock-head-target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move("wait", PokemonType::Normal, 1, 1, 0)],
    );
    let mut battle = Battle::new(
        team("rock-head-one", user),
        team("rock-head-two", target),
        79,
    )
    .unwrap();
    let events = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::AbilityActivated {
            side: Side::One,
            ability: Ability::RockHead,
            ..
        }
    )));
    assert!(!events.iter().any(|event| matches!(
        event,
        BattleEvent::Damage {
            source: DamageSource::Recoil {
                side: Side::One,
                ..
            },
            ..
        }
    )));

    let empty = pokemon_with_ability(
        "rock-head-empty",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move("empty", PokemonType::Normal, 40, 0, 0)],
        Ability::RockHead,
    );
    let passive = pokemon(
        "rock-head-passive",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move("passive", PokemonType::Normal, 1, 1, 0)],
    );
    let mut struggle_battle = Battle::new(
        team("rock-head-empty", empty),
        team("rock-head-passive", passive),
        83,
    )
    .unwrap();
    let struggle_events = submit_turn(
        &mut struggle_battle,
        Action::Struggle,
        Action::UseMove(move_slot(0)),
    );
    assert!(struggle_events.iter().any(|event| matches!(
        event,
        BattleEvent::Damage {
            source: DamageSource::Recoil { side: Side::One, .. },
            amount,
            ..
        } if *amount > 0
    )));
}

#[test]
fn fixed_damage_moves_ignore_damage_modifiers_but_keep_type_immunity() {
    let sonic_boom = battle_move_with_category_and_effect(
        "sonic-boom",
        PokemonType::Normal,
        MoveCategory::Special,
        0,
        Accuracy::AlwaysHit,
        1,
        MoveEffect::fixed_damage_amount(20),
    );
    let seismic_toss = battle_move_with_category_and_effect(
        "seismic-toss",
        PokemonType::Fighting,
        MoveCategory::Physical,
        0,
        Accuracy::AlwaysHit,
        1,
        MoveEffect::fixed_damage_user_level(),
    );
    let user = pokemon(
        "fixed-user",
        PokemonType::Normal,
        None,
        100,
        1,
        100,
        1,
        100,
        100,
        vec![sonic_boom.clone(), seismic_toss],
    );
    let target = pokemon(
        "fixed-target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move("wait", PokemonType::Normal, 1, 1, 0)],
    );
    let mut battle = Battle::new(team("fixed-one", user), team("fixed-two", target), 103).unwrap();
    let sonic_events = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert!(sonic_events.iter().any(|event| matches!(
        event,
        BattleEvent::Damage {
            source: DamageSource::Move {
                side: Side::One,
                ..
            },
            amount: 20,
            ..
        }
    )));
    let seismic_events = submit_turn(&mut battle, Action::UseMove(move_slot(1)), Action::Struggle);
    assert!(seismic_events.iter().any(|event| matches!(
        event,
        BattleEvent::Damage {
            source: DamageSource::Move {
                side: Side::One,
                ..
            },
            amount: 50,
            ..
        }
    )));

    let immune_target = pokemon(
        "fixed-immune-target",
        PokemonType::Ghost,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move("immune-wait", PokemonType::Ghost, 1, 1, 0)],
    );
    let immune_user = pokemon(
        "fixed-immune-user",
        PokemonType::Normal,
        None,
        100,
        1,
        100,
        1,
        100,
        100,
        vec![sonic_boom],
    );
    let mut immune_battle = Battle::new(
        team("fixed-immune-one", immune_user),
        team("fixed-immune-two", immune_target),
        107,
    )
    .unwrap();
    let immune_events = submit_turn(
        &mut immune_battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert!(immune_events.iter().any(|event| matches!(
        event,
        BattleEvent::Effectiveness {
            side: Side::One,
            effectiveness: TypeEffectiveness::Immune,
            ..
        }
    )));
    assert!(immune_events.iter().any(|event| matches!(
        event,
        BattleEvent::Damage {
            source: DamageSource::Move {
                side: Side::One,
                ..
            },
            amount: 0,
            ..
        }
    )));
    assert!(!immune_events.iter().any(|event| matches!(
        event,
        BattleEvent::Critical {
            side: Side::One,
            ..
        }
    )));
}

#[test]
fn haze_resets_both_active_pokemons_stat_stages() {
    let harden = battle_move_with_category_and_effect(
        "harden",
        PokemonType::Normal,
        MoveCategory::Status,
        0,
        Accuracy::AlwaysHit,
        1,
        stage_effect(EffectTarget::User, BattleStat::Defense, 1),
    );
    let haze = battle_move_with_category_and_effect(
        "haze",
        PokemonType::Ice,
        MoveCategory::Status,
        0,
        Accuracy::AlwaysHit,
        1,
        MoveEffect::haze(),
    );
    let growl = battle_move_with_category_and_effect(
        "growl",
        PokemonType::Normal,
        MoveCategory::Status,
        0,
        Accuracy::AlwaysHit,
        1,
        stage_effect(EffectTarget::Opponent, BattleStat::Attack, -1),
    );
    let wait = battle_move_with_category_and_effect(
        "wait",
        PokemonType::Normal,
        MoveCategory::Status,
        0,
        Accuracy::AlwaysHit,
        1,
        MoveEffect::None,
    );
    let user = pokemon(
        "haze-user",
        PokemonType::Ice,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![harden, haze],
    );
    let target = pokemon(
        "haze-target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![growl, wait],
    );
    let mut battle = Battle::new(team("haze", user), team("target", target), 53).unwrap();
    submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(
        battle.active(Side::One).stages().get(BattleStat::Attack),
        -1
    );
    assert_eq!(
        battle.active(Side::One).stages().get(BattleStat::Defense),
        1
    );

    let events = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(1)),
        Action::UseMove(move_slot(1)),
    );
    for stat in BattleStat::ALL {
        assert_eq!(battle.active(Side::One).stages().get(stat), 0);
        assert_eq!(battle.active(Side::Two).stages().get(stat), 0);
    }
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::StatStageChanged {
            side: Side::One,
            stat: BattleStat::Attack,
            stage: 0,
            ..
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::StatStageChanged {
            side: Side::One,
            stat: BattleStat::Defense,
            stage: 0,
            ..
        }
    )));
}

#[test]
fn psych_up_copies_every_target_stat_stage_onto_the_user() {
    let wait = battle_move_with_category_and_effect(
        "wait",
        PokemonType::Normal,
        MoveCategory::Status,
        0,
        Accuracy::AlwaysHit,
        2,
        MoveEffect::None,
    );
    let psych_up = battle_move_with_category_and_effect(
        "psych-up",
        PokemonType::Normal,
        MoveCategory::Status,
        0,
        Accuracy::AlwaysHit,
        1,
        MoveEffect::copy_target_stages(),
    );
    let boost = battle_move_with_category_and_effect(
        "target-boost",
        PokemonType::Normal,
        MoveCategory::Status,
        0,
        Accuracy::AlwaysHit,
        1,
        MoveEffect::change_stages(
            EffectTarget::User,
            StageChanges::new(2, -1, 0, 1, 0, 0, 0).unwrap(),
        ),
    );
    let user = pokemon(
        "psych-up-user",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![wait.clone(), psych_up],
    );
    let target = pokemon(
        "psych-up-target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![boost, wait],
    );
    let mut battle =
        Battle::new(team("psych-up-one", user), team("psych-up-two", target), 57).unwrap();
    submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );

    let events = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(1)),
        Action::UseMove(move_slot(1)),
    );
    for stat in BattleStat::ALL {
        assert_eq!(
            battle.active(Side::One).stages().get(stat),
            battle.active(Side::Two).stages().get(stat),
        );
    }
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::StatStageChanged {
            side: Side::One,
            stat: BattleStat::Attack,
            change: 2,
            stage: 2,
            ..
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::StatStageChanged {
            side: Side::One,
            stat: BattleStat::Defense,
            change: -1,
            stage: -1,
            ..
        }
    )));
}

#[test]
fn rain_and_sun_modify_water_and_fire_damage() {
    let attacker = pokemon(
        "attacker",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move("move", PokemonType::Normal, 1, 1, 0)],
    );
    let defender = pokemon(
        "defender",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move("reply", PokemonType::Normal, 1, 1, 0)],
    );
    let damage = |move_type, weather| {
        crate::rules::calculate_damage(
            &attacker,
            &defender,
            60,
            Some(move_type),
            DamageCategory::Special,
            false,
            100,
            weather,
        )
    };
    assert!(damage(PokemonType::Water, Some(Weather::Rain)) > damage(PokemonType::Water, None));
    assert!(damage(PokemonType::Fire, Some(Weather::Rain)) < damage(PokemonType::Fire, None));
    assert!(damage(PokemonType::Fire, Some(Weather::Sun)) > damage(PokemonType::Fire, None));
}

#[test]
fn stat_changes_and_healing_update_battle_state_and_events() {
    let harden = battle_move_with_category_and_effect(
        "harden",
        PokemonType::Normal,
        MoveCategory::Status,
        0,
        Accuracy::AlwaysHit,
        2,
        MoveEffect::change_stages(
            EffectTarget::User,
            StageChanges::single(BattleStat::Defense, 1).unwrap(),
        ),
    );
    let recover = battle_move_with_category_and_effect(
        "recover",
        PokemonType::Normal,
        MoveCategory::Status,
        0,
        Accuracy::AlwaysHit,
        1,
        MoveEffect::heal_user(1, 2).unwrap(),
    );
    let attacker = pokemon_with_hp(
        "user",
        PokemonType::Normal,
        None,
        100,
        50,
        100,
        100,
        100,
        100,
        100,
        vec![harden, recover],
    );
    let defender = pokemon(
        "target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move("reply", PokemonType::Normal, 1, 2, 0)],
    );
    let mut battle = Battle::new(team("user", attacker), team("target", defender), 6).unwrap();

    let stages = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(
        battle.active(Side::One).stages().get(BattleStat::Defense),
        1
    );
    assert!(stages.iter().any(|event| matches!(
        event,
        BattleEvent::StatStageChanged {
            side: Side::One,
            stat: BattleStat::Defense,
            change: 1,
            stage: 1,
            ..
        }
    )));

    let healing = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(1)),
        Action::UseMove(move_slot(0)),
    );
    assert!(healing.iter().any(|event| matches!(
        event,
        BattleEvent::Healed {
            side: Side::One,
            amount,
            current_hp,
            ..
        } if *amount == 50 && *current_hp > 50
    )));
}

#[test]
fn stat_stages_change_damage_and_validate_boundaries() {
    let attacker = pokemon(
        "attacker",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move("tackle", PokemonType::Normal, 40, 1, 0)],
    );
    let defender = pokemon(
        "defender",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move("reply", PokemonType::Normal, 1, 1, 0)],
    );
    let base = crate::rules::calculate_damage(
        &attacker,
        &defender,
        40,
        Some(PokemonType::Normal),
        DamageCategory::Physical,
        false,
        100,
        None,
    );
    let mut staged_defender = defender.clone();
    assert_eq!(
        staged_defender.change_stage(BattleStat::Defense, 2),
        Some(2)
    );
    let reduced = crate::rules::calculate_damage(
        &attacker,
        &staged_defender,
        40,
        Some(PokemonType::Normal),
        DamageCategory::Physical,
        false,
        100,
        None,
    );
    assert!(reduced < base);
    staged_defender.reset_switch_modifiers();
    assert_eq!(staged_defender.stages().get(BattleStat::Defense), 0);
    assert_eq!(
        StageChanges::single(BattleStat::Attack, 0),
        Err(ValidationError::EmptyStageChanges)
    );
    assert_eq!(
        StageChanges::single(BattleStat::Attack, MAX_STAT_STAGE + 1),
        Err(ValidationError::InvalidStageChange)
    );
}

#[test]
fn critical_hits_follow_the_gen_three_stat_stage_rule() {
    let attacker = pokemon(
        "critical-attacker",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move("tackle", PokemonType::Normal, 40, 1, 0)],
    );
    let defender = pokemon(
        "critical-defender",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move("reply", PokemonType::Normal, 1, 1, 0)],
    );
    let damage = |attacker: &Pokemon, defender: &Pokemon, critical| {
        crate::rules::calculate_damage(
            attacker,
            defender,
            60,
            None,
            DamageCategory::Physical,
            critical,
            100,
            None,
        )
    };
    let baseline_critical = damage(&attacker, &defender, true);
    let mut hindered_attacker = attacker.clone();
    let mut fortified_defender = defender.clone();
    hindered_attacker.change_stage(BattleStat::Attack, -2);
    fortified_defender.change_stage(BattleStat::Defense, 2);
    assert_eq!(
        damage(&hindered_attacker, &fortified_defender, true),
        baseline_critical
    );
    assert!(damage(&hindered_attacker, &fortified_defender, false) < baseline_critical);

    let mut boosted_attacker = attacker.clone();
    let mut weakened_defender = defender.clone();
    boosted_attacker.change_stage(BattleStat::Attack, 2);
    weakened_defender.change_stage(BattleStat::Defense, -2);
    assert!(damage(&boosted_attacker, &weakened_defender, true) > baseline_critical);
}

#[test]
fn damaging_move_can_apply_major_status_or_report_an_immune_target() {
    let burn = MoveEffect::inflict_major_status(MajorStatusKind::Burn, 100).unwrap();
    let attacker = pokemon(
        "burner",
        PokemonType::Fire,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move_with_category_and_effect(
            "ember",
            PokemonType::Fire,
            MoveCategory::Special,
            20,
            Accuracy::AlwaysHit,
            1,
            burn,
        )],
    );
    let target = pokemon(
        "target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move("reply", PokemonType::Normal, 1, 1, 0)],
    );
    let mut battle =
        Battle::new(team("burner", attacker.clone()), team("target", target), 5).unwrap();
    let events = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(
        battle.active(Side::Two).major_status(),
        Some(MajorStatus::Burn)
    );
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::StatusApplied {
            status: MajorStatus::Burn,
            ..
        }
    )));

    let fire_target = pokemon(
        "fire-target",
        PokemonType::Fire,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move("reply", PokemonType::Normal, 1, 1, 0)],
    );
    let mut immune_battle = Battle::new(
        team("burner", attacker),
        team("fire-target", fire_target),
        5,
    )
    .unwrap();
    let events = submit_turn(
        &mut immune_battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(immune_battle.active(Side::Two).major_status(), None);
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::StatusFailed {
            side: Side::One,
            target_side: Side::Two,
            status: MajorStatusKind::Burn,
            ..
        }
    )));
}

#[test]
fn abilities_block_status_and_typed_moves_with_observable_events() {
    let poison = MoveEffect::inflict_major_status(MajorStatusKind::Poison, 100).unwrap();
    let status_user = pokemon(
        "status-user",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move_with_category_and_effect(
            "poison-powder",
            PokemonType::Poison,
            MoveCategory::Status,
            0,
            Accuracy::AlwaysHit,
            1,
            poison,
        )],
    );
    let immune_target = pokemon_with_ability(
        "immune-target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move("reply", PokemonType::Normal, 1, 1, 0)],
        Ability::Immunity,
    );
    let mut status_battle = Battle::new(
        team("status-user", status_user),
        team("immune-target", immune_target),
        17,
    )
    .unwrap();
    let events = submit_turn(
        &mut status_battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(status_battle.active(Side::Two).major_status(), None);
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::AbilityActivated {
            side: Side::Two,
            ability: Ability::Immunity,
            ..
        }
    )));

    let water_user = pokemon(
        "water-user",
        PokemonType::Water,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move_with_category_and_effect(
            "water-pulse",
            PokemonType::Water,
            MoveCategory::Special,
            60,
            Accuracy::AlwaysHit,
            1,
            MoveEffect::None,
        )],
    );
    let mut absorb_target = pokemon_with_ability(
        "absorb-target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move("reply", PokemonType::Normal, 1, 1, 0)],
        Ability::WaterAbsorb,
    );
    absorb_target.apply_damage(40);
    let mut absorb_battle = Battle::new(
        team("water-user", water_user),
        team("absorb-target", absorb_target),
        19,
    )
    .unwrap();
    let events = submit_turn(
        &mut absorb_battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(absorb_battle.active(Side::Two).current_hp(), 85);
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::AbilityActivated {
            side: Side::Two,
            ability: Ability::WaterAbsorb,
            ..
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::Effectiveness {
            effectiveness: TypeEffectiveness::Immune,
            ..
        }
    )));
}

#[test]
fn synchronize_reflects_burn_poison_and_paralysis_after_a_successful_infliction() {
    let poison = MoveEffect::inflict_major_status(MajorStatusKind::Poison, 100).unwrap();
    let user = pokemon(
        "synchronize-user",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move_with_category_and_effect(
            "poison-powder",
            PokemonType::Poison,
            MoveCategory::Status,
            0,
            Accuracy::AlwaysHit,
            1,
            poison,
        )],
    );
    let target = pokemon_with_ability(
        "synchronize-target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move_with_category_and_effect(
            "wait",
            PokemonType::Normal,
            MoveCategory::Status,
            0,
            Accuracy::AlwaysHit,
            1,
            MoveEffect::None,
        )],
        Ability::Synchronize,
    );
    let mut battle =
        Battle::new(team("synchronize-user", user), team("target", target), 89).unwrap();
    let events = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(
        battle.active(Side::One).major_status(),
        Some(MajorStatus::Poison)
    );
    assert_eq!(
        battle.active(Side::Two).major_status(),
        Some(MajorStatus::Poison)
    );
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::AbilityActivated {
            side: Side::Two,
            ability: Ability::Synchronize,
            ..
        }
    )));
}

#[test]
fn intimidate_activates_on_entry_and_lowers_the_opponents_attack() {
    let intimidator = pokemon_with_ability(
        "intimidator",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move("tackle", PokemonType::Normal, 40, 1, 0)],
        Ability::Intimidate,
    );
    let target = pokemon(
        "target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move("reply", PokemonType::Normal, 1, 1, 0)],
    );
    let battle = Battle::new(team("intimidator", intimidator), team("target", target), 23).unwrap();
    assert_eq!(
        battle.active(Side::Two).stages().get(BattleStat::Attack),
        -1
    );
    assert!(battle.events().iter().any(|event| matches!(
        event,
        BattleEvent::AbilityActivated {
            side: Side::One,
            ability: Ability::Intimidate,
            ..
        }
    )));
}

#[test]
fn abilities_block_opponents_stat_drops_including_intimidate() {
    let growl = battle_move_with_category_and_effect(
        "growl",
        PokemonType::Normal,
        MoveCategory::Status,
        0,
        Accuracy::AlwaysHit,
        1,
        stage_effect(EffectTarget::Opponent, BattleStat::Attack, -1),
    );
    let attacker = pokemon(
        "stat-drop-user",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![growl],
    );
    let clear_body = pokemon_with_ability(
        "clear-body-target",
        PokemonType::Steel,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move_with_category_and_effect(
            "wait",
            PokemonType::Normal,
            MoveCategory::Status,
            0,
            Accuracy::AlwaysHit,
            1,
            MoveEffect::None,
        )],
        Ability::ClearBody,
    );
    let mut battle = Battle::new(
        team("stat-drop-user", attacker),
        team("clear-body-target", clear_body),
        59,
    )
    .unwrap();
    let events = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(battle.active(Side::Two).stages().get(BattleStat::Attack), 0);
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::AbilityActivated {
            side: Side::Two,
            ability: Ability::ClearBody,
            ..
        }
    )));

    let intimidator = pokemon_with_ability(
        "intimidator",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move("wait", PokemonType::Normal, 1, 1, 0)],
        Ability::Intimidate,
    );
    let clear_body = pokemon_with_ability(
        "intimidate-clear-body-target",
        PokemonType::Steel,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move("wait", PokemonType::Normal, 1, 1, 0)],
        Ability::ClearBody,
    );
    let battle = Battle::new(
        team("intimidator", intimidator),
        team("clear-body", clear_body),
        61,
    )
    .unwrap();
    assert_eq!(battle.active(Side::Two).stages().get(BattleStat::Attack), 0);
    assert!(battle.events().iter().any(|event| matches!(
        event,
        BattleEvent::AbilityActivated {
            side: Side::Two,
            ability: Ability::ClearBody,
            ..
        }
    )));

    let hyper_cutter = pokemon_with_ability(
        "hyper-cutter",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move("wait", PokemonType::Normal, 1, 1, 0)],
        Ability::HyperCutter,
    );
    assert_eq!(
        hyper_cutter.ability_blocks_opponent_stat_drop(BattleStat::Attack),
        Some(Ability::HyperCutter)
    );
    assert_eq!(
        hyper_cutter.ability_blocks_opponent_stat_drop(BattleStat::Defense),
        None
    );
    let keen_eye = pokemon_with_ability(
        "keen-eye",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move("wait", PokemonType::Normal, 1, 1, 0)],
        Ability::KeenEye,
    );
    assert_eq!(
        keen_eye.ability_blocks_opponent_stat_drop(BattleStat::Accuracy),
        Some(Ability::KeenEye)
    );
}

#[test]
fn natural_cure_clears_major_status_before_switching_out() {
    let mut user = pokemon_with_ability(
        "natural-cure-user",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move("wait", PokemonType::Normal, 1, 1, 0)],
        Ability::NaturalCure,
    );
    assert!(user.inflict_major_status(MajorStatus::Burn));
    let target = pokemon(
        "natural-cure-target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move_with_category_and_effect(
            "wait",
            PokemonType::Normal,
            MoveCategory::Status,
            0,
            Accuracy::AlwaysHit,
            1,
            MoveEffect::None,
        )],
    );
    let mut battle = Battle::new(team("natural-cure", user), team("target", target), 79).unwrap();
    let events = submit_turn(
        &mut battle,
        Action::Switch(team_slot(1)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(
        battle.team(Side::One).member(team_slot(0)).major_status(),
        None
    );
    let cured = events
        .iter()
        .position(|event| {
            matches!(
                event,
                BattleEvent::StatusCured {
                    side: Side::One,
                    status: MajorStatusKind::Burn,
                    ..
                }
            )
        })
        .unwrap();
    let switched = events
        .iter()
        .position(|event| {
            matches!(
                event,
                BattleEvent::Switched {
                    side: Side::One,
                    ..
                }
            )
        })
        .unwrap();
    assert!(cured < switched);
}

#[test]
fn shed_skin_can_cure_a_major_status_at_end_of_turn() {
    for seed in 0..100_u64 {
        let mut user = pokemon_with_ability(
            "shed-skin-user",
            PokemonType::Normal,
            None,
            100,
            100,
            100,
            100,
            100,
            100,
            vec![battle_move_with_category_and_effect(
                "wait",
                PokemonType::Normal,
                MoveCategory::Status,
                0,
                Accuracy::AlwaysHit,
                1,
                MoveEffect::None,
            )],
            Ability::ShedSkin,
        );
        assert!(user.inflict_major_status(MajorStatus::Burn));
        let target = pokemon(
            "shed-skin-target",
            PokemonType::Normal,
            None,
            100,
            100,
            100,
            100,
            100,
            1,
            vec![battle_move_with_category_and_effect(
                "wait",
                PokemonType::Normal,
                MoveCategory::Status,
                0,
                Accuracy::AlwaysHit,
                1,
                MoveEffect::None,
            )],
        );
        let mut battle =
            Battle::new(team("shed-skin", user), team("target", target), seed).unwrap();
        let events = submit_turn(
            &mut battle,
            Action::UseMove(move_slot(0)),
            Action::UseMove(move_slot(0)),
        );
        if events.iter().any(|event| {
            matches!(
                event,
                BattleEvent::AbilityActivated {
                    side: Side::One,
                    ability: Ability::ShedSkin,
                    ..
                }
            )
        }) {
            assert_eq!(battle.active(Side::One).major_status(), None);
            assert!(events.iter().any(|event| matches!(
                event,
                BattleEvent::StatusCured {
                    side: Side::One,
                    status: MajorStatusKind::Burn,
                    ..
                }
            )));
            return;
        }
    }
    panic!("a deterministic seed should trigger the 30 percent Shed Skin cure");
}

#[test]
fn damaging_moves_can_apply_a_chance_based_stat_drop() {
    let icy_wind = battle_move_with_category_and_effect(
        "icy-wind",
        PokemonType::Ice,
        MoveCategory::Special,
        55,
        Accuracy::AlwaysHit,
        1,
        MoveEffect::change_stages_with_chance(
            EffectTarget::Opponent,
            StageChanges::single(BattleStat::Speed, -1).unwrap(),
            100,
        )
        .unwrap(),
    );
    let attacker = pokemon(
        "icy-wind-user",
        PokemonType::Ice,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![icy_wind],
    );
    let target = pokemon(
        "icy-wind-target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move_with_category_and_effect(
            "wait",
            PokemonType::Normal,
            MoveCategory::Status,
            0,
            Accuracy::AlwaysHit,
            1,
            MoveEffect::None,
        )],
    );
    let mut battle = Battle::new(team("icy-wind", attacker), team("target", target), 67).unwrap();
    let events = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(battle.active(Side::Two).stages().get(BattleStat::Speed), -1);
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::StatStageChanged {
            side: Side::Two,
            stat: BattleStat::Speed,
            change: -1,
            stage: -1,
            ..
        }
    )));
}

#[test]
fn rest_cures_status_restores_hp_and_applies_its_fixed_sleep_duration() {
    let mut user = pokemon_with_hp(
        "rest-user",
        PokemonType::Normal,
        None,
        100,
        60,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move_with_category_and_effect(
            "rest",
            PokemonType::Psychic,
            MoveCategory::Status,
            0,
            Accuracy::AlwaysHit,
            1,
            MoveEffect::rest(),
        )],
    );
    assert!(user.inflict_major_status(MajorStatus::Burn));
    let target = pokemon(
        "rest-target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move_with_category_and_effect(
            "wait",
            PokemonType::Normal,
            MoveCategory::Status,
            0,
            Accuracy::AlwaysHit,
            3,
            MoveEffect::None,
        )],
    );
    let mut battle = Battle::new(team("rest", user), team("target", target), 71).unwrap();
    let events = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(battle.active(Side::One).current_hp(), 100);
    assert_eq!(
        battle.active(Side::One).major_status(),
        Some(MajorStatus::Sleep { turns_remaining: 3 })
    );
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::StatusCured {
            side: Side::One,
            status: MajorStatusKind::Burn,
            ..
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::Healed {
            side: Side::One,
            amount: 40,
            ..
        }
    )));
    let first_sleep_turn =
        submit_turn(&mut battle, Action::Struggle, Action::UseMove(move_slot(0)));
    let second_sleep_turn =
        submit_turn(&mut battle, Action::Struggle, Action::UseMove(move_slot(0)));
    let wake_turn = submit_turn(&mut battle, Action::Struggle, Action::Struggle);
    assert!(first_sleep_turn.iter().any(|event| matches!(
        event,
        BattleEvent::StatusPreventsAction {
            side: Side::One,
            status: MajorStatus::Sleep { turns_remaining: 2 },
            ..
        }
    )));
    assert!(second_sleep_turn.iter().any(|event| matches!(
        event,
        BattleEvent::StatusPreventsAction {
            side: Side::One,
            status: MajorStatus::Sleep { turns_remaining: 1 },
            ..
        }
    )));
    assert!(wake_turn.iter().any(|event| matches!(
        event,
        BattleEvent::StatusCured {
            side: Side::One,
            status: MajorStatusKind::Sleep,
            ..
        }
    )));
}

#[test]
fn early_bird_halves_rests_sleep_action_loss() {
    let mut user = pokemon_with_ability(
        "early-bird-user",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move_with_category_and_effect(
            "rest",
            PokemonType::Psychic,
            MoveCategory::Status,
            0,
            Accuracy::AlwaysHit,
            1,
            MoveEffect::rest(),
        )],
        Ability::EarlyBird,
    );
    user.apply_damage(40);
    let target = pokemon(
        "early-bird-target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move_with_category_and_effect(
            "wait",
            PokemonType::Normal,
            MoveCategory::Status,
            0,
            Accuracy::AlwaysHit,
            2,
            MoveEffect::None,
        )],
    );
    let mut battle = Battle::new(team("early-bird", user), team("target", target), 83).unwrap();
    submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    let blocked_turn = submit_turn(&mut battle, Action::Struggle, Action::UseMove(move_slot(0)));
    let wake_turn = submit_turn(&mut battle, Action::Struggle, Action::Struggle);
    assert!(blocked_turn.iter().any(|event| matches!(
        event,
        BattleEvent::StatusPreventsAction {
            side: Side::One,
            status: MajorStatus::Sleep { turns_remaining: 1 },
            ..
        }
    )));
    assert!(wake_turn.iter().any(|event| matches!(
        event,
        BattleEvent::StatusCured {
            side: Side::One,
            status: MajorStatusKind::Sleep,
            ..
        }
    )));
}

#[test]
fn refresh_cures_only_its_supported_major_statuses() {
    let mut user = pokemon(
        "refresh-user",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move_with_category_and_effect(
            "refresh",
            PokemonType::Normal,
            MoveCategory::Status,
            0,
            Accuracy::AlwaysHit,
            1,
            MoveEffect::refresh(),
        )],
    );
    assert!(user.inflict_major_status(MajorStatus::Burn));
    let target = pokemon(
        "refresh-target",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        1,
        vec![battle_move_with_category_and_effect(
            "wait",
            PokemonType::Normal,
            MoveCategory::Status,
            0,
            Accuracy::AlwaysHit,
            1,
            MoveEffect::None,
        )],
    );
    let mut battle = Battle::new(team("refresh", user), team("target", target), 73).unwrap();
    let events = submit_turn(
        &mut battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(battle.active(Side::One).major_status(), None);
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::StatusCured {
            side: Side::One,
            status: MajorStatusKind::Burn,
            ..
        }
    )));

    let mut frozen = pokemon(
        "frozen-refresh",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![battle_move("wait", PokemonType::Normal, 1, 1, 0)],
    );
    assert!(frozen.inflict_major_status(MajorStatus::Freeze));
    assert_eq!(frozen.refresh(), None);
    assert_eq!(frozen.major_status(), Some(MajorStatus::Freeze));
}

#[test]
fn burn_paralysis_and_bad_poison_apply_their_gen_three_battle_modifiers() {
    let burn = MoveEffect::inflict_major_status(MajorStatusKind::Burn, 100).unwrap();
    let paralysis = MoveEffect::inflict_major_status(MajorStatusKind::Paralysis, 100).unwrap();
    let bad_poison = MoveEffect::inflict_major_status(MajorStatusKind::BadlyPoisoned, 100).unwrap();
    let status_move = |id, effect| {
        battle_move_with_category_and_effect(
            id,
            PokemonType::Normal,
            MoveCategory::Status,
            0,
            Accuracy::AlwaysHit,
            3,
            effect,
        )
    };
    let attacker = pokemon(
        "status-user",
        PokemonType::Normal,
        None,
        100,
        100,
        100,
        100,
        100,
        100,
        vec![
            status_move("burn", burn),
            status_move("paralysis", paralysis),
            status_move("toxic", bad_poison),
        ],
    );
    let target = pokemon(
        "status-target",
        PokemonType::Normal,
        None,
        160,
        120,
        100,
        100,
        100,
        20,
        vec![battle_move("reply", PokemonType::Normal, 1, 3, 0)],
    );

    let mut burn_battle = Battle::new(
        team("burn-user", attacker.clone()),
        team("burn-target", target.clone()),
        9,
    )
    .unwrap();
    submit_turn(
        &mut burn_battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(burn_battle.active(Side::Two).physical_attack(), 60);
    assert_eq!(burn_battle.active(Side::Two).current_hp(), 140);

    let mut paralysis_battle = Battle::new(
        team("paralysis-user", attacker.clone()),
        team("paralysis-target", target.clone()),
        9,
    )
    .unwrap();
    submit_turn(
        &mut paralysis_battle,
        Action::UseMove(move_slot(1)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(paralysis_battle.active(Side::Two).effective_speed(), 5);

    let mut poison_battle = Battle::new(
        team("poison-user", attacker),
        team("poison-target", target),
        9,
    )
    .unwrap();
    submit_turn(
        &mut poison_battle,
        Action::UseMove(move_slot(2)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(poison_battle.active(Side::Two).current_hp(), 150);
    submit_turn(
        &mut poison_battle,
        Action::UseMove(move_slot(0)),
        Action::UseMove(move_slot(0)),
    );
    assert_eq!(poison_battle.active(Side::Two).current_hp(), 130);
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
                None,
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
