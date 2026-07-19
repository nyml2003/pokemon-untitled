use super::{
    CurrentDataSet, DamageClass, DataLoadError, GEN3_FIRST_DEX, GEN3_LAST_DEX, MoveId, PokedexData,
    PokemonFormId, TypeId,
};

#[test]
fn embedded_pokedex_covers_the_canonical_gen3_fronts() {
    let pokedex = PokedexData::embedded_gen3().unwrap();
    assert_eq!(pokedex.entries().len(), 386);
    assert_eq!(pokedex.entries()[0].national_dex, GEN3_FIRST_DEX);
    assert_eq!(
        pokedex.entries().last().unwrap().national_dex,
        GEN3_LAST_DEX
    );
    for entry in pokedex.entries() {
        assert_eq!(
            entry.front_asset,
            format!("pokemon/{:04}/form/00/normal/front/00", entry.national_dex)
        );
        assert!(!entry.localized_name.is_empty());
        assert!(!entry.types.is_empty());
    }
}

fn fixture() -> Vec<u8> {
    serde_json::to_vec(&CurrentDataSet {
        metadata: super::DataSetMetadata {
            schema_version: "current-data-set-v4".into(),
            source_repository: "test".into(),
            source_commit: "test".into(),
            generator_version: "test".into(),
            locale: "zh-Hans".into(),
            version_group: "emerald".into(),
        },
        pokemon: vec![super::PokemonRecord {
            id: PokemonFormId(1),
            species_id: super::SpeciesId(1),
            identifier: "bulbasaur".into(),
            is_default: true,
            base_stats: super::BaseStats {
                hp: 45,
                attack: 49,
                defense: 49,
                special_attack: 65,
                special_defense: 65,
                speed: 45,
            },
            types: vec![TypeId(12), TypeId(4)],
            abilities: vec![super::PokemonAbility {
                ability_id: super::AbilityId(1),
                is_hidden: false,
                slot: 1,
            }],
            display_name: super::LocalizedName {
                localized: "妙蛙种子".into(),
                english: "Bulbasaur".into(),
            },
            learnset: vec![super::LearnsetEntry {
                move_id: MoveId(1),
                method: super::MoveLearnMethod::LevelUp,
                level: Some(1),
                order: Some(1),
            }],
        }],
        moves: vec![super::MoveRecord {
            id: MoveId(1),
            identifier: "pound".into(),
            display_name: super::LocalizedName {
                localized: "拍击".into(),
                english: "Pound".into(),
            },
            move_type: TypeId(1),
            power: Some(40),
            accuracy: Some(100),
            pp: Some(35),
            priority: 0,
            damage_class: DamageClass::Physical,
            effect_id: Some(1),
            effect_chance: None,
        }],
        abilities: vec![super::AbilityRecord {
            id: super::AbilityId(1),
            identifier: "stench".into(),
            display_name: super::LocalizedName {
                localized: "恶臭".into(),
                english: "Stench".into(),
            },
        }],
        types: vec![
            super::TypeRecord {
                id: TypeId(1),
                identifier: "normal".into(),
                display_name: super::LocalizedName {
                    localized: "一般".into(),
                    english: "Normal".into(),
                },
            },
            super::TypeRecord {
                id: TypeId(4),
                identifier: "poison".into(),
                display_name: super::LocalizedName {
                    localized: "毒".into(),
                    english: "Poison".into(),
                },
            },
            super::TypeRecord {
                id: TypeId(12),
                identifier: "grass".into(),
                display_name: super::LocalizedName {
                    localized: "草".into(),
                    english: "Grass".into(),
                },
            },
        ],
    })
    .unwrap()
}

fn invalid(mut change: impl FnMut(&mut serde_json::Value)) -> DataLoadError {
    let mut value: serde_json::Value = serde_json::from_slice(&fixture()).unwrap();
    change(&mut value);
    CurrentDataSet::from_json(&serde_json::to_vec(&value).unwrap()).unwrap_err()
}

#[test]
fn loads_and_queries_sorted_records() {
    let data = CurrentDataSet::from_json(&fixture()).unwrap();
    assert_eq!(
        data.pokemon(PokemonFormId(1))
            .unwrap()
            .display_name
            .localized,
        "妙蛙种子"
    );
    assert_eq!(data.move_by_id(MoveId(1)).unwrap().power, Some(40));
    assert!(data.can_learn(PokemonFormId(1), MoveId(1)));
    assert!(data.can_learn_at_level(PokemonFormId(1), MoveId(1), 1));
    assert!(!data.can_learn_at_level(PokemonFormId(1), MoveId(1), 0));
}

#[test]
fn rejects_unknown_schema() {
    let mut bytes = fixture();
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["metadata"]["schema_version"] = serde_json::Value::String("v9".into());
    bytes = serde_json::to_vec(&value).unwrap();
    assert!(matches!(
        CurrentDataSet::from_json(&bytes),
        Err(DataLoadError::UnsupportedSchema(_))
    ));
}

#[test]
fn rejects_every_cross_record_and_metadata_violation() {
    let errors = [
        invalid(|value| {
            let pokemon = value["pokemon"][0].clone();
            value["pokemon"] = serde_json::json!([pokemon.clone(), pokemon]);
        }),
        invalid(|value| value["metadata"]["version_group"] = "".into()),
        invalid(|value| value["pokemon"][0]["types"] = serde_json::json!([])),
        invalid(|value| value["pokemon"][0]["types"] = serde_json::json!([1, 4, 12])),
        invalid(|value| value["pokemon"][0]["types"] = serde_json::json!([4, 4])),
        invalid(|value| value["pokemon"][0]["types"] = serde_json::json!([999])),
        invalid(|value| value["pokemon"][0]["learnset"][0]["move_id"] = 999.into()),
        invalid(|value| {
            let entry = value["pokemon"][0]["learnset"][0].clone();
            value["pokemon"][0]["learnset"] = serde_json::json!([entry.clone(), entry]);
        }),
        invalid(|value| value["moves"][0]["move_type"] = 999.into()),
        invalid(|value| value["moves"][0]["accuracy"] = 0.into()),
    ];
    for error in errors {
        assert!(matches!(error, DataLoadError::InvalidRecord(_)));
        assert!(!error.to_string().is_empty());
    }

    for error in [
        DataLoadError::MalformedData("bad json".into()),
        DataLoadError::UnsupportedSchema("v9".into()),
    ] {
        assert!(!error.to_string().is_empty());
    }
}

#[test]
fn embedded_data_matches_the_pinned_snapshot() {
    let data = CurrentDataSet::embedded().unwrap();
    assert_eq!(
        data.metadata().source_commit,
        "d638fe7791214a8d3c3282e2a3113eea7cfef288"
    );
    assert_eq!(data.metadata().version_group, "emerald");
    assert_eq!(data.pokemon_iter().count(), 1_351);
    assert_eq!(data.move_iter().count(), 937);
    assert_eq!(data.type_iter().count(), 21);
    assert!(data.can_learn(PokemonFormId(1), MoveId(33)));
    assert!(data.can_learn(PokemonFormId(1), MoveId(22)));
    assert!(!data.can_learn_at_level(PokemonFormId(1), MoveId(22), 9));
    assert!(data.can_learn_at_level(PokemonFormId(1), MoveId(22), 10));
}
