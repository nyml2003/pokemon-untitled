use std::collections::BTreeSet;

use battle_application::{
    MoveCategory, StatBlock, StatProjectionError, TrainingValues, ValidationError,
    calculate_gen3_stats,
};
use game_data::{
    BaseStats, CurrentDataSet, DamageClass, DataSetMetadata, LearnsetEntry, LocalizedName,
    MoveId as DataMoveId, MoveLearnMethod, MoveRecord, PokemonFormId, PokemonRecord, SpeciesId,
    TypeId, TypeRecord,
};

use super::*;

fn minimal_data(
    type_name: &str,
    power: Option<u16>,
    pp: Option<u8>,
    learnable: bool,
) -> CurrentDataSet {
    minimal_data_with_category(type_name, power, pp, learnable, DamageClass::Status)
}

fn minimal_data_with_category(
    type_name: &str,
    power: Option<u16>,
    pp: Option<u8>,
    learnable: bool,
    damage_class: DamageClass,
) -> CurrentDataSet {
    CurrentDataSet::new(
        DataSetMetadata {
            schema_version: "current-data-set-v4".into(),
            source_repository: "test".into(),
            source_commit: "test".into(),
            generator_version: "test".into(),
            locale: "en".into(),
            version_group: "emerald".into(),
        },
        vec![PokemonRecord {
            id: PokemonFormId(1),
            species_id: SpeciesId(1),
            identifier: "one".into(),
            is_default: true,
            base_stats: BaseStats {
                hp: 45,
                attack: 49,
                defense: 49,
                special_attack: 65,
                special_defense: 65,
                speed: 45,
            },
            types: vec![TypeId(1)],
            abilities: vec![],
            display_name: LocalizedName {
                localized: "One".into(),
                english: "One".into(),
            },
            learnset: learnable
                .then_some(LearnsetEntry {
                    move_id: DataMoveId(1),
                    method: MoveLearnMethod::LevelUp,
                    level: Some(1),
                    order: Some(1),
                })
                .into_iter()
                .collect(),
        }],
        vec![MoveRecord {
            id: DataMoveId(1),
            identifier: "move".into(),
            display_name: LocalizedName {
                localized: "Move".into(),
                english: "Move".into(),
            },
            move_type: TypeId(1),
            power,
            accuracy: None,
            pp,
            priority: 0,
            damage_class,
            effect_id: Some(1),
            effect_chance: None,
        }],
        vec![],
        vec![TypeRecord {
            id: TypeId(1),
            identifier: type_name.into(),
            display_name: LocalizedName {
                localized: type_name.into(),
                english: type_name.into(),
            },
        }],
    )
    .unwrap()
}

#[test]
fn seeded_roster_has_twelve_unique_pokemon_with_four_unique_learnset_moves() {
    let data = CurrentDataSet::embedded().unwrap();
    let members = random_members(&data, 0xA2B3_C4D5).unwrap();

    assert_eq!(members.len(), ROSTER_SIZE);
    assert_eq!(
        members
            .iter()
            .map(|member| member.pokemon_form_id)
            .collect::<BTreeSet<_>>()
            .len(),
        ROSTER_SIZE
    );
    for member in &members {
        assert_eq!(member.move_ids.len(), MAX_MOVES);
        assert_eq!(
            member
                .move_ids
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .len(),
            MAX_MOVES
        );
        assert!(
            member
                .move_ids
                .iter()
                .all(|move_id| data.can_learn(member.pokemon_form_id, *move_id))
        );
        assert_eq!(member.training, TrainingValues::perfect_untrained());
    }
}

#[test]
fn random_roster_uses_the_complete_gen3_national_range_and_default_forms() {
    for national_dex in [FIRST_NATIONAL_POKEMON, LAST_GEN3_POKEMON] {
        assert!(is_gen3_default_form(national_dex, national_dex));
    }
    for (species_id, form_id) in [
        (0, 0),
        (LAST_GEN3_POKEMON + 1, LAST_GEN3_POKEMON + 1),
        (25, 1_025),
    ] {
        assert!(!is_gen3_default_form(species_id, form_id));
    }

    let data = CurrentDataSet::embedded().unwrap();
    for member in random_members(&data, 42).unwrap() {
        assert!(is_gen3_default_form(
            data.pokemon(member.pokemon_form_id).unwrap().species_id.0,
            member.pokemon_form_id.0,
        ));
    }
}

#[test]
fn equal_seeds_are_reproducible_and_different_seeds_change_the_roster() {
    let data = CurrentDataSet::embedded().unwrap();
    let first = random_members(&data, 7).unwrap();
    let repeated = random_members(&data, 7).unwrap();
    let different = random_members(&data, 8).unwrap();

    assert_eq!(first, repeated);
    assert_ne!(first, different);
}

#[test]
fn generated_members_build_two_valid_battle_teams() {
    let data = CurrentDataSet::embedded().unwrap();
    let roster = random_members(&data, 42).unwrap();
    let (player, opponent) = demo_teams(&data, 42).unwrap();
    let battle_members = player.members().iter().chain(opponent.members());

    assert_eq!(player.members().len(), 6);
    assert_eq!(opponent.members().len(), 6);
    assert_eq!(
        battle_members
            .clone()
            .map(|pokemon| pokemon.name())
            .collect::<BTreeSet<_>>()
            .len(),
        ROSTER_SIZE
    );
    assert!(
        battle_members
            .clone()
            .all(|pokemon| pokemon.moves().len() == MAX_MOVES)
    );
    for (member, pokemon) in roster.iter().zip(battle_members) {
        let base = data.pokemon(member.pokemon_form_id).unwrap().base_stats;
        let expected = calculate_gen3_stats(
            StatBlock::new(
                base.hp,
                base.attack,
                base.defense,
                base.special_attack,
                base.special_defense,
                base.speed,
            ),
            member.level,
            member.training,
        )
        .unwrap();
        assert_eq!(pokemon.max_hp(), expected.max_hp());
        assert_eq!(pokemon.stats(), expected.battle());
        for (move_id, battle_move) in member.move_ids.iter().zip(pokemon.moves()) {
            let expected = match data.move_by_id(*move_id).unwrap().damage_class {
                DamageClass::Physical => MoveCategory::Physical,
                DamageClass::Special => MoveCategory::Special,
                DamageClass::Status => MoveCategory::Status,
            };
            assert_eq!(battle_move.category(), expected);
        }
    }
}

#[test]
fn roster_failures_and_private_mappings_are_explicit() {
    let data = minimal_data("normal", Some(1), Some(1), true);
    assert!(matches!(
        random_members(&data, 1),
        Err(RosterError::NotEnoughEligiblePokemon { .. })
    ));
    assert!(matches!(
        build_pokemon(
            &data,
            "test",
            &RosterMember {
                pokemon_form_id: PokemonFormId(999),
                level: 50,
                move_ids: vec![],
                training: TrainingValues::perfect_untrained(),
            }
        ),
        Err(RosterError::MissingPokemon(PokemonFormId(999)))
    ));

    let not_learnable = minimal_data("normal", Some(1), Some(1), false);
    assert!(matches!(
        build_pokemon(
            &not_learnable,
            "test",
            &RosterMember {
                pokemon_form_id: PokemonFormId(1),
                level: 50,
                move_ids: vec![DataMoveId(1)],
                training: TrainingValues::perfect_untrained(),
            }
        ),
        Err(RosterError::MoveNotLearnable { .. })
    ));
    assert!(matches!(
        battle_move(&data, DataMoveId(999)),
        Err(RosterError::MissingMove(DataMoveId(999)))
    ));
    let missing_power =
        minimal_data_with_category("normal", None, Some(1), true, DamageClass::Physical);
    assert!(matches!(
        battle_move(&missing_power, DataMoveId(1)),
        Err(RosterError::MissingMovePower(DataMoveId(1)))
    ));
    assert!(battle_move(&minimal_data("normal", None, Some(1), true), DataMoveId(1)).is_ok());
    assert!(matches!(
        battle_move(&minimal_data("normal", Some(1), None, true), DataMoveId(1)),
        Err(RosterError::MissingMovePp(DataMoveId(1)))
    ));
    assert!(matches!(
        battle_type(&data, TypeId(999)),
        Err(RosterError::MissingType(TypeId(999)))
    ));
    assert!(matches!(
        battle_type(&minimal_data("fairy", Some(1), Some(1), true), TypeId(1)),
        Err(RosterError::UnsupportedType { .. })
    ));
    assert_eq!(
        battle_move_category(DamageClass::Status),
        MoveCategory::Status
    );

    assert!(matches!(
        RosterError::from(ValidationError::EmptyPokemonId),
        RosterError::InvalidBattleModel(_)
    ));
    assert!(matches!(
        RosterError::from(StatProjectionError::InvalidLevel { value: 0 }),
        RosterError::InvalidTraining(_)
    ));
    assert_ne!(RosterRng::new(0).next(), 0);
}

#[test]
fn supported_non_damage_effects_keep_their_gen_three_semantics() {
    let mut record = minimal_data("normal", None, Some(20), true)
        .move_by_id(DataMoveId(1))
        .unwrap()
        .clone();

    record.effect_id = Some(19);
    assert!(matches!(
        move_effect(&record),
        Some(MoveEffect::ChangeStages {
            target: EffectTarget::Opponent,
            changes,
        }) if changes.get(BattleStat::Attack) == -1
    ));

    record.effect_id = Some(212);
    assert!(matches!(
        move_effect(&record),
        Some(MoveEffect::ChangeStages {
            target: EffectTarget::User,
            changes,
        }) if changes.get(BattleStat::SpecialAttack) == 1
            && changes.get(BattleStat::SpecialDefense) == 1
    ));

    record.effect_id = Some(33);
    assert_eq!(move_effect(&record), MoveEffect::heal_user(1, 2).ok());
    record.effect_id = Some(42);
    assert_eq!(
        move_effect(&record),
        Some(MoveEffect::fixed_damage_amount(40))
    );
    record.effect_id = Some(88);
    assert_eq!(
        move_effect(&record),
        Some(MoveEffect::fixed_damage_user_level())
    );
    record.effect_id = Some(131);
    assert_eq!(
        move_effect(&record),
        Some(MoveEffect::fixed_damage_amount(20))
    );
    record.effect_id = Some(32);
    record.effect_chance = Some(30);
    assert_eq!(move_effect(&record), MoveEffect::flinch_target(30).ok());
    record.effect_id = Some(159);
    record.effect_chance = Some(100);
    assert_eq!(move_effect(&record), MoveEffect::flinch_target(100).ok());
    record.effect_id = Some(112);
    assert_eq!(move_effect(&record), Some(MoveEffect::protect_user()));
    record.effect_id = Some(80);
    assert_eq!(move_effect(&record), Some(MoveEffect::create_substitute()));
    record.effect_id = Some(26);
    assert_eq!(move_effect(&record), Some(MoveEffect::haze()));
    record.effect_id = Some(38);
    assert_eq!(move_effect(&record), Some(MoveEffect::rest()));
    record.effect_id = Some(194);
    assert_eq!(move_effect(&record), Some(MoveEffect::refresh()));
    record.effect_id = Some(4);
    assert_eq!(move_effect(&record), MoveEffect::drain_user(1, 2).ok());
    record.effect_id = Some(49);
    assert_eq!(move_effect(&record), MoveEffect::recoil_user(1, 4).ok());
    record.effect_id = Some(199);
    assert_eq!(move_effect(&record), MoveEffect::recoil_user(1, 3).ok());
    record.effect_chance = Some(10);
    record.effect_id = Some(73);
    assert_eq!(
        move_effect(&record),
        MoveEffect::change_stages_with_chance(
            EffectTarget::Opponent,
            StageChanges::single(BattleStat::SpecialDefense, -1).unwrap(),
            10,
        )
        .ok()
    );
    record.effect_id = Some(137);
    assert_eq!(
        move_effect(&record),
        Some(MoveEffect::start_weather(Weather::Rain))
    );
    record.effect_id = Some(144);
    assert_eq!(move_effect(&record), Some(MoveEffect::copy_target_stages()));
    record.identifier = "thunder".into();
    assert_eq!(
        weather_accuracy(&record),
        Some(WeatherAccuracyModifier::Thunder)
    );
    record.identifier = "weather-ball".into();
    assert_eq!(
        weather_move(&record),
        Some(WeatherMoveModifier::WeatherBall)
    );
    record.effect_id = Some(58);
    assert_eq!(move_effect(&record), None);
}
