use crate::{
    Ability, Accuracy, Battle, BattleError, BattleOutcome, BattlePhase, BattleState, BattleStats,
    BattleUnit, BattleUnitId, FormId, HitPoints, HitPointsPhase, Move, MoveId, NationalDexId,
    PokemonType, ReplacementSides, Side, Species, StatBlock, StatStages, TEAM_SIZE, Team, TeamSlot,
    TypeEffectiveness, ValidationError, VolatileStatus, VolatileStatuses,
};

fn move_id(id: &str) -> MoveId {
    MoveId::new(id).unwrap()
}

fn battle_move(id: &str, move_type: PokemonType, power: u16, pp: u8) -> Move {
    Move::new(
        move_id(id),
        id,
        vec![move_type],
        power,
        Accuracy::AlwaysHit,
        pp,
        pp,
        0,
    )
    .unwrap()
}

fn species(name: &str, move_type: PokemonType) -> Species {
    Species::new(
        name,
        StatBlock::new(45, 49, 49, 65, 65, 45),
        NationalDexId::new(1),
        FormId::new(0),
        vec![move_type],
        vec![Ability::Overgrow],
    )
    .unwrap()
}

fn unit(id: &str, name: &str, move_type: PokemonType, max_hp: u32, current_hp: u32) -> BattleUnit {
    let species = species(name, move_type);
    let unit_id = BattleUnitId::new(id).unwrap();
    let stats = BattleStats::new(49, 49, 65, 65, 45).unwrap();
    let state = BattleState::new(
        50,
        stats,
        max_hp,
        current_hp,
        vec![battle_move(&format!("{id}-move"), move_type, 40, 35)],
        vec![],
        None,
        StatStages::neutral(),
    )
    .unwrap();
    BattleUnit::new(species, unit_id, state).unwrap()
}

fn team(prefix: &str, max_hp: u32, current_hp: u32) -> Team {
    let members = (0..TEAM_SIZE)
        .map(|index| {
            unit(
                &format!("{prefix}-{index}"),
                &format!("{prefix}{index}"),
                PokemonType::Normal,
                max_hp,
                current_hp,
            )
        })
        .collect();
    Team::new(members).unwrap()
}

#[test]
fn battle_unit_id_rejects_empty() {
    assert_eq!(
        BattleUnitId::new("  "),
        Err(ValidationError::EmptyPokemonId)
    );
}

#[test]
fn species_rejects_empty_name_or_types() {
    assert_eq!(
        Species::new(
            " ",
            StatBlock::new(1, 1, 1, 1, 1, 1),
            NationalDexId::new(1),
            FormId::new(0),
            vec![PokemonType::Normal],
            vec![],
        ),
        Err(ValidationError::EmptyPokemonName)
    );
    assert_eq!(
        Species::new(
            "mon",
            StatBlock::new(1, 1, 1, 1, 1, 1),
            NationalDexId::new(1),
            FormId::new(0),
            vec![],
            vec![],
        ),
        Err(ValidationError::EmptySpeciesType)
    );
}

#[test]
fn battle_state_validates_level_hp_and_moves() {
    let stats = BattleStats::new(1, 1, 1, 1, 1).unwrap();
    assert_eq!(
        BattleState::new(
            0,
            stats,
            10,
            10,
            vec![],
            vec![],
            None,
            StatStages::neutral()
        ),
        Err(ValidationError::InvalidLevel { level: 0 })
    );
    assert_eq!(
        BattleState::new(
            50,
            stats,
            10,
            11,
            vec![battle_move("m", PokemonType::Normal, 40, 35)],
            vec![],
            None,
            StatStages::neutral(),
        ),
        Err(ValidationError::CurrentHpExceedsMax {
            current: 11,
            max: 10,
        })
    );
    assert_eq!(
        BattleState::new(
            50,
            stats,
            10,
            10,
            vec![
                battle_move("a", PokemonType::Normal, 40, 35),
                battle_move("a", PokemonType::Normal, 40, 35),
            ],
            vec![],
            None,
            StatStages::neutral(),
        ),
        Err(ValidationError::DuplicateMoveId { id: move_id("a") })
    );
}

#[test]
fn move_constructor_validates_fields() {
    assert_eq!(
        Move::new(
            move_id("m"),
            " ",
            vec![PokemonType::Normal],
            40,
            Accuracy::AlwaysHit,
            35,
            35,
            0
        ),
        Err(ValidationError::EmptyMoveName)
    );
    assert_eq!(
        Move::new(
            move_id("m"),
            "m",
            vec![],
            40,
            Accuracy::AlwaysHit,
            35,
            35,
            0
        ),
        Err(ValidationError::EmptyMoveType)
    );
    assert_eq!(
        Move::new(
            move_id("m"),
            "m",
            vec![PokemonType::Normal],
            0,
            Accuracy::AlwaysHit,
            35,
            35,
            0,
        ),
        Err(ValidationError::ZeroMovePower)
    );
    assert_eq!(
        Move::new(
            move_id("m"),
            "m",
            vec![PokemonType::Normal],
            40,
            Accuracy::AlwaysHit,
            0,
            0,
            0,
        ),
        Err(ValidationError::ZeroMaxPp)
    );
}

#[test]
fn move_accessors_return_validated_values() {
    let battle_move = Move::new(
        move_id("ember"),
        "Ember",
        vec![PokemonType::Fire],
        40,
        Accuracy::Percent(100),
        25,
        24,
        1,
    )
    .unwrap();
    assert_eq!(battle_move.id().as_str(), "ember");
    assert_eq!(battle_move.name(), "Ember");
    assert_eq!(battle_move.move_types(), &[PokemonType::Fire]);
    assert_eq!(battle_move.power(), 40);
    assert_eq!(battle_move.accuracy(), Accuracy::Percent(100));
    assert_eq!(battle_move.max_pp(), 25);
    assert_eq!(battle_move.current_pp(), 24);
    assert_eq!(battle_move.priority(), 1);
    assert!(battle_move.effects().is_empty());
}

#[test]
fn team_validates_size_and_unique_ids() {
    assert_eq!(
        Team::new(vec![]),
        Err(ValidationError::InvalidTeamSize { count: 0 })
    );
    let mut members = vec![unit("a", "a", PokemonType::Normal, 10, 10)];
    for index in 1..TEAM_SIZE {
        members.push(unit(
            &format!("a-{index}"),
            "a",
            PokemonType::Normal,
            10,
            10,
        ));
    }
    let duplicate = members
        .iter()
        .map(|member| {
            let species = member.species().clone();
            let state = member.state().clone();
            BattleUnit::new(species, BattleUnitId::new("a").unwrap(), state).unwrap()
        })
        .collect();
    assert!(matches!(
        Team::new(duplicate),
        Err(ValidationError::DuplicatePokemonId { .. })
    ));
}

#[test]
fn battle_rejects_empty_teams() {
    let fainted = team("fainted", 10, 0);
    let living = team("living", 10, 10);
    assert_eq!(
        Battle::new(fainted, living, 42),
        Err(BattleError::NoLivingPokemon { side: Side::One })
    );
}

#[test]
fn battle_initial_state_is_turn_one() {
    let battle = Battle::new(team("one", 10, 10), team("two", 10, 10), 42).unwrap();
    assert_eq!(battle.phase(), BattlePhase::Turn);
    assert_eq!(battle.turn_number(), 1);
    assert!(battle.events().is_empty());
    assert_eq!(battle.weather(), None);
    assert_eq!(battle.team(Side::One).members().len(), TEAM_SIZE);
    assert_eq!(battle.active_slot(Side::Two), TeamSlot::new(0).unwrap());
    assert!(!battle.active(Side::One).is_fainted());
}

#[test]
fn volatile_statuses_store_values_by_kind() {
    let mut statuses = VolatileStatuses::default();
    assert!(statuses.is_empty());
    statuses.set(VolatileStatus::Substitute, 25);
    assert_eq!(statuses.get(VolatileStatus::Substitute), Some(25));
    assert_eq!(statuses.get(VolatileStatus::ProtectStreak), None);
    statuses.remove(VolatileStatus::Substitute);
    assert!(statuses.is_empty());
}

#[test]
fn side_index_round_trips() {
    assert_eq!(Side::One, Side::One);
    assert_ne!(Side::One, Side::Two);
}

#[test]
fn effectiveness_and_outcome_enums_cover_expected_variants() {
    assert_eq!(TypeEffectiveness::Immune, TypeEffectiveness::Immune);
    assert_eq!(
        BattleOutcome::Winner(Side::One),
        BattleOutcome::Winner(Side::One)
    );
    assert!(ReplacementSides::Both.contains(Side::One));
    assert!(!BattlePhase::Finished(BattleOutcome::Draw).requires_replacement(Side::One));
}

#[test]
fn hit_points_derive_phase_across_health_bands() {
    use HitPointsPhase::{Full, High, Low, Mid, Zero};

    let full = HitPoints::new(100, 100).unwrap();
    assert_eq!(full.phase(), Full);
    assert_eq!(full.percent(), 100);
    assert!(!full.is_zero());

    assert_eq!(HitPoints::new(60, 100).unwrap().phase(), High);
    assert_eq!(HitPoints::new(50, 100).unwrap().phase(), Mid);
    assert_eq!(HitPoints::new(21, 100).unwrap().phase(), Mid);
    assert_eq!(HitPoints::new(20, 100).unwrap().phase(), Low);
    assert_eq!(HitPoints::new(19, 100).unwrap().phase(), Low);
    assert_eq!(HitPoints::new(0, 100).unwrap().phase(), Zero);
    assert!(HitPoints::new(0, 100).unwrap().is_zero());
}

#[test]
fn hit_points_validate_current_against_max() {
    assert_eq!(HitPoints::new(0, 0), Err(ValidationError::ZeroMaxHp));
    assert_eq!(
        HitPoints::new(11, 10),
        Err(ValidationError::CurrentHpExceedsMax {
            current: 11,
            max: 10
        })
    );
}

#[test]
fn hit_points_damage_and_heal_stay_within_bounds_and_report_actual() {
    let (damaged, actual) = HitPoints::new(50, 100).unwrap().damage(30);
    assert_eq!(actual, 30);
    assert_eq!(damaged.current(), 20);

    let (overkill, actual) = damaged.damage(100);
    assert_eq!(actual, 20);
    assert_eq!(overkill.current(), 0);

    let (healed, actual) = overkill.heal(40);
    assert_eq!(actual, 40);
    assert_eq!(healed.current(), 40);

    let (capped, actual) = healed.heal(1000);
    assert_eq!(actual, 60);
    assert_eq!(capped.current(), 100);
    assert_eq!(capped.phase(), HitPointsPhase::Full);
}

#[test]
fn hit_points_lock_is_explicit_and_does_not_change_health() {
    let hp = HitPoints::new(40, 100).unwrap();
    let locked = hp.lock();
    assert!(locked.is_locked());
    assert_eq!(locked.current(), 40);
    assert_eq!(locked.percent(), 40);
    assert!(!locked.unlock().is_locked());
    assert_eq!(locked.unlock(), hp);
}
