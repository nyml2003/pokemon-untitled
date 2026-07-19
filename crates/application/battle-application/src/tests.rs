use super::*;

fn move_slot(index: usize) -> MoveSlot {
    MoveSlot::new(index).unwrap()
}

fn team(prefix: &str, speed: u16) -> Team {
    let members = (0..TEAM_SIZE)
        .map(|index| {
            let battle_move = Move::new(
                MoveId::new(format!("{prefix}-move-{index}")).unwrap(),
                "Tackle",
                PokemonType::Normal,
                40,
                Accuracy::AlwaysHit,
                35,
                35,
                0,
            )
            .unwrap();
            Pokemon::new(
                PokemonId::new(format!("{prefix}-{index}")).unwrap(),
                format!("{prefix}-{index}"),
                50,
                PokemonType::Normal,
                None,
                100,
                100,
                BattleStats::new(80, 80, 80, 80, speed).unwrap(),
                vec![battle_move],
            )
            .unwrap()
        })
        .collect();
    Team::new(members).unwrap()
}

fn battle_move(id: &str, power: u16, accuracy: Accuracy, pp: u8) -> Move {
    Move::new(
        MoveId::new(id).unwrap(),
        id,
        PokemonType::Normal,
        power,
        accuracy,
        pp.max(1),
        pp,
        0,
    )
    .unwrap()
}

fn pokemon(
    id: &str,
    max_hp: u32,
    current_hp: u32,
    attack: u16,
    defense: u16,
    speed: u16,
    moves: Vec<Move>,
) -> Pokemon {
    Pokemon::new(
        PokemonId::new(id).unwrap(),
        id,
        50,
        PokemonType::Normal,
        None,
        max_hp,
        current_hp,
        BattleStats::new(attack, defense, attack, defense, speed).unwrap(),
        moves,
    )
    .unwrap()
}

fn team_with_lead(prefix: &str, lead: Pokemon, bench_hp: u32) -> Team {
    let mut members = vec![lead];
    for index in 1..TEAM_SIZE {
        members.push(pokemon(
            &format!("{prefix}-{index}"),
            100,
            bench_hp,
            50,
            50,
            10,
            vec![battle_move(
                &format!("{prefix}-move-{index}"),
                40,
                Accuracy::AlwaysHit,
                10,
            )],
        ));
    }
    Team::new(members).unwrap()
}

fn application() -> (BattleApplication, BattlePerspective, BattlePerspective) {
    let application = BattleApplication::new(team("one", 80), team("two", 60), 42).unwrap();
    let (one, two) = application.perspectives();
    (application, one, two)
}

#[test]
fn creation_rejects_a_team_without_a_conscious_pokemon() {
    let fainted_members = team("fainted", 80)
        .members()
        .iter()
        .map(|pokemon| {
            Pokemon::new(
                pokemon.id().clone(),
                pokemon.name(),
                pokemon.level(),
                pokemon.primary_type(),
                pokemon.secondary_type(),
                pokemon.max_hp(),
                0,
                pokemon.stats(),
                pokemon.moves().to_vec(),
            )
            .unwrap()
        })
        .collect();
    let fainted = Team::new(fainted_members).unwrap();

    assert!(matches!(
        BattleApplication::new(fainted, team("living", 60), 42),
        Err(BattleError::NoLivingPokemon { side: Side::One })
    ));
}

#[test]
fn opening_observation_reveals_own_team_and_only_the_opponents_lead() {
    let (application, one, _) = application();

    let observation = application.observe(&one).unwrap();

    assert_eq!(observation.viewer(), Side::One);
    assert_eq!(observation.turn(), 1);
    assert_eq!(observation.phase(), BattlePhase::Turn);
    assert_eq!(observation.own().active_slot(), TeamSlot::new(0).unwrap());
    assert_eq!(observation.own().members().len(), TEAM_SIZE);
    assert_eq!(observation.own().members()[0].current_hp(), 100);
    assert_eq!(observation.own().members()[0].moves()[0].current_pp(), 35);
    let lead = observation.opponent().active();
    assert_eq!(lead.id().as_str(), "two-0");
    assert_eq!(lead.name(), "two-0");
    assert_eq!(lead.level(), 50);
    assert_eq!(lead.primary_type(), PokemonType::Normal);
    assert_eq!(lead.secondary_type(), None);
    assert_eq!(lead.max_hp(), 100);
    assert_eq!(lead.current_hp(), 100);
    assert!(!lead.is_fainted());
    assert!(lead.revealed_moves().is_empty());
    assert!(observation.opponent().revealed_bench().is_empty());
    assert_eq!(observation.opponent().unrevealed_count(), TEAM_SIZE - 1);
}

#[test]
fn legal_actions_delegate_to_the_same_domain_rules_for_each_side() {
    let (application, one_perspective, two_perspective) = application();

    let one = application.legal_actions(&one_perspective);
    let two = application.legal_actions(&two_perspective);

    assert!(one.contains(&Action::UseMove(move_slot(0))));
    assert!(two.contains(&Action::UseMove(move_slot(0))));
    assert!(one.contains(&Action::Run));
    assert!(two.contains(&Action::Run));
    assert_eq!(one.len(), TEAM_SIZE + 1);
    assert_eq!(two.len(), TEAM_SIZE + 1);
}

#[test]
fn run_finishes_the_battle_before_the_opponent_can_act() {
    let (mut application, one, two) = application();

    let first = application.submit(&one, Action::Run).unwrap();
    let second = application
        .submit(&two, Action::UseMove(move_slot(0)))
        .unwrap();

    assert!(first.is_waiting_for_opponent());
    assert_eq!(
        second.phase(),
        BattlePhase::Finished(BattleOutcome::Escaped(Side::One))
    );
    assert!(second.events().iter().any(|event| matches!(
        event,
        BattleEvent::BattleFinished {
            outcome: ObservedBattleOutcome::Escaped(Participant::Opponent)
        }
    )));
    assert!(!second.events().iter().any(|event| matches!(
        event,
        BattleEvent::MoveUsed {
            participant: Participant::Own,
            ..
        }
    )));
    assert!(application.legal_actions(&one).is_empty());
    assert!(application.legal_actions(&two).is_empty());
}

#[test]
fn submit_returns_only_the_events_created_by_that_command() {
    let (mut application, one, two) = application();

    let first = application
        .submit(&one, Action::UseMove(move_slot(0)))
        .unwrap();
    let second = application
        .submit(&two, Action::UseMove(move_slot(0)))
        .unwrap();

    assert!(first.is_waiting_for_opponent());
    assert_eq!(first.phase(), BattlePhase::Turn);
    assert!(first.events().is_empty());
    assert!(!second.is_waiting_for_opponent());
    assert!(second.events().len() > 1);
    assert_eq!(
        application.event_log(&two).unwrap().len(),
        first.events().len() + second.events().len()
    );
}

#[test]
fn first_command_is_hidden_from_both_observations_until_both_sides_commit() {
    let (mut application, one, two) = application();
    let before_one = application.observe(&one).unwrap();
    let before_two = application.observe(&two).unwrap();
    let before_one_events = application.event_log(&one).unwrap();
    let before_two_events = application.event_log(&two).unwrap();

    let outcome = application
        .submit(&one, Action::UseMove(move_slot(0)))
        .unwrap();

    assert!(outcome.is_waiting_for_opponent());
    assert!(outcome.events().is_empty());
    assert_eq!(application.observe(&one).unwrap(), before_one);
    assert_eq!(application.observe(&two).unwrap(), before_two);
    assert_eq!(application.event_log(&one).unwrap(), before_one_events);
    assert_eq!(application.event_log(&two).unwrap(), before_two_events);
}

#[test]
fn used_opponent_move_is_revealed_without_exposing_pp() {
    let opponent = pokemon(
        "two-0",
        100,
        100,
        80,
        80,
        60,
        vec![
            battle_move("two-move-0", 40, Accuracy::AlwaysHit, 35),
            battle_move("two-move-1", 60, Accuracy::AlwaysHit, 20),
        ],
    );
    let mut application =
        BattleApplication::new(team("one", 80), team_with_lead("two", opponent, 100), 42).unwrap();
    let (one, two) = application.perspectives();

    application
        .submit(&one, Action::UseMove(move_slot(0)))
        .unwrap();
    let outcome = application
        .submit(&two, Action::UseMove(move_slot(0)))
        .unwrap();

    let observation = application.observe(&one).unwrap();
    let lead = observation.opponent().active();
    assert_eq!(observation.own().members()[0].current_hp(), 76);
    assert_eq!(observation.own().members()[0].moves()[0].current_pp(), 34);
    assert_eq!(lead.current_hp(), 76);
    assert_eq!(lead.revealed_moves().len(), 1);
    let revealed_move = &lead.revealed_moves()[0];
    assert_eq!(revealed_move.id().as_str(), "two-move-0");
    assert_eq!(revealed_move.name(), "two-move-0");
    assert_eq!(revealed_move.move_type(), PokemonType::Normal);
    assert_eq!(revealed_move.category(), MoveCategory::Physical);
    assert_eq!(revealed_move.power(), 40);
    assert_eq!(revealed_move.accuracy(), Accuracy::AlwaysHit);
    assert_eq!(revealed_move.priority(), 0);
    let one_pp_events = application
        .event_log(&one)
        .unwrap()
        .into_iter()
        .filter(|event| matches!(event, BattleEvent::OwnPpSpent { .. }))
        .collect::<Vec<_>>();
    assert_eq!(one_pp_events.len(), 1);
    assert!(matches!(
        &one_pp_events[0],
        BattleEvent::OwnPpSpent { pokemon, remaining: 34, .. }
            if pokemon.as_str() == "one-0"
    ));
    let two_pp_events = outcome
        .events()
        .iter()
        .filter(|event| matches!(event, BattleEvent::OwnPpSpent { .. }))
        .collect::<Vec<_>>();
    assert_eq!(two_pp_events.len(), 1);
    assert!(matches!(
        two_pp_events[0],
        BattleEvent::OwnPpSpent { pokemon, remaining: 34, .. }
            if pokemon.as_str() == "two-0"
    ));
    assert!(
        application
            .event_log(&one)
            .unwrap()
            .iter()
            .any(|event| matches!(event, BattleEvent::OpponentCommandCommitted))
    );
}

#[test]
fn switched_opponent_keeps_seen_members_revealed_and_unseen_bench_hidden() {
    let (mut application, one, two) = application();

    application
        .submit(&one, Action::UseMove(move_slot(0)))
        .unwrap();
    application
        .submit(&two, Action::Switch(TeamSlot::new(1).unwrap()))
        .unwrap();

    let observation = application.observe(&one).unwrap();
    assert_eq!(observation.opponent().active().id().as_str(), "two-1");
    assert_eq!(observation.opponent().revealed_bench().len(), 1);
    assert_eq!(
        observation.opponent().revealed_bench()[0].id().as_str(),
        "two-0"
    );
    assert_eq!(observation.opponent().unrevealed_count(), TEAM_SIZE - 2);
    assert!(
        application
            .event_log(&one)
            .unwrap()
            .iter()
            .any(|event| matches!(
                event,
                BattleEvent::OpponentSwitched { pokemon }
                    if pokemon.id().as_str() == "two-1" && pokemon.current_hp() == 100
            ))
    );
    assert!(
        application
            .event_log(&two)
            .unwrap()
            .iter()
            .any(|event| matches!(
                event,
                BattleEvent::OwnSwitched { from, to, pokemon }
                    if *from == TeamSlot::new(0).unwrap()
                        && *to == TeamSlot::new(1).unwrap()
                        && pokemon.id().as_str() == "two-1"
            ))
    );

    application.observe(&two).unwrap();
    application
        .submit(&one, Action::UseMove(move_slot(0)))
        .unwrap();
    application
        .submit(&two, Action::UseMove(move_slot(0)))
        .unwrap();
    let after_move = application.observe(&one).unwrap();
    assert_eq!(after_move.opponent().active().revealed_moves().len(), 1);

    application
        .submit(&one, Action::UseMove(move_slot(0)))
        .unwrap();
    application
        .submit(&two, Action::Switch(TeamSlot::new(0).unwrap()))
        .unwrap();
    let after_return = application.observe(&one).unwrap();
    assert_eq!(after_return.opponent().active().id().as_str(), "two-0");
    assert_eq!(after_return.opponent().revealed_bench().len(), 1);
    assert_eq!(
        after_return.opponent().revealed_bench()[0]
            .revealed_moves()
            .len(),
        1
    );
}

#[test]
fn knockout_observation_shows_damage_and_forced_replacement_without_revealing_skipped_move() {
    let killer = pokemon(
        "killer",
        100,
        100,
        500,
        10,
        100,
        vec![battle_move("winning-hit", 500, Accuracy::AlwaysHit, 10)],
    );
    let victim = pokemon(
        "victim",
        10,
        10,
        10,
        10,
        1,
        vec![battle_move("skipped-move", 500, Accuracy::AlwaysHit, 10)],
    );
    let mut application = BattleApplication::new(
        team_with_lead("killer-bench", killer, 100),
        team_with_lead("victim-bench", victim, 100),
        3,
    )
    .unwrap();
    let (one, two) = application.perspectives();

    application
        .submit(&one, Action::UseMove(move_slot(0)))
        .unwrap();
    application
        .submit(&two, Action::UseMove(move_slot(0)))
        .unwrap();

    let knocked_out = application.observe(&one).unwrap();
    assert_eq!(
        knocked_out.phase(),
        BattlePhase::ForcedReplacement(ReplacementSides::Two)
    );
    assert_eq!(knocked_out.opponent().active().current_hp(), 0);
    assert!(knocked_out.opponent().active().is_fainted());
    assert!(knocked_out.opponent().active().revealed_moves().is_empty());
    assert!(
        application
            .event_log(&one)
            .unwrap()
            .iter()
            .any(|event| matches!(
                event,
                BattleEvent::Fainted {
                    participant: Participant::Opponent,
                    pokemon,
                } if pokemon.as_str() == "victim"
            ))
    );
    assert!(
        application
            .event_log(&one)
            .unwrap()
            .iter()
            .any(|event| matches!(
                event,
                BattleEvent::ForcedReplacement {
                    participant: Participant::Opponent
                }
            ))
    );

    application
        .submit(&two, Action::Switch(TeamSlot::new(1).unwrap()))
        .unwrap();
    let replaced = application.observe(&one).unwrap();
    assert_eq!(replaced.phase(), BattlePhase::Turn);
    assert_eq!(replaced.opponent().active().id().as_str(), "victim-bench-1");
    assert_eq!(replaced.opponent().revealed_bench().len(), 1);
    assert!(replaced.opponent().revealed_bench()[0].is_fainted());
}

#[test]
fn final_knockout_is_visible_as_a_finished_battle() {
    let winner = pokemon(
        "winner",
        100,
        100,
        500,
        10,
        100,
        vec![battle_move("final-hit", 500, Accuracy::AlwaysHit, 10)],
    );
    let loser = pokemon(
        "loser",
        1,
        1,
        1,
        1,
        1,
        vec![battle_move("losing-hit", 1, Accuracy::AlwaysHit, 1)],
    );
    let mut application = BattleApplication::new(
        team_with_lead("winner-bench", winner, 100),
        team_with_lead("loser-bench", loser, 0),
        10,
    )
    .unwrap();
    let (one, two) = application.perspectives();

    application
        .submit(&one, Action::UseMove(move_slot(0)))
        .unwrap();
    application
        .submit(&two, Action::UseMove(move_slot(0)))
        .unwrap();

    assert_eq!(
        application.observe(&one).unwrap().phase(),
        BattlePhase::Finished(BattleOutcome::Winner(Side::One))
    );
    assert!(
        application
            .event_log(&one)
            .unwrap()
            .iter()
            .any(|event| matches!(
                event,
                BattleEvent::BattleFinished {
                    outcome: ObservedBattleOutcome::Winner(Participant::Own)
                }
            ))
    );
}

#[test]
fn struggle_recoil_draw_is_visible_without_revealing_an_unexecuted_opponent_action() {
    let one_lead = pokemon(
        "struggler-one",
        1,
        1,
        100,
        1,
        100,
        vec![battle_move("empty-one", 50, Accuracy::AlwaysHit, 0)],
    );
    let two_lead = pokemon(
        "struggler-two",
        1,
        1,
        100,
        1,
        1,
        vec![battle_move("empty-two", 50, Accuracy::AlwaysHit, 0)],
    );
    let mut application = BattleApplication::new(
        team_with_lead("struggle-one-bench", one_lead, 0),
        team_with_lead("struggle-two-bench", two_lead, 0),
        5,
    )
    .unwrap();
    let (one, two) = application.perspectives();

    application.submit(&one, Action::Struggle).unwrap();
    application.submit(&two, Action::Struggle).unwrap();

    let events = application.event_log(&one).unwrap();
    assert_eq!(
        application.observe(&one).unwrap().phase(),
        BattlePhase::Finished(BattleOutcome::Draw)
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event, BattleEvent::MoveUsed { .. }))
            .count(),
        1
    );
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::MoveUsed {
            participant: Participant::Own,
            used_move: UsedMove::Struggle,
            ..
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::Damage {
            source: DamageSource::Recoil {
                participant: Participant::Own,
                ..
            },
            ..
        }
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        BattleEvent::BattleFinished {
            outcome: ObservedBattleOutcome::Draw
        }
    )));
}

#[test]
fn first_pending_replacement_is_hidden_when_both_sides_must_replace() {
    let one_lead = pokemon(
        "replacement-one",
        1,
        1,
        100,
        1,
        100,
        vec![battle_move(
            "empty-replacement-one",
            50,
            Accuracy::AlwaysHit,
            0,
        )],
    );
    let two_lead = pokemon(
        "replacement-two",
        1,
        1,
        100,
        1,
        1,
        vec![battle_move(
            "empty-replacement-two",
            50,
            Accuracy::AlwaysHit,
            0,
        )],
    );
    let mut application = BattleApplication::new(
        team_with_lead("replacement-one-bench", one_lead, 100),
        team_with_lead("replacement-two-bench", two_lead, 100),
        5,
    )
    .unwrap();
    let (one, two) = application.perspectives();
    application.submit(&one, Action::Struggle).unwrap();
    application.submit(&two, Action::Struggle).unwrap();
    assert_eq!(
        application.observe(&one).unwrap().phase(),
        BattlePhase::ForcedReplacement(ReplacementSides::Both)
    );
    let before_one = application.observe(&one).unwrap();
    let before_two = application.observe(&two).unwrap();
    let before_one_events = application.event_log(&one).unwrap();
    let before_two_events = application.event_log(&two).unwrap();

    let pending = application
        .submit(&one, Action::Switch(TeamSlot::new(1).unwrap()))
        .unwrap();

    assert!(pending.is_waiting_for_opponent());
    assert!(pending.events().is_empty());
    assert_eq!(application.observe(&one).unwrap(), before_one);
    assert_eq!(application.observe(&two).unwrap(), before_two);
    assert_eq!(application.event_log(&one).unwrap(), before_one_events);
    assert_eq!(application.event_log(&two).unwrap(), before_two_events);

    application
        .submit(&two, Action::Switch(TeamSlot::new(1).unwrap()))
        .unwrap();
    assert_eq!(
        application
            .observe(&one)
            .unwrap()
            .own()
            .active_slot()
            .index(),
        1
    );
    assert_eq!(
        application
            .observe(&two)
            .unwrap()
            .own()
            .active_slot()
            .index(),
        1
    );
}

#[test]
fn miss_and_critical_events_remain_public_after_sanitizing() {
    for (seed, expected_miss, expected_critical) in [(1, true, false), (15, false, true)] {
        let attacker = pokemon(
            "random-attacker",
            100,
            100,
            100,
            100,
            100,
            vec![battle_move("inaccurate-hit", 40, Accuracy::Percent(50), 10)],
        );
        let defender = pokemon(
            "random-defender",
            100,
            100,
            10,
            100,
            1,
            vec![battle_move("reply", 1, Accuracy::AlwaysHit, 10)],
        );
        let mut application = BattleApplication::new(
            team_with_lead("random-one", attacker, 100),
            team_with_lead("random-two", defender, 100),
            seed,
        )
        .unwrap();
        let (one, two) = application.perspectives();

        application
            .submit(&one, Action::UseMove(move_slot(0)))
            .unwrap();
        application
            .submit(&two, Action::UseMove(move_slot(0)))
            .unwrap();

        let events = application.event_log(&one).unwrap();
        assert_eq!(
            events.iter().any(|event| matches!(
                event,
                BattleEvent::Missed {
                    participant: Participant::Own,
                    ..
                }
            )),
            expected_miss
        );
        assert_eq!(
            events.iter().any(|event| matches!(
                event,
                BattleEvent::Critical {
                    participant: Participant::Own,
                    ..
                }
            )),
            expected_critical
        );
    }
}

#[test]
fn rejected_submit_does_not_change_observations_or_event_logs() {
    let (mut application, one, two) = application();
    let before_one = application.observe(&one).unwrap();
    let before_two = application.observe(&two).unwrap();
    let before_one_events = application.event_log(&one).unwrap();
    let before_two_events = application.event_log(&two).unwrap();

    let result = application.submit(&one, Action::Struggle);

    assert!(result.is_err());
    assert_eq!(application.observe(&one).unwrap(), before_one);
    assert_eq!(application.observe(&two).unwrap(), before_two);
    assert_eq!(application.event_log(&one).unwrap(), before_one_events);
    assert_eq!(application.event_log(&two).unwrap(), before_two_events);
}

#[test]
fn rejected_submit_does_not_advance_rng_or_change_public_replay() {
    let (mut candidate, candidate_one, candidate_two) = application();
    let (mut control, control_one, control_two) = application();

    assert!(candidate.submit(&candidate_one, Action::Struggle).is_err());
    candidate
        .submit(&candidate_one, Action::UseMove(move_slot(0)))
        .unwrap();
    candidate
        .submit(&candidate_two, Action::UseMove(move_slot(0)))
        .unwrap();
    control
        .submit(&control_one, Action::UseMove(move_slot(0)))
        .unwrap();
    control
        .submit(&control_two, Action::UseMove(move_slot(0)))
        .unwrap();

    assert_eq!(
        candidate.observe(&candidate_one),
        control.observe(&control_one)
    );
    assert_eq!(
        candidate.observe(&candidate_two),
        control.observe(&control_two)
    );
    assert_eq!(
        candidate.event_log(&candidate_one).unwrap(),
        control.event_log(&control_one).unwrap()
    );
    assert_eq!(
        candidate.event_log(&candidate_two).unwrap(),
        control.event_log(&control_two).unwrap()
    );
}

#[test]
fn identical_inputs_produce_identical_observations_and_event_logs() {
    let (mut first, first_one, first_two) = application();
    let (mut second, second_one, second_two) = application();

    first
        .submit(&first_one, Action::UseMove(move_slot(0)))
        .unwrap();
    second
        .submit(&second_one, Action::UseMove(move_slot(0)))
        .unwrap();
    first
        .submit(&first_two, Action::UseMove(move_slot(0)))
        .unwrap();
    second
        .submit(&second_two, Action::UseMove(move_slot(0)))
        .unwrap();

    assert_eq!(first.observe(&first_one), second.observe(&second_one));
    assert_eq!(first.observe(&first_two), second.observe(&second_two));
    assert_eq!(
        first.event_log(&first_one).unwrap(),
        second.event_log(&second_one).unwrap()
    );
    assert_eq!(
        first.event_log(&first_two).unwrap(),
        second.event_log(&second_two).unwrap()
    );
}

#[test]
fn transition_keeps_one_perspective_for_before_events_and_after() {
    let (mut application, one, two) = application();
    let one_checkpoint = application.checkpoint(&one).unwrap();
    let two_checkpoint = application.checkpoint(&two).unwrap();

    application
        .submit(&one, Action::UseMove(move_slot(0)))
        .unwrap();
    application
        .submit(&two, Action::UseMove(move_slot(0)))
        .unwrap();

    let one_transition = application.transition_since(one_checkpoint).unwrap();
    let two_transition = application.transition_since(two_checkpoint).unwrap();
    assert_eq!(one_transition.before().viewer(), Side::One);
    assert_eq!(one_transition.after().viewer(), Side::One);
    assert_eq!(two_transition.before().viewer(), Side::Two);
    assert_eq!(two_transition.after().viewer(), Side::Two);
    assert!(one_transition.events().iter().any(|event| matches!(
        event,
        BattleEvent::MoveUsed {
            participant: Participant::Own,
            pokemon,
            ..
        } if pokemon.as_str().starts_with("one-")
    )));
    assert!(two_transition.events().iter().any(|event| matches!(
        event,
        BattleEvent::MoveUsed {
            participant: Participant::Opponent,
            pokemon,
            ..
        } if pokemon.as_str().starts_with("one-")
    )));
}

#[test]
fn checkpoint_cannot_be_used_with_another_application() {
    let (first, first_one, _) = application();
    let mut checkpoint = first.checkpoint(&first_one).unwrap();
    checkpoint.event_offset = usize::MAX;
    assert_eq!(
        first.transition_since(checkpoint.clone()),
        Err(TransitionError::EventLogRewound)
    );

    let (second, _, _) = application();

    assert_eq!(
        second.transition_since(checkpoint),
        Err(TransitionError::CheckpointOwnerMismatch)
    );
}
