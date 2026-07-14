//! Deterministic roster construction from explicitly supplied game data and seed.

use std::collections::BTreeSet;

use battle_application::{
    Accuracy, MAX_MOVES, Move, MoveCategory, MoveId, Pokemon, PokemonId, PokemonType, StatBlock,
    StatProjectionError, TEAM_SIZE, Team, TrainingValues, ValidationError, calculate_gen3_stats,
};
use game_data::{
    CurrentDataSet, DamageClass as DataDamageClass, MoveId as DataMoveId, PokemonFormId,
    TypeId as DataTypeId,
};

const ROSTER_SIZE: usize = TEAM_SIZE * 2;
const DEMO_LEVEL: u8 = 50;
const LAST_EMERALD_POKEMON: u32 = 386;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RosterError {
    NotEnoughEligiblePokemon {
        required: usize,
        actual: usize,
    },
    MissingPokemon(PokemonFormId),
    MissingMove(DataMoveId),
    MoveNotLearnable {
        pokemon: PokemonFormId,
        battle_move: DataMoveId,
    },
    MissingType(DataTypeId),
    UnsupportedType {
        id: DataTypeId,
        identifier: String,
    },
    MissingMovePower(DataMoveId),
    MissingMovePp(DataMoveId),
    InvalidBattleModel(ValidationError),
    InvalidTraining(StatProjectionError),
}

impl From<ValidationError> for RosterError {
    fn from(error: ValidationError) -> Self {
        Self::InvalidBattleModel(error)
    }
}

impl From<StatProjectionError> for RosterError {
    fn from(error: StatProjectionError) -> Self {
        Self::InvalidTraining(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RosterMember {
    pokemon_form_id: PokemonFormId,
    level: u8,
    move_ids: Vec<DataMoveId>,
    training: TrainingValues,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EligiblePokemon {
    pokemon_form_id: PokemonFormId,
    move_ids: Vec<DataMoveId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DemoSpriteManifest {
    player: Vec<PokemonFormId>,
    opponent: Vec<PokemonFormId>,
}

impl DemoSpriteManifest {
    pub fn player(&self) -> &[PokemonFormId] {
        &self.player
    }

    pub fn opponent(&self) -> &[PokemonFormId] {
        &self.opponent
    }
}

pub fn demo_teams(data: &CurrentDataSet, seed: u64) -> Result<(Team, Team), RosterError> {
    let members = random_members(data, seed)?;
    Ok((
        build_team(data, "player", &members[..TEAM_SIZE])?,
        build_team(data, "rival", &members[TEAM_SIZE..])?,
    ))
}

pub fn sprite_manifest(
    data: &CurrentDataSet,
    seed: u64,
) -> Result<DemoSpriteManifest, RosterError> {
    let members = random_members(data, seed)?;
    Ok(DemoSpriteManifest {
        player: members[..TEAM_SIZE]
            .iter()
            .map(|member| member.pokemon_form_id)
            .collect(),
        opponent: members[TEAM_SIZE..]
            .iter()
            .map(|member| member.pokemon_form_id)
            .collect(),
    })
}

fn random_members(data: &CurrentDataSet, seed: u64) -> Result<Vec<RosterMember>, RosterError> {
    let mut seen_names = BTreeSet::new();
    let mut eligible = data
        .pokemon_iter()
        .filter_map(|pokemon| {
            if pokemon.id.0 > LAST_EMERALD_POKEMON
                || !pokemon.types.iter().all(|id| is_supported_type(data, *id))
                || !seen_names.insert(pokemon.display_name.localized.clone())
            {
                return None;
            }
            let move_ids = compatible_move_ids(data, pokemon.id);
            (move_ids.len() >= MAX_MOVES).then_some(EligiblePokemon {
                pokemon_form_id: pokemon.id,
                move_ids,
            })
        })
        .collect::<Vec<_>>();
    if eligible.len() < ROSTER_SIZE {
        return Err(RosterError::NotEnoughEligiblePokemon {
            required: ROSTER_SIZE,
            actual: eligible.len(),
        });
    }

    let mut rng = RosterRng::new(seed);
    rng.shuffle(&mut eligible);
    eligible
        .into_iter()
        .take(ROSTER_SIZE)
        .map(|mut pokemon| {
            rng.shuffle(&mut pokemon.move_ids);
            pokemon.move_ids.truncate(MAX_MOVES);
            Ok(RosterMember {
                pokemon_form_id: pokemon.pokemon_form_id,
                level: DEMO_LEVEL,
                move_ids: pokemon.move_ids,
                training: TrainingValues::perfect_untrained(),
            })
        })
        .collect()
}

fn compatible_move_ids(data: &CurrentDataSet, pokemon: PokemonFormId) -> Vec<DataMoveId> {
    data.learnset(pokemon)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            data.can_learn_at_level(pokemon, entry.move_id, DEMO_LEVEL)
                .then_some(())?;
            let battle_move = data.move_by_id(entry.move_id)?;
            battle_move.power.filter(|power| *power > 0)?;
            battle_move.pp.filter(|pp| *pp > 0)?;
            is_supported_type(data, battle_move.move_type).then_some(entry.move_id)
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn is_supported_type(data: &CurrentDataSet, id: DataTypeId) -> bool {
    data.type_by_id(id)
        .is_some_and(|record| is_supported_type_name(&record.identifier))
}

fn is_supported_type_name(identifier: &str) -> bool {
    matches!(
        identifier,
        "normal"
            | "fighting"
            | "flying"
            | "poison"
            | "ground"
            | "rock"
            | "bug"
            | "ghost"
            | "steel"
            | "fire"
            | "water"
            | "grass"
            | "electric"
            | "psychic"
            | "ice"
            | "dragon"
            | "dark"
    )
}

fn build_team(
    data: &CurrentDataSet,
    prefix: &str,
    members: &[RosterMember],
) -> Result<Team, RosterError> {
    let members = members
        .iter()
        .map(|member| build_pokemon(data, prefix, member))
        .collect::<Result<Vec<_>, _>>()?;
    Team::new(members).map_err(Into::into)
}

fn build_pokemon(
    data: &CurrentDataSet,
    prefix: &str,
    member: &RosterMember,
) -> Result<Pokemon, RosterError> {
    let record = data
        .pokemon(member.pokemon_form_id)
        .ok_or(RosterError::MissingPokemon(member.pokemon_form_id))?;
    let primary_type = battle_type(data, record.types[0])?;
    let secondary_type = record
        .types
        .get(1)
        .copied()
        .map(|id| battle_type(data, id))
        .transpose()?;
    let moves = member
        .move_ids
        .iter()
        .copied()
        .map(|id| {
            if !data.can_learn_at_level(member.pokemon_form_id, id, member.level) {
                return Err(RosterError::MoveNotLearnable {
                    pokemon: member.pokemon_form_id,
                    battle_move: id,
                });
            }
            battle_move(data, id)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let stats = record.base_stats;
    let calculated = calculate_gen3_stats(
        StatBlock::new(
            stats.hp,
            stats.attack,
            stats.defense,
            stats.special_attack,
            stats.special_defense,
            stats.speed,
        ),
        member.level,
        member.training,
    )?;
    Pokemon::new(
        PokemonId::new(format!("{prefix}-form-{}", member.pokemon_form_id.0))?,
        &record.display_name.localized,
        member.level,
        primary_type,
        secondary_type,
        calculated.max_hp(),
        calculated.max_hp(),
        calculated.battle(),
        moves,
    )
    .map_err(Into::into)
}

fn battle_move(data: &CurrentDataSet, id: DataMoveId) -> Result<Move, RosterError> {
    let record = data.move_by_id(id).ok_or(RosterError::MissingMove(id))?;
    let power = record.power.ok_or(RosterError::MissingMovePower(id))?;
    let pp = record.pp.ok_or(RosterError::MissingMovePp(id))?;
    let accuracy = record
        .accuracy
        .map(Accuracy::percent)
        .transpose()?
        .unwrap_or(Accuracy::AlwaysHit);
    Move::new_with_category(
        MoveId::new(format!("pokeapi-move-{}", id.0))?,
        &record.display_name.localized,
        battle_type(data, record.move_type)?,
        battle_move_category(record.damage_class),
        power,
        accuracy,
        pp,
        pp,
        record.priority,
    )
    .map_err(Into::into)
}

const fn battle_move_category(damage_class: DataDamageClass) -> MoveCategory {
    match damage_class {
        DataDamageClass::Physical => MoveCategory::Physical,
        DataDamageClass::Special => MoveCategory::Special,
        DataDamageClass::Status => MoveCategory::Status,
    }
}

fn battle_type(data: &CurrentDataSet, id: DataTypeId) -> Result<PokemonType, RosterError> {
    let record = data.type_by_id(id).ok_or(RosterError::MissingType(id))?;
    match record.identifier.as_str() {
        "normal" => Ok(PokemonType::Normal),
        "fighting" => Ok(PokemonType::Fighting),
        "flying" => Ok(PokemonType::Flying),
        "poison" => Ok(PokemonType::Poison),
        "ground" => Ok(PokemonType::Ground),
        "rock" => Ok(PokemonType::Rock),
        "bug" => Ok(PokemonType::Bug),
        "ghost" => Ok(PokemonType::Ghost),
        "steel" => Ok(PokemonType::Steel),
        "fire" => Ok(PokemonType::Fire),
        "water" => Ok(PokemonType::Water),
        "grass" => Ok(PokemonType::Grass),
        "electric" => Ok(PokemonType::Electric),
        "psychic" => Ok(PokemonType::Psychic),
        "ice" => Ok(PokemonType::Ice),
        "dragon" => Ok(PokemonType::Dragon),
        "dark" => Ok(PokemonType::Dark),
        identifier => Err(RosterError::UnsupportedType {
            id,
            identifier: identifier.to_owned(),
        }),
    }
}

struct RosterRng {
    state: u64,
}

impl RosterRng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    fn next(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.state = value;
        value.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn shuffle<T>(&mut self, values: &mut [T]) {
        for upper in (1..values.len()).rev() {
            let index = (self.next() % (upper as u64 + 1)) as usize;
            values.swap(upper, index);
        }
    }
}

#[cfg(test)]
mod tests {
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
        CurrentDataSet::new(
            DataSetMetadata {
                schema_version: "current-data-set-v2".into(),
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
                damage_class: DamageClass::Status,
            }],
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
        assert!(matches!(
            battle_move(&minimal_data("normal", None, Some(1), true), DataMoveId(1)),
            Err(RosterError::MissingMovePower(DataMoveId(1)))
        ));
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
}
