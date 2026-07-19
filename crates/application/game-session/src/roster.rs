//! Deterministic roster construction from explicitly supplied game data and seed.

use std::collections::BTreeSet;

use battle_application::{
    Ability, Accuracy, BattleStat, EffectTarget, MAX_MOVES, MajorStatusKind, Move, MoveCategory,
    MoveEffect, MoveId, Pokemon, PokemonId, PokemonType, StageChanges, StatBlock,
    StatProjectionError, TEAM_SIZE, Team, TrainingValues, ValidationError, Weather,
    WeatherAccuracyModifier, WeatherMoveModifier, calculate_gen3_stats,
};
use game_data::{
    CurrentDataSet, DamageClass as DataDamageClass, MoveId as DataMoveId, PokemonFormId,
    TypeId as DataTypeId,
};

const ROSTER_SIZE: usize = TEAM_SIZE * 2;
const DEMO_LEVEL: u8 = 50;
const FIRST_NATIONAL_POKEMON: u32 = 1;
const LAST_GEN3_POKEMON: u32 = 386;

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
            if !is_gen3_default_form(pokemon.species_id.0, pokemon.id.0)
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

const fn is_gen3_default_form(species_id: u32, form_id: u32) -> bool {
    species_id >= FIRST_NATIONAL_POKEMON && species_id <= LAST_GEN3_POKEMON && form_id == species_id
}

fn compatible_move_ids(data: &CurrentDataSet, pokemon: PokemonFormId) -> Vec<DataMoveId> {
    data.learnset(pokemon)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            data.can_learn_at_level(pokemon, entry.move_id, DEMO_LEVEL)
                .then_some(())?;
            let battle_move = data.move_by_id(entry.move_id)?;
            battle_move.pp.filter(|pp| *pp > 0)?;
            (battle_move.power.is_some() || move_effect(battle_move).is_some()).then_some(())?;
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
    let ability = record
        .abilities
        .iter()
        .filter(|entry| !entry.is_hidden)
        .find_map(|entry| data.ability_by_id(entry.ability_id))
        .and_then(battle_ability);
    let build = |ability| match ability {
        Some(ability) => Pokemon::new_with_ability(
            PokemonId::new(format!("{prefix}-form-{}", member.pokemon_form_id.0))?,
            &record.display_name.localized,
            member.level,
            primary_type,
            secondary_type,
            calculated.max_hp(),
            calculated.max_hp(),
            calculated.battle(),
            moves,
            ability,
        ),
        None => Pokemon::new(
            PokemonId::new(format!("{prefix}-form-{}", member.pokemon_form_id.0))?,
            &record.display_name.localized,
            member.level,
            primary_type,
            secondary_type,
            calculated.max_hp(),
            calculated.max_hp(),
            calculated.battle(),
            moves,
        ),
    };
    build(ability).map_err(Into::into)
}

fn battle_ability(record: &game_data::AbilityRecord) -> Option<Ability> {
    match record.identifier.as_str() {
        "air-lock" => Some(Ability::AirLock),
        "arena-trap" => Some(Ability::ArenaTrap),
        "battle-armor" => Some(Ability::BattleArmor),
        "blaze" => Some(Ability::Blaze),
        "chlorophyll" => Some(Ability::Chlorophyll),
        "clear-body" => Some(Ability::ClearBody),
        "cloud-nine" => Some(Ability::CloudNine),
        "compound-eyes" => Some(Ability::CompoundEyes),
        "drizzle" => Some(Ability::Drizzle),
        "drought" => Some(Ability::Drought),
        "early-bird" => Some(Ability::EarlyBird),
        "flash-fire" => Some(Ability::FlashFire),
        "guts" => Some(Ability::Guts),
        "huge-power" => Some(Ability::HugePower),
        "hyper-cutter" => Some(Ability::HyperCutter),
        "hustle" => Some(Ability::Hustle),
        "immunity" => Some(Ability::Immunity),
        "intimidate" => Some(Ability::Intimidate),
        "inner-focus" => Some(Ability::InnerFocus),
        "keen-eye" => Some(Ability::KeenEye),
        "insomnia" => Some(Ability::Insomnia),
        "levitate" => Some(Ability::Levitate),
        "limber" => Some(Ability::Limber),
        "liquid-ooze" => Some(Ability::LiquidOoze),
        "magma-armor" => Some(Ability::MagmaArmor),
        "marvel-scale" => Some(Ability::MarvelScale),
        "natural-cure" => Some(Ability::NaturalCure),
        "overgrow" => Some(Ability::Overgrow),
        "pressure" => Some(Ability::Pressure),
        "pure-power" => Some(Ability::PurePower),
        "rain-dish" => Some(Ability::RainDish),
        "rock-head" => Some(Ability::RockHead),
        "sand-stream" => Some(Ability::SandStream),
        "sand-veil" => Some(Ability::SandVeil),
        "serene-grace" => Some(Ability::SereneGrace),
        "shell-armor" => Some(Ability::ShellArmor),
        "shed-skin" => Some(Ability::ShedSkin),
        "shield-dust" => Some(Ability::ShieldDust),
        "shadow-tag" => Some(Ability::ShadowTag),
        "synchronize" => Some(Ability::Synchronize),
        "speed-boost" => Some(Ability::SpeedBoost),
        "swift-swim" => Some(Ability::SwiftSwim),
        "swarm" => Some(Ability::Swarm),
        "thick-fat" => Some(Ability::ThickFat),
        "torrent" => Some(Ability::Torrent),
        "vital-spirit" => Some(Ability::VitalSpirit),
        "volt-absorb" => Some(Ability::VoltAbsorb),
        "water-absorb" => Some(Ability::WaterAbsorb),
        "water-veil" => Some(Ability::WaterVeil),
        "white-smoke" => Some(Ability::WhiteSmoke),
        _ => None,
    }
}

fn battle_move(data: &CurrentDataSet, id: DataMoveId) -> Result<Move, RosterError> {
    let record = data.move_by_id(id).ok_or(RosterError::MissingMove(id))?;
    let pp = record.pp.ok_or(RosterError::MissingMovePp(id))?;
    let category = battle_move_category(record.damage_class);
    let effect = move_effect(record).unwrap_or(MoveEffect::None);
    let power = match category {
        MoveCategory::Status => 0,
        MoveCategory::Physical | MoveCategory::Special => record
            .power
            .or_else(|| effect.permits_zero_power().then_some(0))
            .ok_or(RosterError::MissingMovePower(id))?,
    };
    let accuracy = record
        .accuracy
        .map(Accuracy::percent)
        .transpose()?
        .unwrap_or(Accuracy::AlwaysHit);
    Move::new_with_category_and_effect(
        MoveId::new(format!("pokeapi-move-{}", id.0))?,
        &record.display_name.localized,
        battle_type(data, record.move_type)?,
        category,
        power,
        accuracy,
        pp,
        pp,
        record.priority,
        effect,
    )
    .map(|battle_move| {
        let battle_move = match weather_accuracy(record) {
            Some(modifier) => battle_move.with_weather_accuracy(modifier),
            None => battle_move,
        };
        match weather_move(record) {
            Some(modifier) => battle_move.with_weather_move(modifier),
            None => battle_move,
        }
    })
    .map_err(Into::into)
}

fn weather_accuracy(record: &game_data::MoveRecord) -> Option<WeatherAccuracyModifier> {
    match record.identifier.as_str() {
        "thunder" => Some(WeatherAccuracyModifier::Thunder),
        _ => None,
    }
}

fn weather_move(record: &game_data::MoveRecord) -> Option<WeatherMoveModifier> {
    match record.identifier.as_str() {
        "weather-ball" => Some(WeatherMoveModifier::WeatherBall),
        _ => None,
    }
}

fn move_effect(record: &game_data::MoveRecord) -> Option<MoveEffect> {
    match record.effect_id? {
        2 => major_status_effect(record, MajorStatusKind::Sleep),
        3 => major_status_effect(record, MajorStatusKind::Poison),
        34 => major_status_effect(record, MajorStatusKind::BadlyPoisoned),
        5 | 168 => major_status_effect(record, MajorStatusKind::Burn),
        6 => major_status_effect(record, MajorStatusKind::Freeze),
        7 | 68 => major_status_effect(record, MajorStatusKind::Paralysis),
        11 => stage_effect(EffectTarget::User, BattleStat::Attack, 1),
        12 => stage_effect(EffectTarget::User, BattleStat::Defense, 1),
        17 => stage_effect(EffectTarget::User, BattleStat::Evasion, 1),
        19 => stage_effect(EffectTarget::Opponent, BattleStat::Attack, -1),
        20 => stage_effect(EffectTarget::Opponent, BattleStat::Defense, -1),
        21 => stage_effect(EffectTarget::Opponent, BattleStat::Speed, -1),
        24 => stage_effect(EffectTarget::Opponent, BattleStat::Accuracy, -1),
        25 => stage_effect(EffectTarget::Opponent, BattleStat::Evasion, -1),
        26 => Some(MoveEffect::haze()),
        32 | 159 => MoveEffect::flinch_target(record.effect_chance.unwrap_or(100)).ok(),
        42 => Some(MoveEffect::fixed_damage_amount(40)),
        69 => stage_effect_with_chance(record, EffectTarget::Opponent, BattleStat::Attack, -1),
        70 => stage_effect_with_chance(record, EffectTarget::Opponent, BattleStat::Defense, -1),
        71 => stage_effect_with_chance(record, EffectTarget::Opponent, BattleStat::Speed, -1),
        72 => stage_effect_with_chance(
            record,
            EffectTarget::Opponent,
            BattleStat::SpecialAttack,
            -1,
        ),
        73 => stage_effect_with_chance(
            record,
            EffectTarget::Opponent,
            BattleStat::SpecialDefense,
            -1,
        ),
        74 => stage_effect_with_chance(record, EffectTarget::Opponent, BattleStat::Accuracy, -1),
        33 => MoveEffect::heal_user(1, 2).ok(),
        38 => Some(MoveEffect::rest()),
        4 => MoveEffect::drain_user(1, 2).ok(),
        52 => stage_effect(EffectTarget::User, BattleStat::Defense, 2),
        53 => stage_effect(EffectTarget::User, BattleStat::Speed, 2),
        54 => stage_effect(EffectTarget::User, BattleStat::SpecialAttack, 2),
        55 => stage_effect(EffectTarget::User, BattleStat::SpecialDefense, 2),
        59 => stage_effect(EffectTarget::Opponent, BattleStat::Attack, -2),
        60 => stage_effect(EffectTarget::Opponent, BattleStat::Defense, -2),
        61 => stage_effect(EffectTarget::Opponent, BattleStat::Speed, -2),
        62 => stage_effect(EffectTarget::Opponent, BattleStat::SpecialAttack, -2),
        63 => stage_effect(EffectTarget::Opponent, BattleStat::SpecialDefense, -2),
        80 => Some(MoveEffect::create_substitute()),
        88 => Some(MoveEffect::fixed_damage_user_level()),
        49 => MoveEffect::recoil_user(1, 4).ok(),
        112 => Some(MoveEffect::protect_user()),
        116 => Some(MoveEffect::start_weather(Weather::Sandstorm)),
        137 => Some(MoveEffect::start_weather(Weather::Rain)),
        138 => Some(MoveEffect::start_weather(Weather::Sun)),
        144 => Some(MoveEffect::copy_target_stages()),
        165 => Some(MoveEffect::start_weather(Weather::Hail)),
        199 => MoveEffect::recoil_user(1, 3).ok(),
        194 => Some(MoveEffect::refresh()),
        131 => Some(MoveEffect::fixed_damage_amount(20)),
        212 => StageChanges::new(0, 0, 1, 1, 0, 0, 0)
            .ok()
            .map(|changes| MoveEffect::change_stages(EffectTarget::User, changes)),
        278 => StageChanges::new(1, 0, 0, 0, 0, 1, 0)
            .ok()
            .map(|changes| MoveEffect::change_stages(EffectTarget::User, changes)),
        // Growth raises Special Attack by one stage in generation three.
        317 => stage_effect(EffectTarget::User, BattleStat::SpecialAttack, 1),
        328 => StageChanges::new(1, 0, 1, 0, 0, 0, 0)
            .ok()
            .map(|changes| MoveEffect::change_stages(EffectTarget::User, changes)),
        _ => None,
    }
}

fn major_status_effect(
    record: &game_data::MoveRecord,
    status: MajorStatusKind,
) -> Option<MoveEffect> {
    MoveEffect::inflict_major_status(status, record.effect_chance.unwrap_or(100)).ok()
}

fn stage_effect(target: EffectTarget, stat: BattleStat, amount: i8) -> Option<MoveEffect> {
    StageChanges::single(stat, amount)
        .ok()
        .map(|changes| MoveEffect::change_stages(target, changes))
}

fn stage_effect_with_chance(
    record: &game_data::MoveRecord,
    target: EffectTarget,
    stat: BattleStat,
    amount: i8,
) -> Option<MoveEffect> {
    MoveEffect::change_stages_with_chance(
        target,
        StageChanges::single(stat, amount).ok()?,
        record.effect_chance.unwrap_or(100),
    )
    .ok()
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
#[path = "../tests/unit/roster.rs"]
mod tests;
